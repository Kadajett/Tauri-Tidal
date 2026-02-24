use crate::api::models::SearchResults;
use crate::error::AppError;
use tauri::State;

use crate::AppState;

#[tauri::command]
pub async fn search(
    state: State<'_, AppState>,
    query: String,
    limit: Option<u32>,
) -> Result<SearchResults, AppError> {
    let limit = limit.unwrap_or(20);
    let mut results = state.tidal_client.search(&query, limit).await?;
    results.resolve_all_artwork();
    Ok(results)
}

#[tauri::command]
pub async fn search_suggestions(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<String>, AppError> {
    state.tidal_client.search_suggestions(&query).await
}
