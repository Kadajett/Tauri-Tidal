use crate::api::models::{Album, Artist, RecommendationSection, Track};
use crate::error::AppError;
use tauri::State;

use crate::AppState;

#[tauri::command]
pub async fn get_album(state: State<'_, AppState>, album_id: String) -> Result<Album, AppError> {
    let mut album = state.tidal_client.get_album(&album_id).await?;
    album.resolve_artwork();
    Ok(album)
}

#[tauri::command]
pub async fn get_album_tracks(
    state: State<'_, AppState>,
    album_id: String,
) -> Result<Vec<Track>, AppError> {
    let mut tracks = state.tidal_client.get_album_tracks(&album_id).await?;
    for track in &mut tracks {
        track.resolve_artwork();
    }
    Ok(tracks)
}

#[tauri::command]
pub async fn get_artist(state: State<'_, AppState>, artist_id: String) -> Result<Artist, AppError> {
    let mut artist = state.tidal_client.get_artist(&artist_id).await?;
    artist.resolve_artwork();
    Ok(artist)
}

#[tauri::command]
pub async fn get_artist_albums(
    state: State<'_, AppState>,
    artist_id: String,
) -> Result<Vec<Album>, AppError> {
    let mut albums = state.tidal_client.get_artist_albums(&artist_id).await?;
    for album in &mut albums {
        album.resolve_artwork();
    }
    Ok(albums)
}

#[tauri::command]
pub async fn get_recommendations(
    state: State<'_, AppState>,
) -> Result<Vec<RecommendationSection>, AppError> {
    let mut sections = state.tidal_client.get_recommendations().await?;
    for section in &mut sections {
        for track in &mut section.tracks {
            track.resolve_artwork();
        }
    }
    Ok(sections)
}

#[tauri::command]
pub async fn get_similar_tracks(
    state: State<'_, AppState>,
    track_id: String,
) -> Result<Vec<Track>, AppError> {
    let mut tracks = state.tidal_client.get_similar_tracks(&track_id).await?;
    for track in &mut tracks {
        track.resolve_artwork();
    }
    Ok(tracks)
}
