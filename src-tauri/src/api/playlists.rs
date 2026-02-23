use crate::api::client::TidalClient;
use crate::api::models::{Playlist, Track};
use crate::api::search::{get_first_relationship_id, parse_playlist, parse_track};
use crate::error::{AppError, AppResult};
use std::collections::HashMap;

impl TidalClient {
    pub async fn get_playlists(&self) -> AppResult<Vec<Playlist>> {
        let config = self.config().read().await;
        let country = config.country_code.clone();
        drop(config);

        let response = self
            .get_with_query(
                "/playlists",
                &[
                    ("countryCode", country.as_str()),
                    ("filter[owners.id]", "me"),
                    ("include", "coverArt,owners"),
                ],
            )
            .await?;

        let body: serde_json::Value = response.json().await?;
        let data = body.get("data").and_then(|v| v.as_array());
        let included = body.get("included").and_then(|v| v.as_array());

        // Build artwork map from included artworks
        let mut artwork_map: HashMap<String, String> = HashMap::new();
        if let Some(items) = included {
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
        }

        let mut playlists = Vec::new();
        if let Some(items) = data {
            for item in items {
                let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let attrs = item.get("attributes").cloned().unwrap_or_default();
                let rels = item.get("relationships");
                if let Some(mut playlist) = parse_playlist(id, &attrs) {
                    if playlist.artwork_url.is_none() {
                        playlist.artwork_url = get_first_relationship_id(rels, "coverArt")
                            .and_then(|art_id| artwork_map.get(&art_id).cloned());
                    }
                    playlist.creator_id = get_first_relationship_id(rels, "owners");
                    playlists.push(playlist);
                }
            }
        }

        Ok(playlists)
    }

    pub async fn get_playlist(&self, playlist_id: &str) -> AppResult<Playlist> {
        let config = self.config().read().await;
        let country = config.country_code.clone();
        drop(config);

        let path = format!("/playlists/{}", playlist_id);
        let response = self
            .get_with_query(
                &path,
                &[("countryCode", country.as_str()), ("include", "coverArt")],
            )
            .await?;

        let body: serde_json::Value = response.json().await?;
        let data = body.get("data");
        let included = body.get("included").and_then(|v| v.as_array());

        let id = data
            .and_then(|d| d.get("id"))
            .and_then(|v| v.as_str())
            .unwrap_or(playlist_id);
        let attrs = data
            .and_then(|d| d.get("attributes"))
            .cloned()
            .unwrap_or_default();
        let rels = data.and_then(|d| d.get("relationships"));

        let mut playlist = parse_playlist(id, &attrs)
            .ok_or_else(|| AppError::NotFound(format!("Playlist {} not found", playlist_id)))?;

        // Resolve artwork from coverArt relationship
        if playlist.artwork_url.is_none() {
            if let (Some(items), Some(art_id_val)) =
                (included, get_first_relationship_id(rels, "coverArt"))
            {
                for item in items {
                    if item.get("type").and_then(|v| v.as_str()) == Some("artworks")
                        && item.get("id").and_then(|v| v.as_str()) == Some(&art_id_val)
                    {
                        playlist.artwork_url = item
                            .get("attributes")
                            .and_then(|a| a.get("files"))
                            .and_then(|v| v.as_array())
                            .and_then(|arr| arr.last().or(arr.first()))
                            .and_then(|f| f.get("href"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        break;
                    }
                }
            }
        }

        Ok(playlist)
    }

    pub async fn get_playlist_tracks(&self, playlist_id: &str) -> AppResult<Vec<Track>> {
        let config = self.config().read().await;
        let country = config.country_code.clone();
        drop(config);

        let path = format!("/playlists/{}/relationships/items", playlist_id);
        let response = self
            .get_with_query(
                &path,
                &[
                    ("countryCode", country.as_str()),
                    (
                        "include",
                        "items,items.artists,items.albums,items.albums.coverArt",
                    ),
                ],
            )
            .await?;

        let body: serde_json::Value = response.json().await?;
        let included = body.get("included").and_then(|v| v.as_array());

        // Build lookup maps from included resources
        let mut artist_map: HashMap<String, String> = HashMap::new();
        let mut album_map: HashMap<String, (String, Option<String>)> = HashMap::new();
        let mut artwork_map: HashMap<String, String> = HashMap::new();

        if let Some(items) = included {
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
                            let artwork =
                                get_first_relationship_id(item.get("relationships"), "coverArt")
                                    .and_then(|art_id| artwork_map.get(&art_id).cloned());
                            album_map.insert(rid, (title.to_string(), artwork));
                        }
                    }
                    _ => {}
                }
            }
        }

        let mut tracks = Vec::new();
        if let Some(items) = included {
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
        }

        Ok(tracks)
    }

    pub async fn create_playlist(
        &self,
        name: &str,
        description: Option<&str>,
    ) -> AppResult<Playlist> {
        let config = self.config().read().await;
        let country = config.country_code.clone();
        drop(config);

        let body = serde_json::json!({
            "data": {
                "type": "playlists",
                "attributes": {
                    "name": name,
                    "description": description.unwrap_or("")
                }
            }
        });

        let response = self
            .post_with_query("/playlists", &[("countryCode", country.as_str())], &body)
            .await?;
        let resp_body: serde_json::Value = response.json().await?;

        let id = resp_body
            .get("data")
            .and_then(|d| d.get("id"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let attrs = resp_body
            .get("data")
            .and_then(|d| d.get("attributes"))
            .cloned()
            .unwrap_or_default();

        parse_playlist(id, &attrs)
            .ok_or_else(|| AppError::Config("Failed to parse created playlist".into()))
    }

    pub async fn add_to_playlist(&self, playlist_id: &str, track_id: &str) -> AppResult<()> {
        let config = self.config().read().await;
        let country = config.country_code.clone();
        drop(config);

        let body = serde_json::json!({
            "data": [{
                "type": "tracks",
                "id": track_id
            }]
        });

        let path = format!("/playlists/{}/relationships/items", playlist_id);
        self.post_with_query(&path, &[("countryCode", country.as_str())], &body)
            .await?;
        Ok(())
    }

    pub async fn remove_from_playlist(&self, playlist_id: &str, track_id: &str) -> AppResult<()> {
        let path = format!("/playlists/{}/relationships/items", playlist_id);
        let body = serde_json::json!({
            "data": [{
                "type": "tracks",
                "id": track_id,
                "meta": {
                    "itemId": track_id
                }
            }]
        });
        self.delete_with_body(&path, &body).await?;
        Ok(())
    }

    pub async fn delete_playlist(&self, playlist_id: &str) -> AppResult<()> {
        let path = format!("/playlists/{}", playlist_id);
        self.delete(&path).await?;
        Ok(())
    }
}
