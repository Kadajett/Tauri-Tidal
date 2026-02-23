use serde::Serialize;

pub const PLAYBACK_PROGRESS: &str = "playback:progress";
pub const PLAYBACK_TRACK_CHANGED: &str = "playback:track-changed";
pub const PLAYBACK_STATE_CHANGED: &str = "playback:state-changed";
pub const PLAYBACK_TRACK_ENDED: &str = "playback:track-ended";
pub const PLAYBACK_QUEUE_CHANGED: &str = "playback:queue-changed";
pub const AUTH_STATE_CHANGED: &str = "auth:state-changed";

#[derive(Debug, Clone, Serialize)]
pub struct ProgressPayload {
    pub position: f64,
    pub duration: f64,
    pub position_fraction: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrackChangedPayload {
    pub track_id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: f64,
    pub artwork_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StateChangedPayload {
    pub state: PlaybackState,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PlaybackState {
    Playing,
    Paused,
    Stopped,
    Buffering,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuthStatePayload {
    pub authenticated: bool,
    pub user_id: Option<String>,
}
