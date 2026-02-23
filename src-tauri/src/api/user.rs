use crate::api::client::TidalClient;
use crate::api::models::Track;
use crate::api::search::{get_first_relationship_id, parse_track};
use crate::error::{AppError, AppResult};
use std::collections::HashMap;

/// Build artist/album/artwork lookup maps from an included resources array.
/// Shared by all functions in this file that need to resolve track relationships.
fn build_track_lookup_maps(
    included: Option<&Vec<serde_json::Value>>,
) -> (
    HashMap<String, String>,                   // artist_id -> name
    HashMap<String, (String, Option<String>)>, // album_id -> (title, artwork_url)
) {
    let mut artist_map: HashMap<String, String> = HashMap::new();
    let mut album_map: HashMap<String, (String, Option<String>)> = HashMap::new();
    let mut artwork_map: HashMap<String, String> = HashMap::new();

    let items = match included {
        Some(items) => items,
        None => return (artist_map, album_map),
    };

    // First pass: extract artworks
    for item in items {
        if item.get("type").and_then(|v| v.as_str()) == Some("artworks") {
            let id = item
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if let Some(href) = item
                .get("attributes")
                .and_then(|a| a.get("files"))
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.last().or(arr.first()))
                .and_then(|f| f.get("href"))
                .and_then(|v| v.as_str())
            {
                artwork_map.insert(id, href.to_string());
            }
        }
    }

    // Second pass: extract artists and albums
    for item in items {
        let rtype = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let rid = item
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        match rtype {
            "artists" => {
                if let Some(name) = item
                    .get("attributes")
                    .and_then(|a| a.get("name"))
                    .and_then(|v| v.as_str())
                {
                    artist_map.insert(rid, name.to_string());
                }
            }
            "albums" => {
                if let Some(title) = item
                    .get("attributes")
                    .and_then(|a| a.get("title"))
                    .and_then(|v| v.as_str())
                {
                    let artwork = get_first_relationship_id(item.get("relationships"), "coverArt")
                        .and_then(|art_id| artwork_map.get(&art_id).cloned());
                    album_map.insert(rid, (title.to_string(), artwork));
                }
            }
            _ => {}
        }
    }

    (artist_map, album_map)
}

/// Parse tracks from an included array, resolving artist/album relationships.
fn parse_tracks_from_included(included: Option<&Vec<serde_json::Value>>) -> Vec<Track> {
    let items = match included {
        Some(items) => items,
        None => return Vec::new(),
    };

    let (artist_map, album_map) = build_track_lookup_maps(Some(items));

    let mut tracks = Vec::new();
    for item in items {
        let resource_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if resource_type == "tracks" {
            let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let attrs = item.get("attributes").cloned().unwrap_or_default();
            let rels = item.get("relationships");
            if let Some(mut track) = parse_track(id, &attrs) {
                // Resolve artist from relationships
                if let Some(artist_id) = get_first_relationship_id(rels, "artists") {
                    if let Some(name) = artist_map.get(&artist_id) {
                        track.artist_name = name.clone();
                        track.artist_id = Some(artist_id);
                    }
                }
                // Resolve album from relationships
                if let Some(album_id) = get_first_relationship_id(rels, "albums") {
                    if let Some((title, artwork)) = album_map.get(&album_id) {
                        track.album_name = title.clone();
                        track.album_id = Some(album_id);
                        if track.artwork_url.is_none() {
                            track.artwork_url = artwork.clone();
                        }
                    }
                }
                tracks.push(track);
            }
        }
    }

    tracks
}

/// Parse tracks from a v1 API mix items response.
/// The v1 format has { items: [{ item: { id, title, duration, artists: [...], album: {...} }, type: "track" }] }
fn parse_v1_mix_items(body: &serde_json::Value) -> Vec<Track> {
    let mut tracks = Vec::new();
    let items = match body.get("items").and_then(|v| v.as_array()) {
        Some(items) => items,
        None => return tracks,
    };

    for entry in items {
        let item_type = entry.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if item_type != "track" {
            continue;
        }
        let item = match entry.get("item") {
            Some(i) => i,
            None => continue,
        };

        let id = match item.get("id") {
            Some(serde_json::Value::Number(n)) => n.to_string(),
            Some(serde_json::Value::String(s)) => s.clone(),
            _ => continue,
        };

        let title = item
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let duration = item
            .get("duration")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let track_number = item
            .get("trackNumber")
            .and_then(|v| v.as_u64())
            .map(|n| n as u32);
        let volume_number = item
            .get("volumeNumber")
            .and_then(|v| v.as_u64())
            .map(|n| n as u32);

        // Extract first artist
        let (artist_name, artist_id) = item
            .get("artists")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .map(|a| {
                let name = a
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown")
                    .to_string();
                let id = match a.get("id") {
                    Some(serde_json::Value::Number(n)) => Some(n.to_string()),
                    Some(serde_json::Value::String(s)) => Some(s.clone()),
                    _ => None,
                };
                (name, id)
            })
            .unwrap_or(("Unknown".to_string(), None));

        // Extract album
        let album = item.get("album");
        let album_name = album
            .and_then(|a| a.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let album_id = album.and_then(|a| match a.get("id") {
            Some(serde_json::Value::Number(n)) => Some(n.to_string()),
            Some(serde_json::Value::String(s)) => Some(s.clone()),
            _ => None,
        });

        // Build artwork URL from album cover UUID
        let artwork_url = album
            .and_then(|a| a.get("cover"))
            .and_then(|v| v.as_str())
            .map(|cover| {
                let cover_path = cover.replace('-', "/");
                format!(
                    "https://resources.tidal.com/images/{}/{{width}}x{{height}}.jpg",
                    cover_path
                )
            });

        tracks.push(Track {
            id,
            title,
            duration,
            track_number,
            volume_number,
            isrc: item.get("isrc").and_then(|v| v.as_str()).map(String::from),
            artist_name,
            artist_id,
            album_name,
            album_id,
            artwork_url,
            media_tags: Vec::new(),
        });
    }

    tracks
}

impl TidalClient {
    pub async fn get_favorites(&self) -> AppResult<Vec<Track>> {
        let config = self.config().read().await;
        let user_id = config.user_id.clone().ok_or(AppError::AuthRequired)?;
        let country = config.country_code.clone();
        drop(config);

        let path = format!("/userCollections/{}/relationships/tracks", user_id);
        let response = self
            .get_with_query(
                &path,
                &[
                    ("countryCode", country.as_str()),
                    (
                        "include",
                        "tracks,tracks.artists,tracks.albums,tracks.albums.coverArt",
                    ),
                ],
            )
            .await?;

        let body: serde_json::Value = response.json().await?;
        let included = body.get("included").and_then(|v| v.as_array());

        Ok(parse_tracks_from_included(included))
    }

    pub async fn toggle_favorite(&self, track_id: &str, add: bool) -> AppResult<()> {
        let config = self.config().read().await;
        let user_id = config.user_id.clone().ok_or(AppError::AuthRequired)?;
        let country = config.country_code.clone();
        drop(config);

        let path = format!("/userCollections/{}/relationships/tracks", user_id);
        let body = serde_json::json!({
            "data": [{
                "type": "tracks",
                "id": track_id
            }]
        });
        if add {
            self.post_with_query(&path, &[("countryCode", country.as_str())], &body)
                .await?;
        } else {
            self.delete_with_body(&path, &body).await?;
        }
        Ok(())
    }

    pub async fn get_recommendations(&self) -> AppResult<Vec<Track>> {
        let config = self.config().read().await;
        let country = config.country_code.clone();
        let token = config.access_token.clone();
        drop(config);

        let token = token.ok_or(AppError::AuthRequired)?;

        // Step 1: Get discovery mix IDs from v2 userRecommendations
        let response = self
            .get_with_query(
                "/userRecommendations/me",
                &[
                    ("countryCode", country.as_str()),
                    ("include", "discoveryMixes,myMixes"),
                ],
            )
            .await?;

        let body: serde_json::Value = response.json().await?;

        // Extract mix IDs from included resources
        let mut mix_ids: Vec<String> = Vec::new();
        if let Some(included) = body.get("included").and_then(|v| v.as_array()) {
            for item in included {
                if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                    mix_ids.push(id.to_string());
                }
            }
        }

        // Also check relationships.discoveryMixes.data for mix refs
        if mix_ids.is_empty() {
            if let Some(data) = body.get("data") {
                for rel_key in &["discoveryMixes", "myMixes"] {
                    if let Some(refs) = data
                        .get("relationships")
                        .and_then(|r| r.get(*rel_key))
                        .and_then(|r| r.get("data"))
                        .and_then(|d| d.as_array())
                    {
                        for r in refs {
                            if let Some(id) = r.get("id").and_then(|v| v.as_str()) {
                                mix_ids.push(id.to_string());
                            }
                        }
                    }
                }
            }
        }

        if mix_ids.is_empty() {
            log::warn!("No recommendation mixes found, falling back to favorites");
            let favorites = self.get_favorites().await?;
            return Ok(favorites.into_iter().take(20).collect());
        }

        // Step 2: Fetch tracks from the first few mixes via v1 API
        let mut all_tracks: Vec<Track> = Vec::new();
        let max_mixes = mix_ids.len().min(3);

        for mix_id in &mix_ids[..max_mixes] {
            let url = format!("https://api.tidal.com/v1/mixes/{}/items", mix_id);
            let resp = self
                .http_client()
                .get(&url)
                .bearer_auth(&token)
                .query(&[("countryCode", country.as_str()), ("limit", "25")])
                .send()
                .await;

            match resp {
                Ok(r) if r.status().is_success() => {
                    if let Ok(body) = r.json::<serde_json::Value>().await {
                        let tracks = parse_v1_mix_items(&body);
                        all_tracks.extend(tracks);
                    }
                }
                Ok(r) => {
                    log::warn!("v1 mix items request failed with status {}", r.status());
                }
                Err(e) => {
                    log::warn!("v1 mix items request failed: {}", e);
                }
            }
        }

        if all_tracks.is_empty() {
            log::warn!("No tracks from mixes, falling back to favorites");
            let favorites = self.get_favorites().await?;
            return Ok(favorites.into_iter().take(20).collect());
        }

        Ok(all_tracks)
    }

    pub async fn get_similar_tracks(&self, track_id: &str) -> AppResult<Vec<Track>> {
        let config = self.config().read().await;
        let country = config.country_code.clone();
        drop(config);

        let path = format!("/tracks/{}/relationships/similarTracks", track_id);
        let response = self
            .get_with_query(
                &path,
                &[
                    ("countryCode", country.as_str()),
                    ("include", "similarTracks,similarTracks.artists,similarTracks.albums,similarTracks.albums.coverArt"),
                ],
            )
            .await?;

        let body: serde_json::Value = response.json().await?;
        let included = body.get("included").and_then(|v| v.as_array());

        Ok(parse_tracks_from_included(included))
    }
}
