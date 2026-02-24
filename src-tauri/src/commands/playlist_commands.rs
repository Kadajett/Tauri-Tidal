use crate::api::models::{Playlist, Track};
use crate::error::AppError;
use tauri::State;

use crate::AppState;

#[tauri::command]
pub async fn get_playlists(state: State<'_, AppState>) -> Result<Vec<Playlist>, AppError> {
    let mut playlists = state.tidal_client.get_playlists().await?;
    for playlist in &mut playlists {
        playlist.resolve_artwork();
    }
    Ok(playlists)
}

#[tauri::command]
pub async fn get_playlist(
    state: State<'_, AppState>,
    playlist_id: String,
) -> Result<Playlist, AppError> {
    let mut playlist = state.tidal_client.get_playlist(&playlist_id).await?;
    playlist.resolve_artwork();
    Ok(playlist)
}

#[tauri::command]
pub async fn get_playlist_tracks(
    state: State<'_, AppState>,
    playlist_id: String,
) -> Result<Vec<Track>, AppError> {
    let mut tracks = state.tidal_client.get_playlist_tracks(&playlist_id).await?;
    for track in &mut tracks {
        track.resolve_artwork();
    }
    Ok(tracks)
}

#[tauri::command]
pub async fn create_playlist(
    state: State<'_, AppState>,
    name: String,
    description: Option<String>,
) -> Result<Playlist, AppError> {
    state
        .tidal_client
        .create_playlist(&name, description.as_deref())
        .await
}

#[tauri::command]
pub async fn add_to_playlist(
    state: State<'_, AppState>,
    playlist_id: String,
    track_id: String,
) -> Result<(), AppError> {
    state
        .tidal_client
        .add_to_playlist(&playlist_id, &track_id)
        .await
}

#[tauri::command]
pub async fn remove_from_playlist(
    state: State<'_, AppState>,
    playlist_id: String,
    track_id: String,
) -> Result<(), AppError> {
    state
        .tidal_client
        .remove_from_playlist(&playlist_id, &track_id)
        .await
}

#[tauri::command]
pub async fn delete_playlist(
    state: State<'_, AppState>,
    playlist_id: String,
) -> Result<(), AppError> {
    state.tidal_client.delete_playlist(&playlist_id).await
}
