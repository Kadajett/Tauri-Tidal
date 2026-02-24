use crate::audio::player::AudioPlayer;
use crate::audio::stream_source::HttpStreamSource;
use crate::error::AppError;
use crate::events::{PlaybackState, StateChangedPayload, TrackChangedPayload};
use serde::Serialize;
use tauri::{Emitter, State};

use crate::AppState;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerPrefs {
    pub volume: f32,
    pub muted: bool,
}

#[tauri::command]
pub async fn play_track(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    track_id: String,
) -> Result<(), AppError> {
    log::info!("[play_track] track_id={}", track_id);
    let mut track = state.tidal_client.get_track(&track_id).await?;
    track.resolve_artwork();
    {
        let mut pl = state.preloaded_track.lock().await;
        *pl = None;
    }
    play_track_internal(&state, &app, &track).await
}

/// Play a list of tracks, setting them as the queue with a starting index.
#[tauri::command]
pub async fn play_tracks(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    mut tracks: Vec<crate::api::models::Track>,
    start_index: usize,
) -> Result<(), AppError> {
    log::info!(
        "[play_tracks] {} tracks, start_index={}",
        tracks.len(),
        start_index
    );
    {
        let mut pl = state.preloaded_track.lock().await;
        *pl = None;
    }

    for track in &mut tracks {
        track.resolve_artwork();
    }

    let mut queue = state.playback_queue.write().await;
    queue.set_tracks(tracks, start_index);
    let track = queue.current_track().cloned();
    drop(queue);

    if let Some(track) = track {
        log::info!(
            "[play_tracks] Playing: {} - {}",
            track.artist_name,
            track.title
        );
        play_track_internal(&state, &app, &track).await?;
        let _ = app.emit(crate::events::PLAYBACK_QUEUE_CHANGED, ());
    } else {
        log::warn!("[play_tracks] No track at index {}", start_index);
    }
    Ok(())
}

#[tauri::command]
pub async fn pause(state: State<'_, AppState>, app: tauri::AppHandle) -> Result<(), AppError> {
    let mut player = state.audio_player.write().await;
    player.pause();

    let _ = app.emit(
        crate::events::PLAYBACK_STATE_CHANGED,
        StateChangedPayload {
            state: PlaybackState::Paused,
        },
    );

    #[cfg(target_os = "macos")]
    if let Some(track) = state.current_track.read().await.as_ref() {
        let position = player.position_seconds();
        crate::macos::now_playing::update_now_playing(
            &track.title,
            &track.artist_name,
            &track.album_name,
            track.duration,
            position,
            false,
        );
    }

    Ok(())
}

#[tauri::command]
pub async fn resume(state: State<'_, AppState>, app: tauri::AppHandle) -> Result<(), AppError> {
    let mut player = state.audio_player.write().await;
    player.resume();

    let _ = app.emit(
        crate::events::PLAYBACK_STATE_CHANGED,
        StateChangedPayload {
            state: PlaybackState::Playing,
        },
    );

    #[cfg(target_os = "macos")]
    if let Some(track) = state.current_track.read().await.as_ref() {
        let position = player.position_seconds();
        crate::macos::now_playing::update_now_playing(
            &track.title,
            &track.artist_name,
            &track.album_name,
            track.duration,
            position,
            true,
        );
    }

    Ok(())
}

#[tauri::command]
pub async fn stop(state: State<'_, AppState>, app: tauri::AppHandle) -> Result<(), AppError> {
    let mut player = state.audio_player.write().await;
    player.stop();

    *state.current_track.write().await = None;

    let _ = app.emit(
        crate::events::PLAYBACK_STATE_CHANGED,
        StateChangedPayload {
            state: PlaybackState::Stopped,
        },
    );

    #[cfg(target_os = "macos")]
    crate::macos::now_playing::clear_now_playing();

    Ok(())
}

#[tauri::command]
pub async fn seek(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    position: f64,
) -> Result<(), AppError> {
    let player = state.audio_player.read().await;
    player.seek(position);
    let duration = player.duration_seconds();
    drop(player);

    // Emit progress immediately so the UI reflects the seek position
    let fraction = if duration > 0.0 {
        position / duration
    } else {
        0.0
    };
    let _ = app.emit(
        crate::events::PLAYBACK_PROGRESS,
        crate::events::ProgressPayload {
            position,
            duration,
            position_fraction: fraction,
        },
    );

    Ok(())
}

#[tauri::command]
pub async fn set_volume(state: State<'_, AppState>, volume: f32) -> Result<(), AppError> {
    let player = state.audio_player.read().await;
    player.set_volume(volume);
    Ok(())
}

#[tauri::command]
pub async fn get_volume(state: State<'_, AppState>) -> Result<f32, AppError> {
    let player = state.audio_player.read().await;
    Ok(player.volume())
}

#[tauri::command]
pub async fn get_playback_state(state: State<'_, AppState>) -> Result<String, AppError> {
    let player = state.audio_player.read().await;
    if player.is_playing() {
        Ok("playing".to_string())
    } else {
        Ok("paused".to_string())
    }
}

#[tauri::command]
pub async fn next_track(state: State<'_, AppState>, app: tauri::AppHandle) -> Result<(), AppError> {
    let mut queue = state.playback_queue.write().await;
    let next = queue.next_track().cloned();
    drop(queue);

    match next {
        Some(track) => play_track_internal(&state, &app, &track).await,
        None => {
            let mut player = state.audio_player.write().await;
            player.stop();
            drop(player);
            *state.current_track.write().await = None;
            let _ = app.emit(
                crate::events::PLAYBACK_STATE_CHANGED,
                StateChangedPayload {
                    state: PlaybackState::Stopped,
                },
            );
            #[cfg(target_os = "macos")]
            crate::macos::now_playing::clear_now_playing();
            Ok(())
        }
    }
}

#[tauri::command]
pub async fn previous_track(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), AppError> {
    let player = state.audio_player.read().await;
    let position = player.position_seconds();
    drop(player);

    if position > 15.0 {
        let current = state.current_track.read().await.clone();
        if let Some(track) = current {
            play_track_internal(&state, &app, &track).await?;
        }
    } else {
        let mut queue = state.playback_queue.write().await;
        let prev = queue.previous_track().cloned();
        drop(queue);

        if let Some(track) = prev {
            play_track_internal(&state, &app, &track).await?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn get_player_prefs(state: State<'_, AppState>) -> Result<PlayerPrefs, AppError> {
    let config = state.tidal_client.config().read().await;
    Ok(PlayerPrefs {
        volume: config.volume,
        muted: config.muted,
    })
}

#[tauri::command]
pub async fn save_player_prefs(
    state: State<'_, AppState>,
    volume: f32,
    muted: bool,
) -> Result<(), AppError> {
    let mut config = state.tidal_client.config().write().await;
    config.volume = volume;
    config.muted = muted;
    config.save()?;
    Ok(())
}

/// Internal helper to start playing a track (used by next/previous/play commands)
async fn play_track_internal(
    state: &State<'_, AppState>,
    app: &tauri::AppHandle,
    track: &crate::api::models::Track,
) -> Result<(), AppError> {
    log::info!(
        "[play_track_internal] Starting: id={} title={} artist={}",
        track.id,
        track.title,
        track.artist_name
    );

    // Check for preloaded track first
    let preloaded = {
        let mut pl = state.preloaded_track.lock().await;
        pl.take()
    };

    let mut playback_codec: Option<String> = None;

    if let Some(preloaded) = preloaded.filter(|p| p.track_id == track.id) {
        log::info!("[play_track_internal] Using preloaded track");
        playback_codec = preloaded.codec_hint.clone();
        let codec_hint = preloaded.codec_hint.as_deref();
        let mut player = state.audio_player.write().await;
        player.play_stream(preloaded.source, preloaded.abort_handle, codec_hint, preloaded.duration)?;
    } else {
        // Fetch manifest (contains both URI and codec) and play
        log::info!(
            "[play_track_internal] Fetching manifest for track {}",
            track.id
        );
        let manifest = state.tidal_client.get_track_manifest(&track.id).await?;
        log::info!(
            "[play_track_internal] Got manifest: codec={}, uri={}...",
            manifest.codec,
            &manifest.uri[..manifest.uri.len().min(80)]
        );

        playback_codec = Some(manifest.codec.clone());

        let (source, writer, abort_handle) = HttpStreamSource::new();
        let client = state.tidal_client.http_client().clone();

        // Start the download on a background task
        AudioPlayer::start_download(writer, manifest.uri, client);

        // CRITICAL: play_stream blocks the thread while AudioDecoder probes the format.
        // We must use spawn_blocking so we don't block a tokio worker thread,
        // which would prevent the download task from making progress.
        log::info!("[play_track_internal] Starting play_stream (via spawn_blocking)...");
        let player_ref = state.audio_player.clone();
        let codec = manifest.codec.clone();
        let duration = track.duration;

        let result = tokio::task::spawn_blocking(move || {
            // We need to acquire the write lock inside the blocking task.
            // Use tokio's Handle to enter the async context for the lock.
            let rt = tokio::runtime::Handle::current();
            let mut player = rt.block_on(player_ref.write());
            player.play_stream(source, abort_handle, Some(&codec), duration)
        })
        .await
        .map_err(|e| AppError::Audio(format!("spawn_blocking join error: {}", e)))?;

        result?;
        log::info!("[play_track_internal] play_stream succeeded");
    }

    // Derive a human-friendly quality label from the codec
    let quality_label = playback_codec.as_deref().map(|c| {
        match c.to_lowercase().as_str() {
            "flac" | "flac_hires" => "FLAC",
            "aaclc" | "mp4a.40.2" | "mp4a" | "aac" => "AAC",
            "heaacv1" | "mp4a.40.5" => "AAC",
            "mp3" => "MP3",
            "eac3_joc" => "Atmos",
            other => other,
        }
        .to_string()
    });

    *state.current_track.write().await = Some(track.clone());

    let _ = app.emit(
        crate::events::PLAYBACK_TRACK_CHANGED,
        TrackChangedPayload {
            track_id: track.id.clone(),
            title: track.title.clone(),
            artist: track.artist_name.clone(),
            album: track.album_name.clone(),
            duration: track.duration,
            artwork_url: track.artwork_url_sized(480, 480),
            codec: playback_codec,
            quality: quality_label,
        },
    );

    let _ = app.emit(
        crate::events::PLAYBACK_STATE_CHANGED,
        StateChangedPayload {
            state: PlaybackState::Playing,
        },
    );

    #[cfg(target_os = "macos")]
    crate::macos::now_playing::update_now_playing(
        &track.title,
        &track.artist_name,
        &track.album_name,
        track.duration,
        0.0,
        true,
    );

    log::info!("[play_track_internal] Track playing, events emitted");
    Ok(())
}
