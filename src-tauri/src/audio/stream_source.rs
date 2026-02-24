use std::io::{self, Read, Seek, SeekFrom};
use std::sync::{Arc, Condvar, Mutex};

/// Pre-allocate capacity for typical track sizes (~40MB for FLAC).
const INITIAL_CAPACITY: usize = 1024 * 1024; // 1MB

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
    /// Total expected length from HTTP Content-Length header.
    /// Set before data arrives so symphonia can see the stream as seekable.
    total_length: Option<u64>,
}

/// Handle to abort a stream source, unblocking any pending reads.
/// Stored by AudioPlayer so stop_internal() can break the decode thread
/// out of a blocking read when it has seeked past the downloaded data.
pub struct StreamAbortHandle {
    shared: Arc<(Mutex<StreamBuffer>, Condvar)>,
}

impl StreamAbortHandle {
    pub fn abort(&self) {
        let (lock, cvar) = &*self.shared;
        let mut state = lock.lock().unwrap();
        state.error = Some("aborted".to_string());
        state.finished = true;
        cvar.notify_all();
    }
}

/// Adapter that makes an HTTP byte stream look like a seekable `Read` + `symphonia::core::io::MediaSource`.
/// All downloaded bytes are retained in memory so symphonia can seek backwards.
pub struct HttpStreamSource {
    shared: Arc<(Mutex<StreamBuffer>, Condvar)>,
}

impl HttpStreamSource {
    pub fn new() -> (Self, StreamWriter, StreamAbortHandle) {
        let shared = Arc::new((
            Mutex::new(StreamBuffer {
                data: Vec::with_capacity(INITIAL_CAPACITY),
                position: 0,
                finished: false,
                error: None,
                total_length: None,
            }),
            Condvar::new(),
        ));

        let source = Self {
            shared: Arc::clone(&shared),
        };
        let writer = StreamWriter {
            shared: Arc::clone(&shared),
        };
        let abort_handle = StreamAbortHandle { shared };

        (source, writer, abort_handle)
    }
}

impl Read for HttpStreamSource {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let (lock, cvar) = &*self.shared;
        let mut state = lock.lock().unwrap();

        // Wait until we have data beyond our position, the stream is finished, or there's an error.
        // Use a timeout so that seeking past the download cursor doesn't block forever.
        let timeout = std::time::Duration::from_millis(500);
        let mut waited = std::time::Duration::ZERO;
        const MAX_WAIT: std::time::Duration = std::time::Duration::from_secs(3);

        while state.position >= state.data.len() && !state.finished && state.error.is_none() {
            let (new_state, wait_result) = cvar.wait_timeout(state, timeout).unwrap();
            state = new_state;
            if wait_result.timed_out() {
                waited += timeout;
                if waited >= MAX_WAIT {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "Timed out waiting for stream data",
                    ));
                }
            }
        }

        if let Some(ref err) = state.error {
            return Err(io::Error::new(io::ErrorKind::Other, err.clone()));
        }

        let available = state.data.len().saturating_sub(state.position);
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

        let end = if state.finished {
            state.data.len() as i64
        } else {
            state.total_length.unwrap_or(state.data.len() as u64) as i64
        };

        let new_pos = match pos {
            SeekFrom::Start(offset) => offset as i64,
            SeekFrom::Current(offset) => state.position as i64 + offset,
            SeekFrom::End(offset) => end + offset,
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
            // Return the Content-Length so symphonia treats the stream as seekable
            // even before the download completes.
            state.total_length
        }
    }
}

/// Writer end that receives bytes from the HTTP download task.
pub struct StreamWriter {
    shared: Arc<(Mutex<StreamBuffer>, Condvar)>,
}

impl StreamWriter {
    /// Set the total expected length (from HTTP Content-Length header).
    /// Must be called before data arrives so symphonia can see the stream as seekable.
    pub fn set_total_length(&self, length: u64) {
        let (lock, _) = &*self.shared;
        let mut state = lock.lock().unwrap();
        state.total_length = Some(length);
    }

    pub fn write_bytes(&self, data: &[u8]) -> Result<(), String> {
        let (lock, cvar) = &*self.shared;
        let mut state = lock.lock().unwrap();

        if state.finished {
            return Ok(());
        }

        // No back-pressure: download as fast as possible so seeking
        // to any position works immediately. All bytes are retained
        // in memory for backward seek support anyway.
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
