# Architecture

This document covers the internal design of TauriTidal: how audio flows from Tidal's servers to your speakers, how state is managed across Rust and React, and how the two layers communicate.

## Overview

TauriTidal is a Tauri v2 application with two distinct layers:

1. **Rust backend** (`src-tauri/src/`): handles Tidal API communication, audio decoding and playback, queue logic, config persistence, and macOS system integration.
2. **React frontend** (`src/`): handles UI rendering, user interaction, and local UI state. Communicates with the backend exclusively through Tauri IPC commands and event listeners.

The backend owns all network requests, all audio processing, and all persistent state. The frontend is a thin presentation layer that dispatches commands and reacts to events.

## Audio Pipeline

When a user clicks play on a track, four stages execute in parallel:

```
HTTP download ──> StreamBuffer ──> Symphonia decoder ──> Ring buffer ──> cpal output
   (async)         (shared)        (decode thread)       (shared)      (audio thread)
```

### Stage 1: HTTP Stream Source

`HttpStreamSource` (`audio/stream_source.rs`) uses a producer-consumer pattern. A background Tokio task downloads audio bytes from Tidal's CDN in chunks and writes them into a `StreamBuffer`. The buffer has an 8MB back-pressure limit to prevent unbounded memory growth.

All downloaded bytes are retained in the buffer so that seeking backward works without re-downloading. The reader side (`Read + Seek` for Symphonia) blocks on a `Condvar` when it catches up to the writer.

### Stage 2: Audio Decoding

`AudioDecoder` (`audio/decoder.rs`) wraps the Symphonia library. It takes the `HttpStreamSource` as input (which implements `MediaSource`), probes the format using a codec hint (flac, aac, mp3, m4a), and extracts track metadata (sample rate, channels, duration).

The core loop calls `decode_next()` which returns `DecodedSamples`: a `Vec<f32>` of interleaved samples ready for output.

### Stage 3: Ring Buffer

A `SampleRingBuffer` (`audio/player.rs`) sits between the decode thread and the audio output callback. It holds up to 176,400 samples (2 seconds at 44.1kHz stereo). The decode thread fills it; the audio output thread drains it. A `Condvar` handles synchronization in both directions.

When the decoder hits EOF, it sets a `finished` flag on the buffer. The output callback detects this and emits a `track:ended` event to the frontend, which triggers auto-advance.

### Stage 4: Audio Output

`AudioPlayer` (`audio/player.rs`) initializes a cpal output stream on the default audio device. The output callback pulls samples from the ring buffer, applies the current volume multiplier, and fills the output buffer (typically 2048 samples per callback).

Position tracking uses an `Arc<AtomicU64>` counter that increments with each sample written. The frontend polls this via `onProgress` events.

### Seeking

When the user seeks, the target position (in milliseconds) is stored in an `Arc<AtomicU64>`. The decode thread detects the new target on its next loop iteration, calls `decoder.seek()`, clears the ring buffer, and resets the sample counter. This approach avoids locking the audio output thread.

### Preloading

`PreloadedTrack` (`audio/preloader.rs`) starts downloading the next track in the background before the current one finishes. When the current track ends, the preloaded source is already partially buffered and can begin decoding immediately. The download `JoinHandle` is stored in the struct to prevent the background task from being cancelled.

## State Management

### Rust Side: AppState

All backend state lives in a single `AppState` struct, registered as Tauri managed state:

```rust
pub struct AppState {
    pub tidal_client: Arc<TidalClient>,
    pub audio_player: Arc<RwLock<AudioPlayer>>,
    pub playback_queue: Arc<RwLock<PlaybackQueue>>,
    pub current_track: Arc<RwLock<Option<Track>>>,
    pub pkce_verifier: Mutex<Option<String>>,
    pub preloaded_track: Mutex<Option<PreloadedTrack>>,
    _media_key_tokens: Mutex<SendRetainedTokens>,  // macOS only
}
```

Everything is wrapped in `Arc` for cheap cloning across async tasks. `RwLock` is used for data with concurrent readers (audio player, queue, current track). Plain `Mutex` is used for write-once values (PKCE verifier, preloaded track).

Lock-free `AtomicU64` counters handle high-frequency updates like playback position and seek targets.

### Frontend Side: Zustand Stores

Six Zustand stores manage UI state:

| Store | Responsibility |
|-------|---------------|
| `playerStore` | Playback state, current track, position, duration, volume, codec info |
| `queueStore` | Track list, current index, repeat mode, shuffle flag |
| `authStore` | Auth status, login flow state (device code, verification URI) |
| `libraryStore` | Playlists, favorites (with cursor-based pagination), favorite ID set |
| `searchStore` | Query string, results, suggestions, loading flag |
| `uiStore` | UI-only state (sidebar collapsed, etc.) |

Stores are updated by hooks that listen to Tauri events. For example, `usePlayback` subscribes to `onProgress`, `onTrackChanged`, and `onStateChanged` events and writes into `playerStore`.

## IPC Surface

The frontend communicates with the backend through two mechanisms:

### Commands (Request/Response)

Typed wrappers in `src/lib/tauri.ts` call `invoke()` for each backend command. Commands are grouped by domain:

- **Auth** (6 commands): `check_auth_status`, `login`, `poll_login`, `handle_auth_callback`, `init_client_credentials`, `logout`
- **Playback** (12 commands): `play_track`, `play_tracks`, `pause`, `resume`, `stop`, `seek`, `set_volume`, `get_volume`, `get_playback_state`, `next_track`, `previous_track`, `get_player_prefs`, `save_player_prefs`
- **Queue** (11 commands): `get_queue`, `add_to_queue`, `remove_from_queue`, `reorder_queue`, `shuffle_queue`, `unshuffle_queue`, `toggle_repeat`, `clear_queue`, `play_queue_track`, `save_queue_state`, `load_saved_queue`
- **Search** (2 commands): `search`, `search_suggestions`
- **Playlists** (7 commands): `get_playlists`, `get_playlist`, `get_playlist_tracks`, `create_playlist`, `add_to_playlist`, `remove_from_playlist`, `delete_playlist`
- **Favorites** (2 commands): `get_favorites`, `toggle_favorite`
- **Browse** (6 commands): `get_album`, `get_album_tracks`, `get_artist`, `get_artist_albums`, `get_recommendations`, `get_similar_tracks`

### Events (Backend to Frontend)

The backend emits events that the frontend subscribes to via listener functions:

| Event | Payload | Purpose |
|-------|---------|---------|
| `playback:progress` | position, duration, position_fraction | Continuous progress updates |
| `playback:track-changed` | track_id, title, artist, album, duration, artwork_url, codec, quality | New track started |
| `playback:state-changed` | state (playing/paused/stopped) | Playback state transitions |
| `playback:track-ended` | (none) | Track finished, triggers auto-advance |
| `queue:changed` | (none) | Queue was modified |

## Tidal API Layer

### Client

`TidalClient` (`api/client.rs`) wraps `reqwest::Client` with the Tidal-specific base URL (`https://openapi.tidal.com/v2`) and JSON:API accept header (`application/vnd.api+json`). It provides generic `get`, `post`, and `delete` methods that handle auth headers and automatic token refresh on 401 responses.

### Auth Flow

Three authentication modes, tried in order during startup:

1. **User auth** (has refresh_token + user_id): refresh the access token and proceed with full API access.
2. **Client credentials** (no user, has client_secret): acquire a catalog-only token for browsing (30-second previews).
3. **Unauthenticated**: prompt the user to log in.

The primary login flow is **device code auth**: the backend requests a device code from Tidal, the frontend displays a user code and verification URL, and the backend polls until the user authorizes. A **PKCE flow** with deep-link callback (`tauritidal://auth/callback`) exists as a fallback.

Token refresh happens automatically. Every API request checks for 401, refreshes the token, and retries once.

### Data Models

API responses follow JSON:API format. The `api/models.rs` file defines the core data types: `Track`, `Album`, `Artist`, `Playlist`, `TokenResponse`, `DeviceAuthResponse`, and artwork resolution helpers that convert Tidal's image resource URLs into sized URLs.

## Queue System

`PlaybackQueue` (`audio/queue.rs`) manages track ordering and playback position:

- **Shuffle**: randomizes track order while keeping the current track at index 0. The original order is saved and restored when shuffle is disabled.
- **Repeat modes**: Off (stop at end), All (wrap to start), One (repeat current track forever).
- **Persistence**: the full queue state (both shuffled and original order) serializes to `~/.tauritidal/queue.json` and restores on startup.

## macOS Integration

### Now Playing

`update_now_playing()` (`macos/now_playing.rs`) dispatches to the main thread via GCD and sets metadata on `MPNowPlayingInfoCenter`: title, artist, album, duration, elapsed time, and playback rate (1.0 for playing, 0.0 for paused).

### Media Keys

`register_media_key_handlers()` (`macos/media_keys.rs`) registers handlers on `MPRemoteCommandCenter` for play, pause, toggle, next, and previous. Each handler is an `RcBlock` closure that emits a Tauri event. The returned `Retained<AnyObject>` tokens must be kept alive for the lifetime of the app, so they are stored in `AppState._media_key_tokens`.

## Error Handling

A single `AppError` enum (`error.rs`) covers all failure modes:

| Variant | Source |
|---------|--------|
| `Http` | Network errors (reqwest) |
| `Json` | Parse errors (serde_json) |
| `Audio` | Audio device/setup failures |
| `Decode` | Symphonia decode errors |
| `AuthRequired` | 401 Unauthorized |
| `TokenExpired` | Refresh token invalid |
| `TidalApi` | API error responses (status + message) |
| `Config` | Config file read/write errors |
| `NotFound` | 404 responses |
| `Io` | File I/O errors |

Errors serialize to `{ "kind": "...", "message": "..." }` so the frontend can handle them structurally.

## Config Persistence

`AppConfig` (`config.rs`) stores all persistent app state in `~/.tauritidal/config.json`:

- Auth tokens (access, refresh, expiration timestamp)
- User profile (ID, display name, country code)
- Audio quality preference (default: LOSSLESS)
- Volume and mute state

The config is loaded at startup and saved after any mutation (token refresh, volume change, login/logout). The directory is created automatically if it does not exist.
