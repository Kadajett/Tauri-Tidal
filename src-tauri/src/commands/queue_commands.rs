use crate::audio::queue::{PersistedQueueState, QueueState, RepeatMode};
use crate::config::AppConfig;
use crate::error::AppError;
use tauri::State;

use crate::AppState;

#[tauri::command]
pub async fn get_queue(state: State<'_, AppState>) -> Result<QueueState, AppError> {
    let queue = state.playback_queue.read().await;
    let mut qs = queue.state();
    for track in &mut qs.tracks {
        track.resolve_artwork();
    }
    Ok(qs)
}

#[tauri::command]
pub async fn add_to_queue(state: State<'_, AppState>, track_id: String) -> Result<(), AppError> {
    let mut track = state.tidal_client.get_track(&track_id).await?;
    track.resolve_artwork();
    let mut queue = state.playback_queue.write().await;
    queue.add_track(track);
    Ok(())
}

#[tauri::command]
pub async fn remove_from_queue(state: State<'_, AppState>, index: usize) -> Result<(), AppError> {
    let mut queue = state.playback_queue.write().await;
    queue.remove_track(index);
    Ok(())
}

#[tauri::command]
pub async fn reorder_queue(
    state: State<'_, AppState>,
    from: usize,
    to: usize,
) -> Result<(), AppError> {
    let mut queue = state.playback_queue.write().await;
    queue.move_track(from, to);
    Ok(())
}

#[tauri::command]
pub async fn shuffle_queue(state: State<'_, AppState>) -> Result<(), AppError> {
    let mut queue = state.playback_queue.write().await;
    queue.shuffle();
    Ok(())
}

#[tauri::command]
pub async fn unshuffle_queue(state: State<'_, AppState>) -> Result<(), AppError> {
    let mut queue = state.playback_queue.write().await;
    queue.unshuffle();
    Ok(())
}

#[tauri::command]
pub async fn toggle_repeat(state: State<'_, AppState>) -> Result<RepeatMode, AppError> {
    let mut queue = state.playback_queue.write().await;
    Ok(queue.toggle_repeat())
}

#[tauri::command]
pub async fn clear_queue(state: State<'_, AppState>) -> Result<(), AppError> {
    let mut queue = state.playback_queue.write().await;
    queue.clear();
    Ok(())
}

#[tauri::command]
pub async fn play_queue_track(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    index: usize,
) -> Result<(), AppError> {
    let track_id = {
        let queue = state.playback_queue.read().await;
        let tracks = &queue.state().tracks;
        tracks
            .get(index)
            .map(|t| t.id.clone())
            .ok_or_else(|| AppError::NotFound("Track index out of bounds".into()))?
    };

    crate::commands::playback_commands::play_track(state, app, track_id).await
}

#[tauri::command]
pub async fn save_queue_state(state: State<'_, AppState>) -> Result<(), AppError> {
    let queue = state.playback_queue.read().await;
    let persisted = queue.persisted_state();
    drop(queue);

    let path = AppConfig::queue_path()?;
    let dir = AppConfig::config_dir()?;
    std::fs::create_dir_all(&dir)?;
    let content = serde_json::to_string_pretty(&persisted)?;
    std::fs::write(&path, content)?;
    Ok(())
}

#[tauri::command]
pub async fn load_saved_queue() -> Result<QueueState, AppError> {
    let path = AppConfig::queue_path()?;
    if !path.exists() {
        return Ok(QueueState {
            tracks: Vec::new(),
            current_index: None,
            repeat_mode: RepeatMode::Off,
            shuffled: false,
        });
    }

    let content = std::fs::read_to_string(&path)?;
    let persisted: PersistedQueueState = serde_json::from_str(&content)?;

    let mut tracks = persisted.tracks;
    for track in &mut tracks {
        track.resolve_artwork();
    }

    // Return the persisted state for the frontend to restore the current track display,
    // but do NOT load it into the backend queue. The queue starts empty on each launch
    // so the queue page only shows tracks from the current session.
    Ok(QueueState {
        tracks,
        current_index: persisted.current_index,
        repeat_mode: persisted.repeat_mode,
        shuffled: persisted.shuffled,
    })
}
