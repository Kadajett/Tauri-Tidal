use crate::api::models::Track;
use crate::error::AppError;
use tauri::State;

use crate::AppState;

#[tauri::command]
pub async fn get_favorites(state: State<'_, AppState>) -> Result<Vec<Track>, AppError> {
    state.tidal_client.get_favorites().await
}

#[tauri::command]
pub async fn toggle_favorite(
    state: State<'_, AppState>,
    track_id: String,
    add: bool,
) -> Result<(), AppError> {
    state.tidal_client.toggle_favorite(&track_id, add).await
}
