use crate::audio::decoder::AudioDecoder;
use crate::audio::stream_source::{HttpStreamSource, StreamWriter};
use crate::error::{AppError, AppResult};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};

/// Shared ring buffer between the decode thread and the cpal callback.
struct SampleRingBuffer {
    buffer: VecDeque<f32>,
    finished: bool,
}

/// Wrapper to make cpal::Stream Send+Sync.
/// This is safe because we only modify the stream from a single logical owner (AudioPlayer),
/// and cpal::Stream is only non-Send due to macOS CoreAudio API requirements that
/// are satisfied by our usage pattern (created and dropped on the same thread).
struct SendStream(Option<cpal::Stream>);
unsafe impl Send for SendStream {}
unsafe impl Sync for SendStream {}

/// Sentinel value meaning "no seek requested".
const NO_SEEK: u64 = u64::MAX;

pub struct AudioPlayer {
    /// cpal stream handle (kept alive)
    stream: SendStream,
    /// Sample buffer shared with the output callback
    ring: Arc<(Mutex<SampleRingBuffer>, Condvar)>,
    /// Volume [0.0, 1.0]
    volume: Arc<Mutex<f32>>,
    /// Samples played counter (for position tracking)
    samples_played: Arc<AtomicU64>,
    /// Sample rate of the current track
    sample_rate: Arc<Mutex<u32>>,
    /// Number of channels
    channels: Arc<Mutex<usize>>,
    /// Whether playback is active
    playing: Arc<AtomicBool>,
    /// Handle to the decode thread
    decode_handle: Option<std::thread::JoinHandle<()>>,
    /// Signal to stop the decode thread
    stop_signal: Arc<AtomicBool>,
    /// Total duration in seconds (from track metadata)
    total_duration: Arc<Mutex<f64>>,
    /// Seek target in milliseconds (NO_SEEK = no pending seek).
    /// The decode thread reads and clears this.
    seek_target_ms: Arc<AtomicU64>,
}

impl AudioPlayer {
    pub fn new() -> AppResult<Self> {
        let ring = Arc::new((
            Mutex::new(SampleRingBuffer {
                buffer: VecDeque::with_capacity(88200),
                finished: false,
            }),
            Condvar::new(),
        ));

        let volume = Arc::new(Mutex::new(1.0f32));
        let samples_played = Arc::new(AtomicU64::new(0));
        let playing = Arc::new(AtomicBool::new(false));

        Ok(Self {
            stream: SendStream(None),
            ring,
            volume,
            samples_played,
            sample_rate: Arc::new(Mutex::new(44100)),
            channels: Arc::new(Mutex::new(2)),
            playing,
            decode_handle: None,
            stop_signal: Arc::new(AtomicBool::new(false)),
            total_duration: Arc::new(Mutex::new(0.0)),
            seek_target_ms: Arc::new(AtomicU64::new(NO_SEEK)),
        })
    }

    pub fn play_stream(
        &mut self,
        source: HttpStreamSource,
        codec_hint: Option<&str>,
        duration: f64,
    ) -> AppResult<()> {
        self.stop_internal();

        let mut decoder = AudioDecoder::new(source, codec_hint)?;
        let sr = decoder.sample_rate();
        let ch = decoder.channels();

        *self.sample_rate.lock().unwrap() = sr;
        *self.channels.lock().unwrap() = ch;
        *self.total_duration.lock().unwrap() = duration;
        self.samples_played.store(0, Ordering::SeqCst);
        // Clear any stale seek from a previous track
        self.seek_target_ms.store(NO_SEEK, Ordering::SeqCst);

        {
            let (lock, cvar) = &*self.ring;
            let mut ring = lock.lock().unwrap();
            ring.buffer.clear();
            ring.finished = false;
            cvar.notify_all();
        }

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| AppError::Audio("No output device available".into()))?;

        let stream_config = cpal::StreamConfig {
            channels: ch as u16,
            sample_rate: cpal::SampleRate(sr),
            buffer_size: cpal::BufferSize::Default,
        };

        let ring_clone = Arc::clone(&self.ring);
        let volume_clone = Arc::clone(&self.volume);
        let samples_played_clone = Arc::clone(&self.samples_played);
        let playing_clone = Arc::clone(&self.playing);

        let cpal_stream = device
            .build_output_stream(
                &stream_config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    if !playing_clone.load(Ordering::Relaxed) {
                        data.fill(0.0);
                        return;
                    }

                    let vol = *volume_clone.lock().unwrap();
                    let (lock, cvar) = &*ring_clone;
                    let mut ring = lock.lock().unwrap();

                    let available = ring.buffer.len().min(data.len());
                    for (i, sample) in data.iter_mut().enumerate() {
                        if i < available {
                            *sample = ring.buffer.pop_front().unwrap_or(0.0) * vol;
                        } else {
                            *sample = 0.0;
                        }
                    }

                    samples_played_clone.fetch_add(available as u64, Ordering::Relaxed);
                    cvar.notify_all();
                },
                |err| {
                    log::error!("cpal output error: {}", err);
                },
                None,
            )
            .map_err(|e| AppError::Audio(format!("Failed to build output stream: {}", e)))?;

        cpal_stream
            .play()
            .map_err(|e| AppError::Audio(format!("Failed to start playback: {}", e)))?;

        self.stream = SendStream(Some(cpal_stream));
        self.playing.store(true, Ordering::SeqCst);

        let ring_clone = Arc::clone(&self.ring);
        let stop_signal = Arc::new(AtomicBool::new(false));
        self.stop_signal = Arc::clone(&stop_signal);
        let seek_target = Arc::clone(&self.seek_target_ms);
        let samples_played_decode = Arc::clone(&self.samples_played);
        let sr_decode = sr;
        let ch_decode = ch;

        let handle = std::thread::spawn(move || {
            const MAX_RING_SAMPLES: usize = 176400;

            loop {
                if stop_signal.load(Ordering::Relaxed) {
                    break;
                }

                // Check for pending seek request
                let pending_seek = seek_target.swap(NO_SEEK, Ordering::SeqCst);
                if pending_seek != NO_SEEK {
                    let seek_seconds = pending_seek as f64 / 1000.0;
                    log::info!("Decode thread: seeking to {:.2}s", seek_seconds);

                    // Clear the ring buffer
                    {
                        let (lock, cvar) = &*ring_clone;
                        let mut ring = lock.lock().unwrap();
                        ring.buffer.clear();
                        cvar.notify_all();
                    }

                    // Seek the decoder
                    if let Err(e) = decoder.seek(seek_seconds) {
                        log::error!("Decode thread: seek failed: {}", e);
                        // Update position counter anyway so UI reflects the attempt
                    }

                    // Update samples_played to reflect new position
                    let new_samples = (seek_seconds * sr_decode as f64 * ch_decode as f64) as u64;
                    samples_played_decode.store(new_samples, Ordering::SeqCst);
                    continue;
                }

                {
                    let (lock, cvar) = &*ring_clone;
                    let mut ring = lock.lock().unwrap();
                    while ring.buffer.len() >= MAX_RING_SAMPLES
                        && !stop_signal.load(Ordering::Relaxed)
                        && seek_target.load(Ordering::Relaxed) == NO_SEEK
                    {
                        ring = cvar.wait(ring).unwrap();
                    }
                }

                if stop_signal.load(Ordering::Relaxed) {
                    break;
                }

                // Re-check seek after waking from wait
                if seek_target.load(Ordering::Relaxed) != NO_SEEK {
                    continue;
                }

                match decoder.decode_next() {
                    Ok(Some(decoded)) => {
                        let (lock, cvar) = &*ring_clone;
                        let mut ring = lock.lock().unwrap();
                        ring.buffer.extend(decoded.samples.iter());
                        cvar.notify_all();
                    }
                    Ok(None) => {
                        let (lock, cvar) = &*ring_clone;
                        let mut ring = lock.lock().unwrap();
                        ring.finished = true;
                        cvar.notify_all();
                        break;
                    }
                    Err(e) => {
                        log::error!("Decode error: {}", e);
                        break;
                    }
                }
            }
        });

        self.decode_handle = Some(handle);
        Ok(())
    }

    fn stop_internal(&mut self) {
        self.stop_signal.store(true, Ordering::SeqCst);
        self.playing.store(false, Ordering::SeqCst);

        {
            let (_lock, cvar) = &*self.ring;
            cvar.notify_all();
        }

        if let Some(handle) = self.decode_handle.take() {
            let _ = handle.join();
        }

        self.stream = SendStream(None);
        self.stop_signal = Arc::new(AtomicBool::new(false));
    }

    pub fn stop(&mut self) {
        self.stop_internal();
        self.samples_played.store(0, Ordering::SeqCst);
    }

    pub fn pause(&mut self) {
        self.playing.store(false, Ordering::SeqCst);
    }

    pub fn resume(&mut self) {
        self.playing.store(true, Ordering::SeqCst);
    }

    pub fn is_playing(&self) -> bool {
        self.playing.load(Ordering::Relaxed)
    }

    pub fn set_volume(&self, vol: f32) {
        *self.volume.lock().unwrap() = vol.clamp(0.0, 1.0);
    }

    pub fn volume(&self) -> f32 {
        *self.volume.lock().unwrap()
    }

    pub fn position_seconds(&self) -> f64 {
        let samples = self.samples_played.load(Ordering::Relaxed) as f64;
        let sr = *self.sample_rate.lock().unwrap() as f64;
        let ch = *self.channels.lock().unwrap() as f64;
        if sr > 0.0 && ch > 0.0 {
            samples / (sr * ch)
        } else {
            0.0
        }
    }

    pub fn duration_seconds(&self) -> f64 {
        *self.total_duration.lock().unwrap()
    }

    pub fn seek(&self, position_seconds: f64) {
        // Send seek request to the decode thread (in milliseconds for precision)
        let ms = (position_seconds * 1000.0) as u64;
        self.seek_target_ms.store(ms, Ordering::SeqCst);

        // Wake the decode thread if it's waiting on the ring buffer
        let (_lock, cvar) = &*self.ring;
        cvar.notify_all();

        // Immediately update the position counter for responsive UI
        let sr = *self.sample_rate.lock().unwrap() as f64;
        let ch = *self.channels.lock().unwrap() as f64;
        let sample_position = (position_seconds * sr * ch) as u64;
        self.samples_played.store(sample_position, Ordering::SeqCst);
    }

    pub fn is_finished(&self) -> bool {
        let (lock, _) = &*self.ring;
        let ring = lock.lock().unwrap();
        ring.finished && ring.buffer.is_empty()
    }

    pub fn start_download(
        writer: StreamWriter,
        url: String,
        client: reqwest::Client,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            log::info!("Starting audio download: {}...", &url[..url.len().min(100)]);
            match client.get(&url).send().await {
                Ok(response) => {
                    let status = response.status();
                    let content_type = response
                        .headers()
                        .get("content-type")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("unknown")
                        .to_string();
                    let content_len = response
                        .headers()
                        .get("content-length")
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("unknown")
                        .to_string();

                    log::info!(
                        "Audio download response: status={}, content-type={}, content-length={}",
                        status,
                        content_type,
                        content_len
                    );

                    if !status.is_success() {
                        let body = response.text().await.unwrap_or_default();
                        log::error!(
                            "Audio download failed ({}): {}",
                            status,
                            &body[..body.len().min(500)]
                        );
                        writer.set_error(format!("Download failed: HTTP {}", status));
                        return;
                    }

                    use futures_util::StreamExt;
                    let mut stream = response.bytes_stream();
                    let mut total_bytes = 0u64;
                    while let Some(chunk) = stream.next().await {
                        match chunk {
                            Ok(bytes) => {
                                total_bytes += bytes.len() as u64;
                                if writer.write_bytes(&bytes).is_err() {
                                    log::warn!(
                                        "Audio download: writer closed after {} bytes",
                                        total_bytes
                                    );
                                    break;
                                }
                            }
                            Err(e) => {
                                log::error!(
                                    "Audio download stream error after {} bytes: {}",
                                    total_bytes,
                                    e
                                );
                                writer.set_error(format!("Download error: {}", e));
                                return;
                            }
                        }
                    }
                    log::info!("Audio download complete: {} bytes", total_bytes);
                    writer.finish();
                }
                Err(e) => {
                    log::error!("Failed to start audio download: {}", e);
                    writer.set_error(format!("Failed to start download: {}", e));
                }
            }
        })
    }
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        self.stop_internal();
    }
}
