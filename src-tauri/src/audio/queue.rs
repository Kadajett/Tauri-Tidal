use crate::api::models::Track;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RepeatMode {
    Off,
    All,
    One,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueState {
    pub tracks: Vec<Track>,
    pub current_index: Option<usize>,
    pub repeat_mode: RepeatMode,
    pub shuffled: bool,
}

/// Full queue state including original order, for disk persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedQueueState {
    pub tracks: Vec<Track>,
    pub original_order: Vec<Track>,
    pub current_index: Option<usize>,
    pub repeat_mode: RepeatMode,
    pub shuffled: bool,
}

pub struct PlaybackQueue {
    tracks: Vec<Track>,
    original_order: Vec<Track>,
    current_index: Option<usize>,
    repeat_mode: RepeatMode,
    shuffled: bool,
}

impl PlaybackQueue {
    pub fn new() -> Self {
        Self {
            tracks: Vec::new(),
            original_order: Vec::new(),
            current_index: None,
            repeat_mode: RepeatMode::Off,
            shuffled: false,
        }
    }

    pub fn set_tracks(&mut self, tracks: Vec<Track>, start_index: usize) {
        self.original_order = tracks.clone();
        self.tracks = tracks;
        self.shuffled = false;
        self.current_index = if self.tracks.is_empty() {
            None
        } else {
            Some(start_index.min(self.tracks.len() - 1))
        };
    }

    pub fn add_track(&mut self, track: Track) {
        self.tracks.push(track.clone());
        self.original_order.push(track);
        if self.current_index.is_none() {
            self.current_index = Some(0);
        }
    }

    pub fn remove_track(&mut self, index: usize) {
        if index >= self.tracks.len() {
            return;
        }

        let removed_id = self.tracks[index].id.clone();
        self.tracks.remove(index);
        self.original_order.retain(|t| t.id != removed_id);

        if let Some(current) = self.current_index {
            if index < current {
                self.current_index = Some(current - 1);
            } else if index == current && current >= self.tracks.len() {
                self.current_index = if self.tracks.is_empty() {
                    None
                } else {
                    Some(self.tracks.len() - 1)
                };
            }
        }
    }

    pub fn move_track(&mut self, from: usize, to: usize) {
        if from >= self.tracks.len() || to >= self.tracks.len() {
            return;
        }

        let track = self.tracks.remove(from);
        self.tracks.insert(to, track);

        if let Some(current) = self.current_index {
            if from == current {
                self.current_index = Some(to);
            } else if from < current && to >= current {
                self.current_index = Some(current - 1);
            } else if from > current && to <= current {
                self.current_index = Some(current + 1);
            }
        }
    }

    pub fn current_track(&self) -> Option<&Track> {
        self.current_index.and_then(|i| self.tracks.get(i))
    }

    pub fn next_track(&mut self) -> Option<&Track> {
        let len = self.tracks.len();
        if len == 0 {
            return None;
        }

        match self.repeat_mode {
            RepeatMode::One => self.current_track(),
            RepeatMode::All => {
                let next = self.current_index.map(|i| (i + 1) % len).unwrap_or(0);
                self.current_index = Some(next);
                self.tracks.get(next)
            }
            RepeatMode::Off => {
                let current = self.current_index.unwrap_or(0);
                if current + 1 < len {
                    self.current_index = Some(current + 1);
                    self.tracks.get(current + 1)
                } else {
                    None
                }
            }
        }
    }

    pub fn previous_track(&mut self) -> Option<&Track> {
        let len = self.tracks.len();
        if len == 0 {
            return None;
        }

        let current = self.current_index.unwrap_or(0);
        if current > 0 {
            self.current_index = Some(current - 1);
        } else if self.repeat_mode == RepeatMode::All {
            self.current_index = Some(len - 1);
        }
        self.current_track()
    }

    pub fn peek_next(&self) -> Option<&Track> {
        let len = self.tracks.len();
        if len == 0 {
            return None;
        }

        match self.repeat_mode {
            RepeatMode::One => self.current_track(),
            RepeatMode::All => {
                let next = self.current_index.map(|i| (i + 1) % len).unwrap_or(0);
                self.tracks.get(next)
            }
            RepeatMode::Off => {
                let current = self.current_index.unwrap_or(0);
                self.tracks.get(current + 1)
            }
        }
    }

    pub fn shuffle(&mut self) {
        if self.tracks.len() <= 1 {
            return;
        }

        let current_track = self.current_track().cloned();
        let mut rng = rand::thread_rng();

        if !self.shuffled {
            self.original_order = self.tracks.clone();
        }

        self.tracks.shuffle(&mut rng);
        self.shuffled = true;

        // Put current track at position 0
        if let Some(current) = current_track {
            if let Some(pos) = self.tracks.iter().position(|t| t.id == current.id) {
                self.tracks.swap(0, pos);
            }
            self.current_index = Some(0);
        }
    }

    pub fn unshuffle(&mut self) {
        if !self.shuffled {
            return;
        }

        let current_track = self.current_track().cloned();
        self.tracks = self.original_order.clone();
        self.shuffled = false;

        if let Some(current) = current_track {
            self.current_index = self.tracks.iter().position(|t| t.id == current.id);
        }
    }

    pub fn toggle_repeat(&mut self) -> RepeatMode {
        self.repeat_mode = match self.repeat_mode {
            RepeatMode::Off => RepeatMode::All,
            RepeatMode::All => RepeatMode::One,
            RepeatMode::One => RepeatMode::Off,
        };
        self.repeat_mode
    }

    pub fn state(&self) -> QueueState {
        QueueState {
            tracks: self.tracks.clone(),
            current_index: self.current_index,
            repeat_mode: self.repeat_mode,
            shuffled: self.shuffled,
        }
    }

    pub fn clear(&mut self) {
        self.tracks.clear();
        self.original_order.clear();
        self.current_index = None;
        self.shuffled = false;
    }

    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    pub fn persisted_state(&self) -> PersistedQueueState {
        PersistedQueueState {
            tracks: self.tracks.clone(),
            original_order: self.original_order.clone(),
            current_index: self.current_index,
            repeat_mode: self.repeat_mode,
            shuffled: self.shuffled,
        }
    }

    pub fn restore_from_persisted(&mut self, state: PersistedQueueState) {
        self.tracks = state.tracks;
        self.original_order = state.original_order;
        self.current_index = state.current_index;
        self.repeat_mode = state.repeat_mode;
        self.shuffled = state.shuffled;
    }
}
