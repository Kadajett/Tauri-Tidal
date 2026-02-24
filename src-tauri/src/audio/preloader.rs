use crate::audio::stream_source::{HttpStreamSource, StreamAbortHandle};

/// Holds a preloaded track's stream source, ready for immediate playback.
pub struct PreloadedTrack {
    pub source: HttpStreamSource,
    pub abort_handle: StreamAbortHandle,
    pub codec_hint: Option<String>,
    pub track_id: String,
    pub duration: f64,
    /// Keep the download handle alive
    _download_handle: tokio::task::JoinHandle<()>,
}

impl PreloadedTrack {
    pub fn new(
        track_id: String,
        codec_hint: Option<String>,
        duration: f64,
        url: String,
        client: reqwest::Client,
    ) -> Self {
        let (source, writer, abort_handle) = HttpStreamSource::new();

        let handle = crate::audio::player::AudioPlayer::start_download(writer, url, client);

        Self {
            source,
            abort_handle,
            codec_hint,
            track_id,
            duration,
            _download_handle: handle,
        }
    }
}
