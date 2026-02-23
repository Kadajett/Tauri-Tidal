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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthStatus {
    pub authenticated: bool,
    pub user_id: Option<String>,
    pub country_code: String,
}

// Artwork helpers
impl Track {
    pub fn artwork_url_sized(&self, width: u32, height: u32) -> Option<String> {
        self.artwork_url.as_ref().map(|url| {
            url.replace("{width}", &width.to_string())
                .replace("{height}", &height.to_string())
        })
    }
}

impl Album {
    pub fn artwork_url_sized(&self, width: u32, height: u32) -> Option<String> {
        self.artwork_url.as_ref().map(|url| {
            url.replace("{width}", &width.to_string())
                .replace("{height}", &height.to_string())
        })
    }
}
