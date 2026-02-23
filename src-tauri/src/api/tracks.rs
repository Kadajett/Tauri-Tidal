use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use crate::api::client::TidalClient;
use crate::api::models::Track;
use crate::api::search::{parse_track, resolve_track_relationships};
use crate::error::{AppError, AppResult};

/// v1 API base URL for playback endpoints
const V1_BASE_URL: &str = "https://api.tidal.com/v1";

impl TidalClient {
    pub async fn get_track(&self, track_id: &str) -> AppResult<Track> {
        let config = self.config().read().await;
        let country = config.country_code.clone();
        drop(config);

        let path = format!("/tracks/{}", track_id);
        let response = self
            .get_with_query(
                &path,
                &[
                    ("countryCode", country.as_str()),
                    ("include", "artists,albums,albums.coverArt"),
                ],
            )
            .await?;

        let body: serde_json::Value = response.json().await?;
        let data = body.get("data");
        let id = data
            .and_then(|d| d.get("id"))
            .and_then(|v| v.as_str())
            .unwrap_or(track_id);
        let attrs = data
            .and_then(|d| d.get("attributes"))
            .cloned()
            .unwrap_or_default();
        let rels = data.and_then(|d| d.get("relationships"));
        let included = body.get("included").and_then(|v| v.as_array());

        let mut track = parse_track(id, &attrs)
            .ok_or_else(|| AppError::NotFound(format!("Track {} not found", track_id)))?;

        resolve_track_relationships(&mut track, rels, included);

        Ok(track)
    }

    /// Fetch playback manifest for a track.
    ///
    /// Strategy:
    /// 1. Try the v2 trackManifests endpoint with uriScheme=DATA (as the official SDK does)
    /// 2. Fall back to the v1 /tracks/{id}/playbackinfo endpoint
    pub async fn get_track_manifest(&self, track_id: &str) -> AppResult<TrackManifestData> {
        // Try v2 first
        match self.get_track_manifest_v2(track_id).await {
            Ok(data) => return Ok(data),
            Err(e) => {
                log::info!("v2 trackManifests failed for {}: {}, trying v1", track_id, e);
            }
        }

        // Fall back to v1
        self.get_track_manifest_v1(track_id).await
    }

    /// v2 API: GET /trackManifests/{id} with uriScheme=DATA
    /// Returns a data URL containing the manifest (DASH XML or HLS).
    /// The SDK uses this path for browser/Shaka playback.
    async fn get_track_manifest_v2(&self, track_id: &str) -> AppResult<TrackManifestData> {
        let config = self.config().read().await;
        let quality = config.audio_quality.clone();
        drop(config);

        let formats = match quality.as_str() {
            "HI_RES" | "HI_RES_LOSSLESS" => "FLAC_HIRES,FLAC,AACLC,HEAACV1",
            "LOSSLESS" => "FLAC,AACLC,HEAACV1",
            _ => "AACLC,HEAACV1",
        };

        let path = format!("/trackManifests/{}", track_id);
        let response = self
            .get_with_query(
                &path,
                &[
                    ("manifestType", "MPEG_DASH"),
                    ("formats", formats),
                    ("uriScheme", "DATA"),
                    ("usage", "PLAYBACK"),
                    ("adaptive", "false"),
                ],
            )
            .await?;

        let body: serde_json::Value = response.json().await?;
        log::debug!(
            "v2 trackManifests response keys: {:?}",
            body.as_object().map(|o| o.keys().collect::<Vec<_>>())
        );

        let attrs = body
            .get("data")
            .and_then(|d| d.get("attributes"))
            .ok_or_else(|| AppError::NotFound("No manifest data in v2 response".into()))?;

        let data_uri = attrs
            .get("uri")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::NotFound("No URI in v2 manifest".into()))?;

        let codec_from_formats = attrs
            .get("formats")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_str())
            .unwrap_or("AACLC");

        // Parse the data URL: data:{mime};base64,{content}
        parse_data_url_manifest(data_uri, codec_from_formats)
    }

    /// v1 API: GET /tracks/{id}/playbackinfo
    /// Returns BTS/EMU manifest with direct streaming URLs.
    /// Used as fallback and for native player scenarios.
    async fn get_track_manifest_v1(&self, track_id: &str) -> AppResult<TrackManifestData> {
        let config = self.config().read().await;
        let quality = config.audio_quality.clone();
        let token = config.access_token.clone();
        let client_id = config.client_id.clone();
        drop(config);

        let token = token.ok_or(AppError::AuthRequired)?;

        let audio_quality = match quality.as_str() {
            "HI_RES" | "HI_RES_LOSSLESS" => "HI_RES_LOSSLESS",
            "LOSSLESS" => "LOSSLESS",
            "HIGH" => "HIGH",
            _ => "HIGH",
        };

        let url = format!("{}/tracks/{}/playbackinfo", V1_BASE_URL, track_id);
        log::info!("Fetching v1 playback info: {} quality={}", url, audio_quality);

        let response = self
            .http_client()
            .get(&url)
            .bearer_auth(&token)
            .header("x-tidal-token", &client_id)
            .query(&[
                ("playbackmode", "STREAM"),
                ("assetpresentation", "FULL"),
                ("audioquality", audio_quality),
            ])
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            if status == reqwest::StatusCode::UNAUTHORIZED {
                return Err(AppError::AuthRequired);
            }
            let message = response.text().await.unwrap_or_default();
            log::error!("v1 playback info failed ({}): {}", status, message);
            return Err(AppError::TidalApi {
                status: status.as_u16(),
                message,
            });
        }

        let body: serde_json::Value = response.json().await?;
        log::info!(
            "v1 playback info: manifestMimeType={}, audioQuality={}, audioMode={}",
            body.get("manifestMimeType").and_then(|v| v.as_str()).unwrap_or("?"),
            body.get("audioQuality").and_then(|v| v.as_str()).unwrap_or("?"),
            body.get("audioMode").and_then(|v| v.as_str()).unwrap_or("?"),
        );

        let manifest_mime = body
            .get("manifestMimeType")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let manifest_b64 = body
            .get("manifest")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AppError::NotFound("No manifest in v1 playback info".into()))?;

        let manifest_bytes = STANDARD
            .decode(manifest_b64)
            .map_err(|e| AppError::Decode(format!("Base64 decode failed: {}", e)))?;
        let manifest_str = String::from_utf8(manifest_bytes)
            .map_err(|e| AppError::Decode(format!("UTF-8 decode failed: {}", e)))?;

        log::info!("v1 manifest decoded: mime={}, content_len={}", manifest_mime, manifest_str.len());

        let audio_quality_str = body
            .get("audioQuality")
            .and_then(|v| v.as_str())
            .unwrap_or("HIGH");

        if manifest_mime == "application/vnd.tidal.bts" {
            let bts: serde_json::Value = serde_json::from_str(&manifest_str)?;
            log::info!(
                "BTS manifest: codecs={}, mimeType={}, encryptionType={}, urls_count={}",
                bts.get("codecs").and_then(|v| v.as_str()).unwrap_or("?"),
                bts.get("mimeType").and_then(|v| v.as_str()).unwrap_or("?"),
                bts.get("encryptionType").and_then(|v| v.as_str()).unwrap_or("NONE"),
                bts.get("urls").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
            );

            let encryption = bts
                .get("encryptionType")
                .and_then(|v| v.as_str())
                .unwrap_or("NONE");
            if encryption != "NONE" {
                log::warn!("Track is DRM-encrypted ({}), playback may fail", encryption);
            }

            let uri = bts
                .get("urls")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::NotFound("No URL in BTS manifest".into()))?
                .to_string();

            let codec = bts
                .get("codecs")
                .and_then(|v| v.as_str())
                .unwrap_or(audio_quality_str)
                .to_string();

            log::info!("Using streaming URL: {}... codec={}", &uri[..uri.len().min(80)], codec);
            Ok(TrackManifestData { uri, codec })
        } else if manifest_mime == "application/vnd.tidal.emu" {
            // EMU manifest: similar to BTS but simpler
            let emu: serde_json::Value = serde_json::from_str(&manifest_str)?;
            let uri = emu
                .get("urls")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::NotFound("No URL in EMU manifest".into()))?
                .to_string();

            let codec = audio_quality_str.to_string();
            log::info!("EMU streaming URL: {}... codec={}", &uri[..uri.len().min(80)], codec);
            Ok(TrackManifestData { uri, codec })
        } else if manifest_mime == "application/dash+xml" {
            let uri = extract_dash_base_url(&manifest_str)
                .ok_or_else(|| AppError::Decode("Could not extract URL from DASH manifest".into()))?;
            let codec = audio_quality_str.to_string();
            log::info!("DASH streaming URL: {}... codec={}", &uri[..uri.len().min(80)], codec);
            Ok(TrackManifestData { uri, codec })
        } else {
            log::error!("Unsupported manifest type: {}", manifest_mime);
            log::debug!("Manifest content: {}", &manifest_str[..manifest_str.len().min(500)]);
            Err(AppError::Decode(format!(
                "Unsupported manifest type: {}",
                manifest_mime
            )))
        }
    }

    pub async fn get_streaming_url(&self, track_id: &str) -> AppResult<String> {
        let manifest = self.get_track_manifest(track_id).await?;
        Ok(manifest.uri)
    }
}

/// Manifest data containing a direct streaming URL and codec info.
pub struct TrackManifestData {
    pub uri: String,
    pub codec: String,
}

/// Parse a data URL (data:{mime};base64,{content}) into a TrackManifestData.
/// The v2 API returns manifests in this format when uriScheme=DATA.
fn parse_data_url_manifest(data_uri: &str, fallback_codec: &str) -> AppResult<TrackManifestData> {
    // Parse data:{mime};base64,{content}
    let (mime, b64_content) = if let Some(rest) = data_uri.strip_prefix("data:") {
        if let Some((mime_part, b64_part)) = rest.split_once(";base64,") {
            (mime_part, b64_part)
        } else {
            return Err(AppError::Decode("Data URL missing ;base64, separator".into()));
        }
    } else if data_uri.starts_with("https://") {
        // Direct HTTPS URL, not a data URL: use it directly
        log::info!("v2 returned direct HTTPS URL instead of data URL");
        return Ok(TrackManifestData {
            uri: data_uri.to_string(),
            codec: fallback_codec.to_string(),
        });
    } else {
        return Err(AppError::Decode("Could not parse data URL from v2 response".into()));
    };

    let manifest_bytes = STANDARD
        .decode(b64_content)
        .map_err(|e| AppError::Decode(format!("Base64 decode of data URL failed: {}", e)))?;
    let manifest_str = String::from_utf8(manifest_bytes)
        .map_err(|e| AppError::Decode(format!("UTF-8 decode of data URL failed: {}", e)))?;

    log::info!("v2 data URL: mime={}, content_len={}", mime, manifest_str.len());

    match mime {
        "application/vnd.tidal.bts" => {
            let bts: serde_json::Value = serde_json::from_str(&manifest_str)?;
            let uri = bts
                .get("urls")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str())
                .ok_or_else(|| AppError::NotFound("No URL in v2 BTS manifest".into()))?
                .to_string();
            let codec = bts
                .get("codecs")
                .and_then(|v| v.as_str())
                .unwrap_or(fallback_codec)
                .to_string();
            Ok(TrackManifestData { uri, codec })
        }
        "application/dash+xml" => {
            // DASH: extract BaseURL from MPD XML
            let uri = extract_dash_base_url(&manifest_str)
                .ok_or_else(|| AppError::Decode("Could not extract BaseURL from DASH MPD".into()))?;
            // Try to extract codec from DASH Representation
            let codec = extract_dash_codec(&manifest_str)
                .unwrap_or_else(|| fallback_codec.to_string());
            log::info!("v2 DASH: uri={}..., codec={}", &uri[..uri.len().min(80)], codec);
            Ok(TrackManifestData { uri, codec })
        }
        "application/vnd.apple.mpegurl" => {
            // HLS: extract first segment URL
            let uri = extract_hls_url(&manifest_str)
                .ok_or_else(|| AppError::Decode("Could not extract URL from HLS manifest".into()))?;
            let codec = fallback_codec.to_string();
            Ok(TrackManifestData { uri, codec })
        }
        _ => Err(AppError::Decode(format!(
            "Unsupported data URL mime type: {}",
            mime
        ))),
    }
}

/// Extract the first BaseURL from a DASH MPD XML manifest.
fn extract_dash_base_url(mpd_xml: &str) -> Option<String> {
    let start_tag = "<BaseURL>";
    let end_tag = "</BaseURL>";
    let start = mpd_xml.find(start_tag)? + start_tag.len();
    let end = mpd_xml[start..].find(end_tag)? + start;
    Some(mpd_xml[start..end].trim().to_string())
}

/// Extract codec from a DASH Representation element.
fn extract_dash_codec(mpd_xml: &str) -> Option<String> {
    // Look for codecs="..." in a Representation element
    let codecs_start = mpd_xml.find("codecs=\"")? + 8;
    let codecs_end = mpd_xml[codecs_start..].find('"')? + codecs_start;
    Some(mpd_xml[codecs_start..codecs_end].to_string())
}

/// Extract a stream URL from an HLS playlist.
fn extract_hls_url(hls: &str) -> Option<String> {
    for line in hls.lines() {
        let line = line.trim();
        if !line.is_empty() && !line.starts_with('#') {
            return Some(line.to_string());
        }
    }
    None
}
