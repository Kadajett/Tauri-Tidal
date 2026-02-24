use std::io::{self, Read, Seek, SeekFrom};
use std::sync::{Arc, Condvar, Mutex};

const MAX_BUFFER_SIZE: usize = 8 * 1024 * 1024; // 8MB back-pressure limit

/// Shared state between the HTTP download task and the symphonia reader.
struct StreamBuffer {
    /// All downloaded bytes (append-only from writer side).
    data: Vec<u8>,
    /// Read cursor position.
    position: usize,
    /// Whether the download has completed.
    finished: bool,
    /// Download error, if any.
    error: Option<String>,
}

/// Adapter that makes an HTTP byte stream look like a seekable `Read` + `symphonia::core::io::MediaSource`.
/// All downloaded bytes are retained in memory so symphonia can seek backwards.
pub struct HttpStreamSource {
    shared: Arc<(Mutex<StreamBuffer>, Condvar)>,
}

impl HttpStreamSource {
    pub fn new() -> (Self, StreamWriter) {
        let shared = Arc::new((
            Mutex::new(StreamBuffer {
                data: Vec::with_capacity(1024 * 1024), // Pre-allocate 1MB
                position: 0,
                finished: false,
                error: None,
            }),
            Condvar::new(),
        ));

        let source = Self {
            shared: Arc::clone(&shared),
        };
        let writer = StreamWriter { shared };

        (source, writer)
    }
}

impl Read for HttpStreamSource {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let (lock, cvar) = &*self.shared;
        let mut state = lock.lock().unwrap();

        // Wait until we have data beyond our position, the stream is finished, or there's an error
        while state.position >= state.data.len() && !state.finished && state.error.is_none() {
            state = cvar.wait(state).unwrap();
        }

        if let Some(ref err) = state.error {
            return Err(io::Error::new(io::ErrorKind::Other, err.clone()));
        }

        let available = state.data.len() - state.position;
        if available == 0 && state.finished {
            return Ok(0); // EOF
        }

        let to_read = buf.len().min(available);
        buf[..to_read].copy_from_slice(&state.data[state.position..state.position + to_read]);
        state.position += to_read;

        // Notify writer (for back-pressure, though we no longer drain bytes)
        cvar.notify_all();

        Ok(to_read)
    }
}

impl Seek for HttpStreamSource {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let (lock, _cvar) = &*self.shared;
        let mut state = lock.lock().unwrap();

        let new_pos = match pos {
            SeekFrom::Start(offset) => offset as i64,
            SeekFrom::Current(offset) => state.position as i64 + offset,
            SeekFrom::End(offset) => state.data.len() as i64 + offset,
        };

        if new_pos < 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Seek to negative position",
            ));
        }

        state.position = new_pos as usize;
        Ok(state.position as u64)
    }
}

impl symphonia::core::io::MediaSource for HttpStreamSource {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        let (lock, _) = &*self.shared;
        let state = lock.lock().unwrap();
        if state.finished {
            Some(state.data.len() as u64)
        } else {
            None
        }
    }
}

/// Writer end that receives bytes from the HTTP download task.
pub struct StreamWriter {
    shared: Arc<(Mutex<StreamBuffer>, Condvar)>,
}

impl StreamWriter {
    pub fn write_bytes(&self, data: &[u8]) -> Result<(), String> {
        let (lock, cvar) = &*self.shared;
        let mut state = lock.lock().unwrap();

        // Back-pressure: wait if we're too far ahead of the reader
        while (state.data.len() - state.position) >= MAX_BUFFER_SIZE && !state.finished {
            state = cvar.wait(state).unwrap();
        }

        if state.finished {
            return Ok(());
        }

        state.data.extend_from_slice(data);
        cvar.notify_all();
        Ok(())
    }

    pub fn finish(&self) {
        let (lock, cvar) = &*self.shared;
        let mut state = lock.lock().unwrap();
        state.finished = true;
        cvar.notify_all();
    }

    pub fn set_error(&self, error: String) {
        let (lock, cvar) = &*self.shared;
        let mut state = lock.lock().unwrap();
        state.error = Some(error);
        state.finished = true;
        cvar.notify_all();
    }
}
