use std::collections::VecDeque;
use std::io::{self, Read};
use std::sync::{Arc, Condvar, Mutex};

const MAX_BUFFER_SIZE: usize = 2 * 1024 * 1024; // 2MB back-pressure limit

/// Shared state between the HTTP download task and the symphonia reader.
struct StreamBuffer {
    buffer: VecDeque<u8>,
    finished: bool,
    error: Option<String>,
}

/// Adapter that makes an HTTP byte stream look like a `Read` + `symphonia::core::io::MediaSource`.
pub struct HttpStreamSource {
    shared: Arc<(Mutex<StreamBuffer>, Condvar)>,
}

impl HttpStreamSource {
    pub fn new() -> (Self, StreamWriter) {
        let shared = Arc::new((
            Mutex::new(StreamBuffer {
                buffer: VecDeque::new(),
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

        // Wait until we have data, the stream is finished, or there's an error
        while state.buffer.is_empty() && !state.finished && state.error.is_none() {
            state = cvar.wait(state).unwrap();
        }

        if let Some(ref err) = state.error {
            return Err(io::Error::new(io::ErrorKind::Other, err.clone()));
        }

        if state.buffer.is_empty() && state.finished {
            return Ok(0); // EOF
        }

        let to_read = buf.len().min(state.buffer.len());
        let (front, back) = state.buffer.as_slices();

        if to_read <= front.len() {
            buf[..to_read].copy_from_slice(&front[..to_read]);
        } else {
            let front_len = front.len();
            buf[..front_len].copy_from_slice(front);
            let remaining = to_read - front_len;
            buf[front_len..front_len + remaining].copy_from_slice(&back[..remaining]);
        }

        state.buffer.drain(..to_read);

        // Notify writer that buffer space is available
        cvar.notify_all();

        Ok(to_read)
    }
}

impl symphonia::core::io::MediaSource for HttpStreamSource {
    fn is_seekable(&self) -> bool {
        false
    }

    fn byte_len(&self) -> Option<u64> {
        None
    }
}

impl io::Seek for HttpStreamSource {
    fn seek(&mut self, _pos: io::SeekFrom) -> io::Result<u64> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "HttpStreamSource is not seekable",
        ))
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

        // Back-pressure: wait if buffer is full
        while state.buffer.len() >= MAX_BUFFER_SIZE && !state.finished {
            state = cvar.wait(state).unwrap();
        }

        if state.finished {
            return Ok(());
        }

        state.buffer.extend(data);
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
