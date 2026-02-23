use crate::audio::stream_source::{HttpStreamSource, StreamWriter};

/// Holds a preloaded track's stream source, ready for immediate playback.
pub struct PreloadedTrack {
    pub source: HttpStreamSource,
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
        let (source, writer) = HttpStreamSource::new();

        let handle = crate::audio::player::AudioPlayer::start_download(writer, url, client);

        Self {
            source,
            codec_hint,
            track_id,
            duration,
            _download_handle: handle,
        }
    }
}
