use crate::api::client::TidalClient;
use crate::api::models::{Album, Track};
use crate::api::search::{get_first_relationship_id, parse_album, parse_tracks_from_included};
use crate::error::{AppError, AppResult};

impl TidalClient {
    pub async fn get_album(&self, album_id: &str) -> AppResult<Album> {
        let config = self.config().read().await;
        let country = config.country_code.clone();
        drop(config);

        let path = format!("/albums/{}", album_id);
        let response = self
            .get_with_query(
                &path,
                &[
                    ("countryCode", country.as_str()),
                    ("include", "artists,coverArt"),
                ],
            )
            .await?;

        let body: serde_json::Value = response.json().await?;
        let data = body.get("data");
        let id = data
            .and_then(|d| d.get("id"))
            .and_then(|v| v.as_str())
            .unwrap_or(album_id);
        let attrs = data
            .and_then(|d| d.get("attributes"))
            .cloned()
            .unwrap_or_default();
        let rels = data.and_then(|d| d.get("relationships"));
        let included = body.get("included").and_then(|v| v.as_array());

        let mut album = parse_album(id, &attrs)
            .ok_or_else(|| AppError::NotFound(format!("Album {} not found", album_id)))?;

        // Resolve artist from included resources
        if let (Some(items), Some(rels)) = (included, rels) {
            if let Some(artist_id) = get_first_relationship_id(Some(rels), "artists") {
                for item in items {
                    if item.get("type").and_then(|v| v.as_str()) == Some("artists")
                        && item.get("id").and_then(|v| v.as_str()) == Some(&artist_id)
                    {
                        if let Some(name) = item
                            .get("attributes")
                            .and_then(|a| a.get("name"))
                            .and_then(|v| v.as_str())
                        {
                            album.artist_name = name.to_string();
                            album.artist_id = Some(artist_id);
                        }
                        break;
                    }
                }
            }

            // Resolve artwork from coverArt relationship
            if album.artwork_url.is_none() {
                if let Some(art_id) = get_first_relationship_id(Some(rels), "coverArt") {
                    for item in items {
                        if item.get("type").and_then(|v| v.as_str()) == Some("artworks")
                            && item.get("id").and_then(|v| v.as_str()) == Some(&art_id)
                        {
                            album.artwork_url = item
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

        Ok(album)
    }

    pub async fn get_album_tracks(&self, album_id: &str) -> AppResult<Vec<Track>> {
        let config = self.config().read().await;
        let country = config.country_code.clone();
        drop(config);

        let path = format!("/albums/{}/relationships/items", album_id);
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

        Ok(parse_tracks_from_included(included))
    }
}
