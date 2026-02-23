use crate::api::client::TidalClient;
use crate::api::models::{Album, Artist, Playlist, SearchResults, Track};
use crate::error::AppResult;
use std::collections::HashMap;

impl TidalClient {
    pub async fn search(&self, query: &str, _limit: u32) -> AppResult<SearchResults> {
        let config = self.config().read().await;
        let country = config.country_code.clone();
        drop(config);

        // Tidal v2 API: search query is the path parameter (resource identifier)
        let encoded_query = urlencoding::encode(query);
        let path = format!("/searchResults/{}", encoded_query);

        // Request nested includes so the included array contains:
        // - tracks + their artists/albums (via dot-notation)
        // - albums + their artists/coverArt
        // - artists + their profileArt
        // - playlists + their coverArt
        // If the API doesn't support dot-notation, it will still return
        // first-level includes and we fall back to batch fetch.
        let response = self
            .get_with_query(
                &path,
                &[
                    (
                        "include",
                        "tracks,tracks.artists,tracks.albums,albums,albums.artists,albums.coverArt,artists,artists.profileArt,playlists,playlists.coverArt",
                    ),
                    ("countryCode", &country),
                ],
            )
            .await?;

        let body: serde_json::Value = response.json().await?;
        log::info!(
            "Search response top-level keys: {:?}",
            body.as_object().map(|o| o.keys().collect::<Vec<_>>())
        );
        if let Some(included) = body.get("included").and_then(|v| v.as_array()) {
            log::info!("Search included count: {}", included.len());
            // Log resource type counts for debugging
            let mut type_counts: HashMap<&str, usize> = HashMap::new();
            for item in included {
                let t = item.get("type").and_then(|v| v.as_str()).unwrap_or("?");
                *type_counts.entry(t).or_default() += 1;
            }
            log::info!("Search included types: {:?}", type_counts);
        } else {
            log::warn!("Search response has no 'included' array");
            log::debug!(
                "Full search response: {}",
                serde_json::to_string_pretty(&body).unwrap_or_default()
            );
        }

        let mut results = parse_search_results(&body);

        // Check if tracks have unresolved artists (dot-notation might not be supported)
        let unresolved: Vec<String> = results
            .tracks
            .iter()
            .filter(|t| t.artist_name == "Unknown Artist")
            .map(|t| t.id.clone())
            .collect();

        if !unresolved.is_empty() && !unresolved.iter().all(|id| id.is_empty()) {
            log::info!(
                "Batch-fetching {} tracks with unresolved artists",
                unresolved.len()
            );
            // Batch fetch using GET /tracks?filter[id]=...&include=artists,albums
            let ids_param = unresolved.join(",");
            match self
                .get_with_query(
                    "/tracks",
                    &[
                        ("filter[id]", &ids_param),
                        ("include", "artists,albums"),
                        ("countryCode", &country),
                    ],
                )
                .await
            {
                Ok(response) => {
                    let batch_body: serde_json::Value = response.json().await?;
                    let enriched = parse_tracks_batch(&batch_body);
                    // Replace tracks with enriched versions
                    for enriched_track in enriched {
                        if let Some(existing) = results
                            .tracks
                            .iter_mut()
                            .find(|t| t.id == enriched_track.id)
                        {
                            *existing = enriched_track;
                        }
                    }
                }
                Err(e) => {
                    log::warn!("Batch track fetch failed: {}", e);
                }
            }
        }

        log::info!(
            "Parsed search results: {} tracks, {} albums, {} artists, {} playlists",
            results.tracks.len(),
            results.albums.len(),
            results.artists.len(),
            results.playlists.len()
        );
        Ok(results)
    }

    pub async fn search_suggestions(&self, query: &str) -> AppResult<Vec<String>> {
        let config = self.config().read().await;
        let country = config.country_code.clone();
        drop(config);

        let encoded_query = urlencoding::encode(query);
        let path = format!("/searchSuggestions/{}", encoded_query);
        let response = self
            .get_with_query(&path, &[("countryCode", &country)])
            .await?;

        let body: serde_json::Value = response.json().await?;

        // searchSuggestions returns attributes.suggestions array with query strings
        let mut suggestions = Vec::new();
        if let Some(data) = body.get("data") {
            if let Some(attrs) = data.get("attributes") {
                if let Some(suggestion_list) = attrs.get("suggestions").and_then(|v| v.as_array()) {
                    for s in suggestion_list.iter().take(5) {
                        if let Some(query_str) = s.get("query").and_then(|v| v.as_str()) {
                            suggestions.push(query_str.to_string());
                        }
                    }
                }
            }
        }

        // Fall back to search results if suggestions endpoint returns nothing
        if suggestions.is_empty() {
            let search_path = format!("/searchResults/{}", encoded_query);
            let response = self
                .get_with_query(
                    &search_path,
                    &[("include", "tracks,artists"), ("countryCode", &country)],
                )
                .await?;
            let body: serde_json::Value = response.json().await?;
            let results = parse_search_results(&body);
            suggestions = results
                .tracks
                .iter()
                .take(5)
                .map(|t| format!("{} - {}", t.title, t.artist_name))
                .collect();
        }

        Ok(suggestions)
    }
}

/// Parse ISO 8601 duration string (e.g., "PT2M58S") to seconds as f64.
/// Handles hours (H), minutes (M), and seconds (S).
pub fn parse_iso8601_duration(duration: &str) -> f64 {
    let mut seconds = 0.0;
    let mut num_buf = String::new();

    for ch in duration.chars() {
        match ch {
            'P' | 'T' => {
                num_buf.clear();
            }
            '0'..='9' | '.' => {
                num_buf.push(ch);
            }
            'H' => {
                if let Ok(h) = num_buf.parse::<f64>() {
                    seconds += h * 3600.0;
                }
                num_buf.clear();
            }
            'M' => {
                if let Ok(m) = num_buf.parse::<f64>() {
                    seconds += m * 60.0;
                }
                num_buf.clear();
            }
            'S' => {
                if let Ok(s) = num_buf.parse::<f64>() {
                    seconds += s;
                }
                num_buf.clear();
            }
            _ => {}
        }
    }

    seconds
}

/// Extract the first relationship ID from a JSON:API relationships object.
/// Handles both to-one (single object) and to-many (array) relationships.
pub fn get_first_relationship_id(
    rels: Option<&serde_json::Value>,
    rel_name: &str,
) -> Option<String> {
    let data = rels?.get(rel_name)?.get("data")?;
    if let Some(arr) = data.as_array() {
        arr.first()?.get("id")?.as_str().map(|s| s.to_string())
    } else {
        data.get("id")?.as_str().map(|s| s.to_string())
    }
}

/// Extract artwork URL from an artworks resource's attributes.files array.
fn extract_artwork_href(attrs: &serde_json::Value) -> Option<String> {
    attrs
        .get("files")
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            // Prefer larger images (last in the files array tends to be largest)
            arr.last()
                .or(arr.first())
                .and_then(|f| f.get("href").and_then(|v| v.as_str()))
                .map(|s| s.to_string())
        })
}

/// Try to extract an image URL from various possible attribute locations.
/// Falls back through multiple patterns since the API response format
/// can vary between endpoints.
fn extract_image_url(attrs: &serde_json::Value) -> Option<String> {
    // Try artworks files (v2 artworks resource format)
    if let Some(url) = extract_artwork_href(attrs) {
        return Some(url);
    }

    // Try imageLinks array
    if let Some(url) = attrs
        .get("imageLinks")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("href"))
        .and_then(|v| v.as_str())
    {
        return Some(url.to_string());
    }

    // Try image array
    if let Some(url) = attrs
        .get("image")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("href"))
        .and_then(|v| v.as_str())
    {
        return Some(url.to_string());
    }

    // Try direct imageUrl string
    if let Some(url) = attrs.get("imageUrl").and_then(|v| v.as_str()) {
        return Some(url.to_string());
    }

    None
}

/// Build comprehensive lookup maps from a JSON:API included array.
/// Returns (artist_map, album_map, artwork_map).
fn build_lookup_maps(
    included: &[serde_json::Value],
) -> (
    HashMap<String, String>,                   // artist_id -> name
    HashMap<String, (String, Option<String>)>, // album_id -> (title, artwork_url)
    HashMap<String, String>,                   // artwork_id -> href URL
) {
    let mut artist_map: HashMap<String, String> = HashMap::new();
    let mut album_map: HashMap<String, (String, Option<String>)> = HashMap::new();
    let mut artwork_map: HashMap<String, String> = HashMap::new();

    // First: extract all artwork URLs from artworks resources
    for item in included {
        if item.get("type").and_then(|v| v.as_str()) == Some("artworks") {
            let id = item
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if let Some(attrs) = item.get("attributes") {
                if let Some(href) = extract_artwork_href(attrs) {
                    artwork_map.insert(id, href);
                }
            }
        }
    }

    // Second: build artist and album maps
    for item in included {
        let resource_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let id = item
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let attrs = item.get("attributes");
        let rels = item.get("relationships");

        match resource_type {
            "artists" => {
                if let Some(name) = attrs.and_then(|a| a.get("name")).and_then(|v| v.as_str()) {
                    artist_map.insert(id, name.to_string());
                }
            }
            "albums" => {
                if let Some(title) = attrs.and_then(|a| a.get("title")).and_then(|v| v.as_str()) {
                    // Try to get artwork from coverArt relationship -> artwork_map
                    let artwork = get_first_relationship_id(rels, "coverArt")
                        .and_then(|art_id| artwork_map.get(&art_id).cloned())
                        .or_else(|| extract_image_url(&attrs.cloned().unwrap_or_default()));
                    album_map.insert(id, (title.to_string(), artwork));
                }
            }
            _ => {}
        }
    }

    log::debug!(
        "Lookup maps built: {} artists, {} albums, {} artworks",
        artist_map.len(),
        album_map.len(),
        artwork_map.len()
    );

    (artist_map, album_map, artwork_map)
}

fn parse_search_results(body: &serde_json::Value) -> SearchResults {
    let included = body.get("included").and_then(|v| v.as_array());

    let empty_vec = Vec::new();
    let items = included.unwrap_or(&empty_vec);

    let (artist_map, album_map, artwork_map) = build_lookup_maps(items);

    // Parse all resources with relationship resolution
    let mut tracks = Vec::new();
    let mut albums = Vec::new();
    let mut artists = Vec::new();
    let mut playlists = Vec::new();

    for item in items {
        let resource_type = item.get("type").and_then(|v| v.as_str()).unwrap_or("");
        let id = item
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let attrs = item.get("attributes").cloned().unwrap_or_default();
        let rels = item.get("relationships");

        match resource_type {
            "tracks" => {
                if let Some(mut track) = parse_track(&id, &attrs) {
                    // Resolve artist name from relationships -> included artists
                    if let Some(artist_id) = get_first_relationship_id(rels, "artists") {
                        if let Some(name) = artist_map.get(&artist_id) {
                            track.artist_name = name.clone();
                            track.artist_id = Some(artist_id);
                        }
                    }
                    // Resolve album name and artwork from relationships -> included albums
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
            "albums" => {
                if let Some(mut album) = parse_album(&id, &attrs) {
                    // Resolve artist name from relationships
                    if let Some(artist_id) = get_first_relationship_id(rels, "artists") {
                        if let Some(name) = artist_map.get(&artist_id) {
                            album.artist_name = name.clone();
                            album.artist_id = Some(artist_id);
                        }
                    }
                    // Resolve artwork from coverArt relationship
                    if album.artwork_url.is_none() {
                        album.artwork_url = get_first_relationship_id(rels, "coverArt")
                            .and_then(|art_id| artwork_map.get(&art_id).cloned());
                    }
                    albums.push(album);
                }
            }
            "artists" => {
                if let Some(mut artist) = parse_artist(&id, &attrs) {
                    // Resolve picture from profileArt relationship
                    if artist.picture_url.is_none() {
                        artist.picture_url = get_first_relationship_id(rels, "profileArt")
                            .and_then(|art_id| artwork_map.get(&art_id).cloned());
                    }
                    artists.push(artist);
                }
            }
            "playlists" => {
                if let Some(mut playlist) = parse_playlist(&id, &attrs) {
                    // Resolve artwork from coverArt relationship
                    if playlist.artwork_url.is_none() {
                        playlist.artwork_url = get_first_relationship_id(rels, "coverArt")
                            .and_then(|art_id| artwork_map.get(&art_id).cloned());
                    }
                    playlists.push(playlist);
                }
            }
            _ => {}
        }
    }

    SearchResults {
        tracks,
        albums,
        artists,
        playlists,
    }
}

/// Parse a batch response from GET /tracks?filter[id]=... with include=artists,albums.
/// Returns fully resolved Track objects.
fn parse_tracks_batch(body: &serde_json::Value) -> Vec<Track> {
    let data = body.get("data").and_then(|v| v.as_array());
    let included = body.get("included").and_then(|v| v.as_array());

    let empty_vec = Vec::new();
    let inc_items = included.unwrap_or(&empty_vec);
    let (artist_map, album_map, _artwork_map) = build_lookup_maps(inc_items);

    let mut tracks = Vec::new();
    if let Some(items) = data {
        for item in items {
            let id = item
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let attrs = item.get("attributes").cloned().unwrap_or_default();
            let rels = item.get("relationships");

            if let Some(mut track) = parse_track(&id, &attrs) {
                if let Some(artist_id) = get_first_relationship_id(rels, "artists") {
                    if let Some(name) = artist_map.get(&artist_id) {
                        track.artist_name = name.clone();
                        track.artist_id = Some(artist_id);
                    }
                }
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

/// Parse a track from its attributes. Artist/album names default to "Unknown"
/// and should be resolved from relationships by the caller when possible.
pub fn parse_track(id: &str, attrs: &serde_json::Value) -> Option<Track> {
    let title = attrs.get("title")?.as_str()?.to_string();

    // Duration: Tidal v2 uses ISO 8601 strings (e.g., "PT2M58S"),
    // but also handle f64 for backward compatibility
    let duration = attrs
        .get("duration")
        .and_then(|v| {
            v.as_f64()
                .or_else(|| v.as_str().map(parse_iso8601_duration))
        })
        .unwrap_or(0.0);

    // These default to "Unknown" when called without relationship context.
    // The caller overrides them from relationships after calling this function.
    let artist_name = attrs
        .get("artistName")
        .or_else(|| attrs.get("artist"))
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown Artist")
        .to_string();

    let album_name = attrs
        .get("albumName")
        .or_else(|| attrs.get("album"))
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown Album")
        .to_string();

    let artwork_url = extract_image_url(attrs);

    let media_tags = attrs
        .get("mediaTags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    Some(Track {
        id: id.to_string(),
        title,
        duration,
        track_number: attrs
            .get("trackNumber")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32),
        volume_number: attrs
            .get("volumeNumber")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32),
        isrc: attrs
            .get("isrc")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        artist_name,
        artist_id: None,
        album_name,
        album_id: None,
        artwork_url,
        media_tags,
    })
}

/// Resolve a track's artist/album from a JSON:API included array.
/// Used by get_track and similar single-resource endpoints.
pub fn resolve_track_relationships(
    track: &mut Track,
    rels: Option<&serde_json::Value>,
    included: Option<&Vec<serde_json::Value>>,
) {
    let items = match included {
        Some(items) => items,
        None => return,
    };

    // Build artwork map first
    let mut artwork_map: HashMap<String, String> = HashMap::new();
    for item in items {
        if item.get("type").and_then(|v| v.as_str()) == Some("artworks") {
            let id = item
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if let Some(attrs) = item.get("attributes") {
                if let Some(href) = extract_artwork_href(attrs) {
                    artwork_map.insert(id, href);
                }
            }
        }
    }

    // Resolve artist
    if let Some(artist_id) = get_first_relationship_id(rels, "artists") {
        for item in items {
            if item.get("type").and_then(|v| v.as_str()) == Some("artists")
                && item.get("id").and_then(|v| v.as_str()) == Some(&artist_id)
            {
                if let Some(name) = item
                    .get("attributes")
                    .and_then(|a| a.get("name"))
                    .and_then(|v| v.as_str())
                {
                    track.artist_name = name.to_string();
                    track.artist_id = Some(artist_id);
                }
                break;
            }
        }
    }

    // Resolve album
    if let Some(album_id) = get_first_relationship_id(rels, "albums") {
        for item in items {
            if item.get("type").and_then(|v| v.as_str()) == Some("albums")
                && item.get("id").and_then(|v| v.as_str()) == Some(&album_id)
            {
                let item_attrs = item.get("attributes");
                let item_rels = item.get("relationships");
                if let Some(title) = item_attrs
                    .and_then(|a| a.get("title"))
                    .and_then(|v| v.as_str())
                {
                    track.album_name = title.to_string();
                    track.album_id = Some(album_id);
                }
                if track.artwork_url.is_none() {
                    // Try coverArt relationship -> artwork_map
                    track.artwork_url = get_first_relationship_id(item_rels, "coverArt")
                        .and_then(|art_id| artwork_map.get(&art_id).cloned())
                        .or_else(|| extract_image_url(&item_attrs.cloned().unwrap_or_default()));
                }
                break;
            }
        }
    }
}

pub fn parse_album(id: &str, attrs: &serde_json::Value) -> Option<Album> {
    let title = attrs.get("title")?.as_str()?.to_string();

    let artist_name = attrs
        .get("artistName")
        .or_else(|| attrs.get("artist"))
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown Artist")
        .to_string();

    let artwork_url = extract_image_url(attrs);

    // Duration: handle ISO 8601 string or f64
    let duration = attrs.get("duration").and_then(|v| {
        v.as_f64()
            .or_else(|| v.as_str().map(parse_iso8601_duration))
    });

    let media_tags = attrs
        .get("mediaTags")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    Some(Album {
        id: id.to_string(),
        title,
        artist_name,
        artist_id: None,
        duration,
        number_of_tracks: attrs
            .get("numberOfTracks")
            .or_else(|| attrs.get("numberOfItems"))
            .and_then(|v| v.as_u64())
            .map(|v| v as u32),
        number_of_volumes: attrs
            .get("numberOfVolumes")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32),
        release_date: attrs
            .get("releaseDate")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        artwork_url,
        media_tags,
    })
}

pub fn parse_artist(id: &str, attrs: &serde_json::Value) -> Option<Artist> {
    let name = attrs.get("name")?.as_str()?.to_string();

    let picture_url = extract_image_url(attrs);

    Some(Artist {
        id: id.to_string(),
        name,
        picture_url,
    })
}

pub fn parse_playlist(id: &str, attrs: &serde_json::Value) -> Option<Playlist> {
    let name = attrs.get("name")?.as_str()?.to_string();

    let artwork_url = extract_image_url(attrs);

    // Duration: handle ISO 8601 string or f64
    let duration = attrs.get("duration").and_then(|v| {
        v.as_f64()
            .or_else(|| v.as_str().map(parse_iso8601_duration))
    });

    Some(Playlist {
        id: id.to_string(),
        name,
        description: attrs
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        duration,
        number_of_items: attrs
            .get("numberOfItems")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32),
        playlist_type: attrs
            .get("playlistType")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        artwork_url,
        creator_id: None,
    })
}
