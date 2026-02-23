use crate::api::models::{Album, Artist, Track};
use crate::error::AppError;
use tauri::State;

use crate::AppState;

#[tauri::command]
pub async fn get_album(state: State<'_, AppState>, album_id: String) -> Result<Album, AppError> {
    state.tidal_client.get_album(&album_id).await
}

#[tauri::command]
pub async fn get_album_tracks(
    state: State<'_, AppState>,
    album_id: String,
) -> Result<Vec<Track>, AppError> {
    state.tidal_client.get_album_tracks(&album_id).await
}

#[tauri::command]
pub async fn get_artist(state: State<'_, AppState>, artist_id: String) -> Result<Artist, AppError> {
    state.tidal_client.get_artist(&artist_id).await
}

#[tauri::command]
pub async fn get_artist_albums(
    state: State<'_, AppState>,
    artist_id: String,
) -> Result<Vec<Album>, AppError> {
    state.tidal_client.get_artist_albums(&artist_id).await
}

#[tauri::command]
pub async fn get_recommendations(state: State<'_, AppState>) -> Result<Vec<Track>, AppError> {
    state.tidal_client.get_recommendations().await
}

#[tauri::command]
pub async fn get_similar_tracks(
    state: State<'_, AppState>,
    track_id: String,
) -> Result<Vec<Track>, AppError> {
    state.tidal_client.get_similar_tracks(&track_id).await
}
