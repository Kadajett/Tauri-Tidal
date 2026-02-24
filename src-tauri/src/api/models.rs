use serde::{Deserialize, Serialize};

// JSON:API envelope types
#[derive(Debug, Deserialize)]
pub struct JsonApiResponse<T> {
    pub data: T,
    #[serde(default)]
    pub included: Vec<JsonApiResource>,
}

#[derive(Debug, Deserialize)]
pub struct JsonApiListResponse<T> {
    pub data: Vec<T>,
    #[serde(default)]
    pub included: Vec<JsonApiResource>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JsonApiResource {
    pub id: String,
    #[serde(rename = "type")]
    pub resource_type: String,
    #[serde(default)]
    pub attributes: serde_json::Value,
    #[serde(default)]
    pub relationships: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JsonApiResourceRef {
    pub id: String,
    #[serde(rename = "type")]
    pub resource_type: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JsonApiRelationship {
    pub data: Option<JsonApiRelationshipData>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum JsonApiRelationshipData {
    Single(JsonApiResourceRef),
    Multiple(Vec<JsonApiResourceRef>),
}

// Tidal domain types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: String,
    pub title: String,
    pub duration: f64,
    pub track_number: Option<u32>,
    pub volume_number: Option<u32>,
    pub isrc: Option<String>,
    pub artist_name: String,
    pub artist_id: Option<String>,
    pub album_name: String,
    pub album_id: Option<String>,
    pub artwork_url: Option<String>,
    pub media_tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Album {
    pub id: String,
    pub title: String,
    pub artist_name: String,
    pub artist_id: Option<String>,
    pub duration: Option<f64>,
    pub number_of_tracks: Option<u32>,
    pub number_of_volumes: Option<u32>,
    pub release_date: Option<String>,
    pub artwork_url: Option<String>,
    pub media_tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Artist {
    pub id: String,
    pub name: String,
    pub picture_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub duration: Option<f64>,
    pub number_of_items: Option<u32>,
    pub playlist_type: Option<String>,
    pub artwork_url: Option<String>,
    pub creator_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FavoritesPage {
    pub tracks: Vec<Track>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationSection {
    pub title: String,
    pub subtitle: Option<String>,
    pub tracks: Vec<Track>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResults {
    pub tracks: Vec<Track>,
    pub albums: Vec<Album>,
    pub artists: Vec<Artist>,
    pub playlists: Vec<Playlist>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Lyrics {
    pub track_id: String,
    pub lyrics: Option<String>,
    pub subtitles: Option<String>,
}

// Auth types
#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: u64,
    pub token_type: String,
    pub user_id: Option<serde_json::Value>,
}

/// Response from the device authorization endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceAuthResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: Option<String>,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthStatus {
    pub authenticated: bool,
    pub user_id: Option<String>,
    pub display_name: Option<String>,
    pub country_code: String,
}

/// Resolve `{width}` and `{height}` placeholders in an artwork URL.
/// Returns the URL with placeholders replaced by the given dimensions.
pub fn resolve_artwork_url(url: &str, width: u32, height: u32) -> String {
    url.replace("{width}", &width.to_string())
        .replace("{height}", &height.to_string())
}

// Artwork helpers
impl Track {
    pub fn artwork_url_sized(&self, width: u32, height: u32) -> Option<String> {
        self.artwork_url
            .as_ref()
            .map(|url| resolve_artwork_url(url, width, height))
    }

    /// Resolve artwork URL placeholders in-place with a default size.
    pub fn resolve_artwork(&mut self) {
        if let Some(ref url) = self.artwork_url {
            if url.contains("{width}") || url.contains("{height}") {
                self.artwork_url = Some(resolve_artwork_url(url, 480, 480));
            }
        }
    }
}

impl Album {
    pub fn artwork_url_sized(&self, width: u32, height: u32) -> Option<String> {
        self.artwork_url
            .as_ref()
            .map(|url| resolve_artwork_url(url, width, height))
    }

    pub fn resolve_artwork(&mut self) {
        if let Some(ref url) = self.artwork_url {
            if url.contains("{width}") || url.contains("{height}") {
                self.artwork_url = Some(resolve_artwork_url(url, 480, 480));
            }
        }
    }
}

impl Artist {
    pub fn resolve_artwork(&mut self) {
        if let Some(ref url) = self.picture_url {
            if url.contains("{width}") || url.contains("{height}") {
                self.picture_url = Some(resolve_artwork_url(url, 480, 480));
            }
        }
    }
}

impl Playlist {
    pub fn resolve_artwork(&mut self) {
        if let Some(ref url) = self.artwork_url {
            if url.contains("{width}") || url.contains("{height}") {
                self.artwork_url = Some(resolve_artwork_url(url, 480, 480));
            }
        }
    }
}

impl SearchResults {
    /// Resolve all artwork URL placeholders in search results.
    pub fn resolve_all_artwork(&mut self) {
        for track in &mut self.tracks {
            track.resolve_artwork();
        }
        for album in &mut self.albums {
            album.resolve_artwork();
        }
        for artist in &mut self.artists {
            artist.resolve_artwork();
        }
        for playlist in &mut self.playlists {
            playlist.resolve_artwork();
        }
    }
}
