mod api;
mod audio;
mod commands;
mod config;
mod error;
mod events;
#[cfg(target_os = "macos")]
mod macos;

use api::client::TidalClient;
use api::models::Track;
use audio::player::AudioPlayer;
use audio::preloader::PreloadedTrack;
use audio::queue::PlaybackQueue;
use config::AppConfig;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

/// Wrapper to make ObjC retained objects Send+Sync.
/// These tokens are only kept alive, never accessed across threads.
#[cfg(target_os = "macos")]
struct SendRetainedTokens(Vec<objc2::rc::Retained<objc2::runtime::AnyObject>>);
#[cfg(target_os = "macos")]
unsafe impl Send for SendRetainedTokens {}
#[cfg(target_os = "macos")]
unsafe impl Sync for SendRetainedTokens {}

pub struct AppState {
    pub tidal_client: Arc<TidalClient>,
    pub audio_player: Arc<RwLock<AudioPlayer>>,
    pub playback_queue: Arc<RwLock<PlaybackQueue>>,
    pub current_track: Arc<RwLock<Option<Track>>>,
    pub pkce_verifier: Mutex<Option<String>>,
    pub preloaded_track: Mutex<Option<PreloadedTrack>>,
    /// Keep media key handler tokens alive for the lifetime of the app (macOS only)
    #[cfg(target_os = "macos")]
    _media_key_tokens: std::sync::Mutex<SendRetainedTokens>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("tauritidal=info"),
    )
    .init();

    // Install a panic hook that writes the panic message to a file.
    // This captures the error before abort() kills the process when
    // a panic occurs inside an extern "C" (ObjC) callback frame.
    std::panic::set_hook(Box::new(|info| {
        use std::io::Write;
        let crash_path = dirs::home_dir()
            .unwrap_or_default()
            .join(".tauritidal")
            .join("crash.log");
        let _ = std::fs::create_dir_all(crash_path.parent().unwrap());
        let msg = format!(
            "PANIC at {}: {}\nBacktrace:\n{}\n---\n",
            info.location()
                .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
                .unwrap_or_else(|| "unknown".into()),
            info.payload()
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| info.payload().downcast_ref::<String>().map(|s| s.as_str()))
                .unwrap_or("(no message)"),
            std::backtrace::Backtrace::force_capture(),
        );
        // Append so we capture the FIRST panic before the "cannot unwind" overwrites it
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&crash_path)
        {
            let _ = f.write_all(msg.as_bytes());
        }
        eprintln!("{}", msg);
    }));

    let config = AppConfig::load().unwrap_or_else(|e| {
        log::warn!("Failed to load config: {}. Using defaults.", e);
        let default_config = AppConfig::default();
        // Save defaults so the config file exists for next launch
        if let Err(save_err) = default_config.save() {
            log::error!("Failed to save default config: {}", save_err);
        }
        default_config
    });

    // Read volume/muted before wrapping config in Arc<RwLock>
    let restored_volume = if config.muted { 0.0 } else { config.volume };

    let config = Arc::new(RwLock::new(config));
    let tidal_client =
        Arc::new(TidalClient::new(Arc::clone(&config)).expect("Failed to create Tidal client"));

    let audio_player = Arc::new(RwLock::new({
        let player = AudioPlayer::new().expect("Failed to initialize audio player");
        player.set_volume(restored_volume);
        player
    }));

    let playback_queue = Arc::new(RwLock::new(PlaybackQueue::new()));
    let current_track: Arc<RwLock<Option<Track>>> = Arc::new(RwLock::new(None));

    let player_for_progress = Arc::clone(&audio_player);
    let track_for_progress = Arc::clone(&current_track);
    let queue_for_progress = Arc::clone(&playback_queue);
    let client_for_progress = Arc::clone(&tidal_client);

    // Auto-acquire client credentials token on startup if no token exists
    let client_for_init = Arc::clone(&tidal_client);
    let config_for_init = Arc::clone(&config);

    let app_state = AppState {
        tidal_client,
        audio_player,
        playback_queue,
        current_track,
        pkce_verifier: Mutex::new(None),
        preloaded_track: Mutex::new(None),
        #[cfg(target_os = "macos")]
        _media_key_tokens: std::sync::Mutex::new(SendRetainedTokens(Vec::new())),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_deep_link::init())
        .manage(app_state)
        .setup(move |app| {
            let app_handle = app.handle().clone();

            // Auto-refresh or acquire token on startup.
            // Priority: refresh user token > client credentials fallback.
            let init_client = Arc::clone(&client_for_init);
            let init_config = Arc::clone(&config_for_init);
            tauri::async_runtime::spawn(async move {
                let config = init_config.read().await;
                let client_id = config.client_id.clone();
                let client_secret = config.client_secret.clone();
                let refresh_token = config.refresh_token.clone();
                let has_user_id = config.user_id.is_some();
                drop(config);

                if client_id.is_empty() {
                    return;
                }

                // If user was previously logged in (has refresh_token), ALWAYS refresh.
                // We cannot tell if the saved token is a user PKCE token or a client_credentials
                // token just by looking at it. A previous bug could have overwritten the user
                // token with a client_credentials one that appears "valid" but only gives
                // 30-second previews. Refreshing always gives us a proper user token.
                if let Some(ref rt) = refresh_token {
                    if has_user_id {
                        log::info!(
                            "Refreshing user PKCE token (always refresh for logged-in users)..."
                        );
                        match api::auth::refresh_user_token(
                            init_client.http_client(),
                            &client_id,
                            rt,
                        )
                        .await
                        {
                            Ok(token) => {
                                let mut config = init_config.write().await;
                                config.access_token = Some(token.access_token);
                                config.expires_at = Some(
                                    chrono::Utc::now()
                                        + chrono::Duration::seconds(token.expires_in as i64),
                                );
                                // Update refresh token if a new one was provided
                                if let Some(new_rt) = token.refresh_token {
                                    config.refresh_token = Some(new_rt);
                                }
                                if let Err(e) = config.save() {
                                    log::error!("Failed to save refreshed token: {}", e);
                                } else {
                                    log::info!("User token refreshed successfully");
                                }
                                return;
                            }
                            Err(e) => {
                                log::warn!(
                                    "Token refresh failed: {}. User will need to re-login.",
                                    e
                                );
                                // Do NOT fall through to client_credentials when a user was
                                // previously logged in. Client credentials tokens only give
                                // 30-second previews, silently degrading the experience.
                                return;
                            }
                        }
                    }
                }

                // Client credentials require a client_secret and only provide
                // catalog-only (30s preview) access. Skip if no secret or user was logged in.
                if has_user_id || client_secret.is_empty() {
                    log::info!(
                        "Skipping client credentials (no secret or user was previously logged in)"
                    );
                    return;
                }

                // Check if we already have a valid client credentials token
                let config = init_config.read().await;
                let needs_token = config.access_token.is_none() || config.is_token_expired();
                drop(config);

                if !needs_token {
                    log::info!("Client credentials token still valid, skipping");
                    return;
                }

                log::info!("Acquiring client credentials token (no user login history)...");
                match api::auth::client_credentials_token(
                    init_client.http_client(),
                    &client_id,
                    &client_secret,
                )
                .await
                {
                    Ok(token) => {
                        let mut config = init_config.write().await;
                        config.access_token = Some(token.access_token);
                        config.expires_at = Some(
                            chrono::Utc::now() + chrono::Duration::seconds(token.expires_in as i64),
                        );
                        if let Err(e) = config.save() {
                            log::error!("Failed to save token: {}", e);
                        } else {
                            log::info!("Client credentials token acquired (catalog-only access)");
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to acquire client credentials: {}", e);
                    }
                }
            });

            // Defer media key registration until after app finishes launching (macOS only).
            // Calling ObjC MediaPlayer APIs synchronously during applicationDidFinishLaunching
            // causes a panic that cannot unwind through ObjC frames, resulting in SIGABRT.
            #[cfg(target_os = "macos")]
            {
                let deferred_handle = app.handle().clone();
                let deferred_player = Arc::clone(&player_for_progress);
                let deferred_queue = Arc::clone(&queue_for_progress);
                let deferred_track = Arc::clone(&track_for_progress);
                let deferred_client = Arc::clone(&client_for_progress);
                tauri::async_runtime::spawn(async move {
                    // Give the app time to finish launching before touching MediaPlayer framework
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

                    // Register media keys on the main thread (ObjC requirement)
                    let reg_handle = deferred_handle.clone();
                    let _ = deferred_handle.run_on_main_thread(move || {
                        use tauri::Manager;
                        let tokens =
                            macos::media_keys::register_media_key_handlers(reg_handle.clone());
                        let state = reg_handle.state::<AppState>();
                        *state._media_key_tokens.lock().unwrap() = SendRetainedTokens(tokens);
                        log::info!("Media keys registered (deferred)");
                    });

                    // Set up event listeners for media key events
                    use tauri::Listener;

                    // Toggle play/pause
                    let media_player = Arc::clone(&deferred_player);
                    let media_track = Arc::clone(&deferred_track);
                    let media_handle = deferred_handle.clone();
                    deferred_handle.listen(
                        macos::media_keys::MEDIA_KEY_TOGGLE_PLAY,
                        move |event: tauri::Event| {
                            let player = Arc::clone(&media_player);
                            let track = Arc::clone(&media_track);
                            let handle = media_handle.clone();
                            let payload = event.payload().to_string();
                            tauri::async_runtime::spawn(async move {
                                use tauri::Emitter;
                                let is_playing = player.read().await.is_playing();
                                match payload.trim_matches('"') {
                                    "play" => {
                                        if !is_playing {
                                            player.write().await.resume();
                                            let _ = handle.emit(
                                                events::PLAYBACK_STATE_CHANGED,
                                                events::StateChangedPayload {
                                                    state: events::PlaybackState::Playing,
                                                },
                                            );
                                        }
                                    }
                                    "pause" => {
                                        if is_playing {
                                            player.write().await.pause();
                                            let _ = handle.emit(
                                                events::PLAYBACK_STATE_CHANGED,
                                                events::StateChangedPayload {
                                                    state: events::PlaybackState::Paused,
                                                },
                                            );
                                        }
                                    }
                                    _ => {
                                        // toggle
                                        if is_playing {
                                            player.write().await.pause();
                                            let _ = handle.emit(
                                                events::PLAYBACK_STATE_CHANGED,
                                                events::StateChangedPayload {
                                                    state: events::PlaybackState::Paused,
                                                },
                                            );
                                        } else {
                                            player.write().await.resume();
                                            let _ = handle.emit(
                                                events::PLAYBACK_STATE_CHANGED,
                                                events::StateChangedPayload {
                                                    state: events::PlaybackState::Playing,
                                                },
                                            );
                                        }
                                    }
                                }
                                // Update now playing
                                let p = player.read().await;
                                if let Some(t) = track.read().await.as_ref() {
                                    macos::now_playing::update_now_playing(
                                        &t.title,
                                        &t.artist_name,
                                        &t.album_name,
                                        t.duration,
                                        p.position_seconds(),
                                        p.is_playing(),
                                    );
                                }
                            });
                        },
                    );

                    // Next track
                    let next_player = Arc::clone(&deferred_player);
                    let next_queue = Arc::clone(&deferred_queue);
                    let next_track = Arc::clone(&deferred_track);
                    let next_client = Arc::clone(&deferred_client);
                    let next_handle = deferred_handle.clone();
                    deferred_handle.listen(
                        macos::media_keys::MEDIA_KEY_NEXT,
                        move |_event: tauri::Event| {
                            let player = Arc::clone(&next_player);
                            let queue = Arc::clone(&next_queue);
                            let track_ref = Arc::clone(&next_track);
                            let client = Arc::clone(&next_client);
                            let handle = next_handle.clone();
                            tauri::async_runtime::spawn(async move {
                                use tauri::Emitter;
                                let mut q = queue.write().await;
                                let next = q.next_track().cloned();
                                drop(q);

                                if let Some(next_trk) = next {
                                    match client.get_track_manifest(&next_trk.id).await {
                                        Ok(manifest) => {
                                            let (source, writer, abort_handle) =
                                                audio::stream_source::HttpStreamSource::new();
                                            AudioPlayer::start_download(
                                                writer,
                                                manifest.uri,
                                                client.http_client().clone(),
                                            );
                                            // Use spawn_blocking to avoid deadlocking tokio
                                            let player_ref = Arc::clone(&player);
                                            let codec = manifest.codec.clone();
                                            let duration = next_trk.duration;
                                            let result = tokio::task::spawn_blocking(move || {
                                                let rt = tokio::runtime::Handle::current();
                                                let mut p = rt.block_on(player_ref.write());
                                                p.play_stream(source, abort_handle, Some(&codec), duration)
                                            })
                                            .await;
                                            match result {
                                                Ok(Ok(())) => {}
                                                Ok(Err(e)) => {
                                                    log::error!(
                                                        "Media key next play failed: {}",
                                                        e
                                                    );
                                                    return;
                                                }
                                                Err(e) => {
                                                    log::error!("Media key next join error: {}", e);
                                                    return;
                                                }
                                            }
                                            *track_ref.write().await = Some(next_trk.clone());
                                            let _ = handle.emit(
                                                events::PLAYBACK_TRACK_CHANGED,
                                                events::TrackChangedPayload {
                                                    track_id: next_trk.id.clone(),
                                                    title: next_trk.title.clone(),
                                                    artist: next_trk.artist_name.clone(),
                                                    album: next_trk.album_name.clone(),
                                                    duration: next_trk.duration,
                                                    artwork_url: next_trk
                                                        .artwork_url_sized(640, 640),
                                                    codec: None,
                                                    quality: None,
                                                },
                                            );
                                            let _ = handle.emit(
                                                events::PLAYBACK_STATE_CHANGED,
                                                events::StateChangedPayload {
                                                    state: events::PlaybackState::Playing,
                                                },
                                            );
                                            macos::now_playing::update_now_playing(
                                                &next_trk.title,
                                                &next_trk.artist_name,
                                                &next_trk.album_name,
                                                next_trk.duration,
                                                0.0,
                                                true,
                                            );
                                        }
                                        Err(e) => {
                                            log::error!("Media key next failed: {}", e)
                                        }
                                    }
                                }
                            });
                        },
                    );

                    // Previous track
                    let prev_player = Arc::clone(&deferred_player);
                    let prev_queue = Arc::clone(&deferred_queue);
                    let prev_track = Arc::clone(&deferred_track);
                    let prev_client = Arc::clone(&deferred_client);
                    let prev_handle = deferred_handle.clone();
                    deferred_handle.listen(
                        macos::media_keys::MEDIA_KEY_PREVIOUS,
                        move |_event: tauri::Event| {
                            let player = Arc::clone(&prev_player);
                            let queue = Arc::clone(&prev_queue);
                            let track_ref = Arc::clone(&prev_track);
                            let client = Arc::clone(&prev_client);
                            let handle = prev_handle.clone();
                            tauri::async_runtime::spawn(async move {
                                use tauri::Emitter;
                                let position = player.read().await.position_seconds();
                                if position > 15.0 {
                                    // Restart current track
                                    if let Some(current) = track_ref.read().await.clone() {
                                        match client.get_track_manifest(&current.id).await {
                                            Ok(manifest) => {
                                                let (source, writer, abort_handle) =
                                                    audio::stream_source::HttpStreamSource::new();
                                                AudioPlayer::start_download(
                                                    writer,
                                                    manifest.uri,
                                                    client.http_client().clone(),
                                                );
                                                let player_ref = Arc::clone(&player);
                                                let codec = manifest.codec.clone();
                                                let dur = current.duration;
                                                let result =
                                                    tokio::task::spawn_blocking(move || {
                                                        let rt = tokio::runtime::Handle::current();
                                                        let mut p = rt.block_on(player_ref.write());
                                                        p.play_stream(source, abort_handle, Some(&codec), dur)
                                                    })
                                                    .await;
                                                if let Err(e) = result.unwrap_or_else(|e| {
                                                    Err(crate::error::AppError::Audio(format!(
                                                        "join error: {}",
                                                        e
                                                    )))
                                                }) {
                                                    log::error!(
                                                        "Media key prev restart failed: {}",
                                                        e
                                                    );
                                                }
                                                macos::now_playing::update_now_playing(
                                                    &current.title,
                                                    &current.artist_name,
                                                    &current.album_name,
                                                    current.duration,
                                                    0.0,
                                                    true,
                                                );
                                            }
                                            Err(e) => log::error!(
                                                "Media key prev restart manifest failed: {}",
                                                e
                                            ),
                                        }
                                    }
                                } else {
                                    let mut q = queue.write().await;
                                    let prev = q.previous_track().cloned();
                                    drop(q);

                                    if let Some(prev_trk) = prev {
                                        match client.get_track_manifest(&prev_trk.id).await {
                                            Ok(manifest) => {
                                                let (source, writer, abort_handle) =
                                                    audio::stream_source::HttpStreamSource::new();
                                                AudioPlayer::start_download(
                                                    writer,
                                                    manifest.uri,
                                                    client.http_client().clone(),
                                                );
                                                let player_ref = Arc::clone(&player);
                                                let codec = manifest.codec.clone();
                                                let dur = prev_trk.duration;
                                                let result =
                                                    tokio::task::spawn_blocking(move || {
                                                        let rt = tokio::runtime::Handle::current();
                                                        let mut p = rt.block_on(player_ref.write());
                                                        p.play_stream(source, abort_handle, Some(&codec), dur)
                                                    })
                                                    .await;
                                                match result {
                                                    Ok(Ok(())) => {}
                                                    Ok(Err(e)) => {
                                                        log::error!(
                                                            "Media key prev play failed: {}",
                                                            e
                                                        );
                                                        return;
                                                    }
                                                    Err(e) => {
                                                        log::error!(
                                                            "Media key prev join error: {}",
                                                            e
                                                        );
                                                        return;
                                                    }
                                                }
                                                *track_ref.write().await = Some(prev_trk.clone());
                                                let _ = handle.emit(
                                                    events::PLAYBACK_TRACK_CHANGED,
                                                    events::TrackChangedPayload {
                                                        track_id: prev_trk.id.clone(),
                                                        title: prev_trk.title.clone(),
                                                        artist: prev_trk.artist_name.clone(),
                                                        album: prev_trk.album_name.clone(),
                                                        duration: prev_trk.duration,
                                                        artwork_url: prev_trk
                                                            .artwork_url_sized(640, 640),
                                                        codec: None,
                                                        quality: None,
                                                    },
                                                );
                                                let _ = handle.emit(
                                                    events::PLAYBACK_STATE_CHANGED,
                                                    events::StateChangedPayload {
                                                        state: events::PlaybackState::Playing,
                                                    },
                                                );
                                                macos::now_playing::update_now_playing(
                                                    &prev_trk.title,
                                                    &prev_trk.artist_name,
                                                    &prev_trk.album_name,
                                                    prev_trk.duration,
                                                    0.0,
                                                    true,
                                                );
                                            }
                                            Err(e) => log::error!("Media key prev failed: {}", e),
                                        }
                                    }
                                }
                            });
                        },
                    );
                });
            }

            // Start progress emission + auto-advance + preload loop
            tauri::async_runtime::spawn(async move {
                use tauri::{Emitter, Manager};
                let mut preload_triggered = false;
                let mut advancing = false; // Guard against re-entering auto-advance

                loop {
                    tokio::time::sleep(std::time::Duration::from_millis(250)).await;

                    // Skip polling while we're in the middle of advancing to the next track
                    if advancing {
                        continue;
                    }

                    let player = player_for_progress.read().await;
                    let is_playing = player.is_playing();
                    let is_finished = player.is_finished();
                    let position = player.position_seconds();
                    let duration = player.duration_seconds();
                    drop(player);

                    // Debug: log state near end of track
                    if duration > 0.0 && position > 0.0 {
                        let remaining = duration - position;
                        if remaining < 5.0 || is_finished {
                            log::info!(
                                "[progress] pos={:.1} dur={:.1} rem={:.1} playing={} finished={}",
                                position, duration, remaining, is_playing, is_finished,
                            );
                        }
                    }

                    if is_playing {
                        let fraction = if duration > 0.0 {
                            position / duration
                        } else {
                            0.0
                        };
                        let _ = app_handle.emit(
                            events::PLAYBACK_PROGRESS,
                            events::ProgressPayload {
                                position,
                                duration,
                                position_fraction: fraction,
                            },
                        );

                        #[cfg(target_os = "macos")]
                        if let Some(track) = track_for_progress.read().await.as_ref() {
                            macos::now_playing::update_now_playing(
                                &track.title,
                                &track.artist_name,
                                &track.album_name,
                                track.duration,
                                position,
                                true,
                            );
                        }

                        // Preload next track when within 30s of the end.
                        // Use duration > 0.0 to avoid div-by-zero; drop the remaining > 0.0
                        // check since position can slightly overshoot duration due to
                        // sample counting vs API metadata mismatch.
                        let remaining = duration - position;
                        if duration > 0.0 && remaining < 30.0 && !preload_triggered {
                            preload_triggered = true;
                            let queue = queue_for_progress.read().await;
                            if let Some(next) = queue.peek_next() {
                                let next_id = next.id.clone();
                                let next_duration = next.duration;
                                let client = Arc::clone(&client_for_progress);
                                let app_h = app_handle.clone();
                                tauri::async_runtime::spawn(async move {
                                    log::info!("Preloading next track: {}", next_id);
                                    match client.get_track_manifest(&next_id).await {
                                        Ok(manifest) => {
                                            let preloaded = PreloadedTrack::new(
                                                next_id,
                                                Some(manifest.codec),
                                                next_duration,
                                                manifest.uri,
                                                client.http_client().clone(),
                                            );
                                            let state: tauri::State<'_, AppState> =
                                                app_h.state::<AppState>();
                                            let mut pl = state.preloaded_track.lock().await;
                                            *pl = Some(preloaded);
                                            log::info!("Next track preloaded successfully");
                                        }
                                        Err(e) => log::warn!("Preload manifest failed: {}", e),
                                    }
                                });
                            }
                        }
                    }

                    // Auto-advance when track finishes
                    if is_finished && duration > 0.0 {
                        advancing = true; // Block re-entry while we fetch/play
                        log::info!("Track finished, auto-advancing...");
                        let _ = app_handle.emit(events::PLAYBACK_TRACK_ENDED, ());

                        // Stop the old player immediately so is_finished resets
                        {
                            let mut player = player_for_progress.write().await;
                            player.stop();
                        }

                        // Advance queue
                        let mut queue = queue_for_progress.write().await;
                        let next = queue.next_track().cloned();
                        drop(queue);

                        if let Some(next_track) = next {
                            // Check if we have a preloaded track
                            let state: tauri::State<'_, AppState> = app_handle.state::<AppState>();
                            let preloaded: Option<PreloadedTrack> = {
                                let mut pl = state.preloaded_track.lock().await;
                                pl.take()
                            };

                            if let Some(preloaded) =
                                preloaded.filter(|p| p.track_id == next_track.id)
                            {
                                log::info!("Using preloaded track for gapless playback");
                                // Use spawn_blocking so the blocking format-probe
                                // inside play_stream doesn't stall the Tokio runtime.
                                let player_ref = Arc::clone(&player_for_progress);
                                let result = tokio::task::spawn_blocking(move || {
                                    let rt = tokio::runtime::Handle::current();
                                    let mut player = rt.block_on(player_ref.write());
                                    let codec_hint = preloaded.codec_hint.as_deref();
                                    player.play_stream(
                                        preloaded.source,
                                        preloaded.abort_handle,
                                        codec_hint,
                                        preloaded.duration,
                                    )
                                })
                                .await;
                                match result {
                                    Ok(Ok(())) => {}
                                    Ok(Err(e)) => {
                                        log::error!("Failed to play preloaded track: {}", e);
                                        advancing = false;
                                        continue;
                                    }
                                    Err(e) => {
                                        log::error!("spawn_blocking join error: {}", e);
                                        advancing = false;
                                        continue;
                                    }
                                }
                            } else {
                                // Fetch and play normally
                                let client = &client_for_progress;
                                match client.get_track_manifest(&next_track.id).await {
                                    Ok(manifest) => {
                                        let (source, writer, abort_handle) =
                                            audio::stream_source::HttpStreamSource::new();
                                        AudioPlayer::start_download(
                                            writer,
                                            manifest.uri,
                                            client.http_client().clone(),
                                        );
                                        // Use spawn_blocking so the blocking format-probe
                                        // inside play_stream doesn't stall the Tokio runtime
                                        // and deadlock with the download task.
                                        let player_ref = Arc::clone(&player_for_progress);
                                        let codec = manifest.codec.clone();
                                        let duration = next_track.duration;
                                        let result = tokio::task::spawn_blocking(move || {
                                            let rt = tokio::runtime::Handle::current();
                                            let mut player = rt.block_on(player_ref.write());
                                            player.play_stream(
                                                source,
                                                abort_handle,
                                                Some(&codec),
                                                duration,
                                            )
                                        })
                                        .await;
                                        match result {
                                            Ok(Ok(())) => {}
                                            Ok(Err(e)) => {
                                                log::error!("Failed to play next track: {}", e);
                                                advancing = false;
                                                continue;
                                            }
                                            Err(e) => {
                                                log::error!("spawn_blocking join error: {}", e);
                                                advancing = false;
                                                continue;
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        log::error!("Failed to get manifest for next track: {}", e);
                                        advancing = false;
                                        continue;
                                    }
                                }
                            }

                            *track_for_progress.write().await = Some(next_track.clone());

                            let _ = app_handle.emit(
                                events::PLAYBACK_TRACK_CHANGED,
                                events::TrackChangedPayload {
                                    track_id: next_track.id.clone(),
                                    title: next_track.title.clone(),
                                    artist: next_track.artist_name.clone(),
                                    album: next_track.album_name.clone(),
                                    duration: next_track.duration,
                                    artwork_url: next_track.artwork_url_sized(640, 640),
                                    codec: None,
                                    quality: None,
                                },
                            );

                            let _ = app_handle.emit(
                                events::PLAYBACK_STATE_CHANGED,
                                events::StateChangedPayload {
                                    state: events::PlaybackState::Playing,
                                },
                            );

                            let _ = app_handle.emit(events::PLAYBACK_QUEUE_CHANGED, ());

                            preload_triggered = false;
                        } else {
                            // No next track, already stopped above
                            *track_for_progress.write().await = None;

                            let _ = app_handle.emit(
                                events::PLAYBACK_STATE_CHANGED,
                                events::StateChangedPayload {
                                    state: events::PlaybackState::Stopped,
                                },
                            );
                            #[cfg(target_os = "macos")]
                            macos::now_playing::clear_now_playing();
                        }
                        advancing = false;
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Auth
            commands::auth_commands::check_auth_status,
            commands::auth_commands::login,
            commands::auth_commands::poll_login,
            commands::auth_commands::handle_auth_callback,
            commands::auth_commands::init_client_credentials,
            commands::auth_commands::logout,
            // Playback
            commands::playback_commands::play_track,
            commands::playback_commands::play_tracks,
            commands::playback_commands::pause,
            commands::playback_commands::resume,
            commands::playback_commands::stop,
            commands::playback_commands::seek,
            commands::playback_commands::set_volume,
            commands::playback_commands::get_volume,
            commands::playback_commands::get_playback_state,
            commands::playback_commands::get_player_prefs,
            commands::playback_commands::save_player_prefs,
            commands::playback_commands::next_track,
            commands::playback_commands::previous_track,
            // Queue
            commands::queue_commands::get_queue,
            commands::queue_commands::add_to_queue,
            commands::queue_commands::remove_from_queue,
            commands::queue_commands::reorder_queue,
            commands::queue_commands::shuffle_queue,
            commands::queue_commands::unshuffle_queue,
            commands::queue_commands::toggle_repeat,
            commands::queue_commands::clear_queue,
            commands::queue_commands::play_queue_track,
            commands::queue_commands::save_queue_state,
            commands::queue_commands::load_saved_queue,
            // Search
            commands::search_commands::search,
            commands::search_commands::search_suggestions,
            // Playlists
            commands::playlist_commands::get_playlists,
            commands::playlist_commands::get_playlist,
            commands::playlist_commands::get_playlist_tracks,
            commands::playlist_commands::create_playlist,
            commands::playlist_commands::add_to_playlist,
            commands::playlist_commands::remove_from_playlist,
            commands::playlist_commands::delete_playlist,
            // Favorites
            commands::favorites_commands::get_favorites,
            commands::favorites_commands::toggle_favorite,
            // Browse
            commands::browse_commands::get_album,
            commands::browse_commands::get_album_tracks,
            commands::browse_commands::get_artist,
            commands::browse_commands::get_artist_albums,
            commands::browse_commands::get_recommendations,
            commands::browse_commands::get_similar_tracks,
            // Images
            commands::image_commands::proxy_image,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
