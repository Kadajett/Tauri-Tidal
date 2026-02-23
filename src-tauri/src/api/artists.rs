use crate::api::client::TidalClient;
use crate::api::models::{Album, Artist};
use crate::api::search::{get_first_relationship_id, parse_album, parse_artist};
use crate::error::{AppError, AppResult};
use std::collections::HashMap;

impl TidalClient {
    pub async fn get_artist(&self, artist_id: &str) -> AppResult<Artist> {
        let config = self.config().read().await;
        let country = config.country_code.clone();
        drop(config);

        let path = format!("/artists/{}", artist_id);
        let response = self
            .get_with_query(
                &path,
                &[("countryCode", country.as_str()), ("include", "profileArt")],
            )
            .await?;

        let body: serde_json::Value = response.json().await?;
        let data = body.get("data");
        let id = data
            .and_then(|d| d.get("id"))
            .and_then(|v| v.as_str())
            .unwrap_or(artist_id);
        let attrs = data
            .and_then(|d| d.get("attributes"))
            .cloned()
            .unwrap_or_default();
        let rels = data.and_then(|d| d.get("relationships"));
        let included = body.get("included").and_then(|v| v.as_array());

        let mut artist = parse_artist(id, &attrs)
            .ok_or_else(|| AppError::NotFound(format!("Artist {} not found", artist_id)))?;

        // Resolve profile art from included artworks
        if artist.picture_url.is_none() {
            if let (Some(items), Some(rels)) = (included, rels) {
                if let Some(art_id) = get_first_relationship_id(Some(rels), "profileArt") {
                    for item in items {
                        if item.get("type").and_then(|v| v.as_str()) == Some("artworks")
                            && item.get("id").and_then(|v| v.as_str()) == Some(&art_id)
                        {
                            artist.picture_url = item
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
        }

        Ok(artist)
    }

    pub async fn get_artist_albums(&self, artist_id: &str) -> AppResult<Vec<Album>> {
        let config = self.config().read().await;
        let country = config.country_code.clone();
        drop(config);

        let path = format!("/artists/{}/relationships/albums", artist_id);
        let response = self
            .get_with_query(
                &path,
                &[
                    ("countryCode", country.as_str()),
                    ("include", "albums,albums.coverArt,albums.artists"),
                ],
            )
            .await?;

        let body: serde_json::Value = response.json().await?;
        let included = body.get("included").and_then(|v| v.as_array());

        // Build lookup maps
        let mut artist_map: HashMap<String, String> = HashMap::new();
        let mut artwork_map: HashMap<String, String> = HashMap::new();

        if let Some(items) = included {
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
                    "artworks" => {
                        if let Some(href) = item
                            .get("attributes")
                            .and_then(|a| a.get("files"))
                            .and_then(|v| v.as_array())
                            .and_then(|arr| arr.last().or(arr.first()))
                            .and_then(|f| f.get("href"))
                            .and_then(|v| v.as_str())
                        {
                            artwork_map.insert(rid, href.to_string());
                        }
                    }
                    _ => {}
                }
            }
        }

        let mut albums = Vec::new();
        if let Some(items) = included {
            for item in items {
                let resource_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
                if resource_type == "albums" {
                    let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("");
                    let attrs = item.get("attributes").cloned().unwrap_or_default();
                    let rels = item.get("relationships");
                    if let Some(mut album) = parse_album(id, &attrs) {
                        // Resolve artist
                        if let Some(aid) = get_first_relationship_id(rels, "artists") {
                            if let Some(name) = artist_map.get(&aid) {
                                album.artist_name = name.clone();
                                album.artist_id = Some(aid);
                            }
                        }
                        // Resolve cover art
                        if album.artwork_url.is_none() {
                            if let Some(art_id) = get_first_relationship_id(rels, "coverArt") {
                                album.artwork_url = artwork_map.get(&art_id).cloned();
                            }
                        }
                        albums.push(album);
                    }
                }
            }
        }

        Ok(albums)
    }
}
