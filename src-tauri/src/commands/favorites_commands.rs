use crate::api::models::FavoritesPage;
use crate::error::AppError;
use tauri::State;

use crate::AppState;

#[tauri::command]
pub async fn get_favorites(
    state: State<'_, AppState>,
    cursor: Option<String>,
) -> Result<FavoritesPage, AppError> {
    state.tidal_client.get_favorites(cursor.as_deref()).await
}

#[tauri::command]
pub async fn toggle_favorite(
    state: State<'_, AppState>,
    track_id: String,
    add: bool,
) -> Result<(), AppError> {
    state.tidal_client.toggle_favorite(&track_id, add).await
}
