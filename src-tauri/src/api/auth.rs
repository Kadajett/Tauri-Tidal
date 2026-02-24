use crate::api::models::TokenResponse;
use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::Rng;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tokio::sync::RwLock;

const AUTH_URL: &str = "https://login.tidal.com/authorize";
const TOKEN_URL: &str = "https://auth.tidal.com/v1/oauth2/token";
const REDIRECT_URI: &str = "mactidalplayer://auth/callback";

pub struct PkceChallenge {
    pub verifier: String,
    pub challenge: String,
}

impl PkceChallenge {
    pub fn generate() -> Self {
        let mut rng = rand::thread_rng();
        let verifier_bytes: Vec<u8> = (0..32).map(|_| rng.gen::<u8>()).collect();
        let verifier = URL_SAFE_NO_PAD.encode(&verifier_bytes);

        let digest = Sha256::digest(verifier.as_bytes());
        let challenge = URL_SAFE_NO_PAD.encode(digest);

        Self {
            verifier,
            challenge,
        }
    }
}

pub fn build_auth_url(client_id: &str, code_challenge: &str) -> String {
    let scopes = [
        "user.read",
        "collection.read",
        "collection.write",
        "playlists.read",
        "playlists.write",
        "search.read",
        "search.write",
        "recommendations.read",
        "entitlements.read",
        "playback",
    ]
    .join(" ");

    let url = format!(
        "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&code_challenge_method=S256&code_challenge={}",
        AUTH_URL,
        client_id,
        urlencoding::encode(REDIRECT_URI),
        urlencoding::encode(&scopes),
        code_challenge
    );
    log::info!("Auth URL: {}", url);
    url
}

pub async fn exchange_code(
    http: &reqwest::Client,
    config: &Arc<RwLock<AppConfig>>,
    code: &str,
    code_verifier: &str,
) -> AppResult<TokenResponse> {
    let config_read = config.read().await;
    let client_id = config_read.client_id.clone();
    drop(config_read);

    // Per Tidal docs: authorization code exchange uses PKCE (code_verifier),
    // not client_secret. Only client_id, code, redirect_uri, code_verifier are needed.
    let params = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", REDIRECT_URI),
        ("client_id", &client_id),
        ("code_verifier", code_verifier),
    ];

    let response = http.post(TOKEN_URL).form(&params).send().await?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::TidalApi {
            status: 401,
            message: format!("Token exchange failed: {}", body),
        });
    }

    let token: TokenResponse = response.json().await?;
    Ok(token)
}

/// Refresh an expired user token using the refresh_token grant.
pub async fn refresh_user_token(
    http: &reqwest::Client,
    client_id: &str,
    refresh_token: &str,
) -> AppResult<TokenResponse> {
    let params = [
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", client_id),
    ];

    let response = http.post(TOKEN_URL).form(&params).send().await?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::TidalApi {
            status: 401,
            message: format!("Token refresh failed: {}", body),
        });
    }

    let token: TokenResponse = response.json().await?;
    Ok(token)
}

const DEVICE_AUTH_URL: &str = "https://auth.tidal.com/v1/oauth2/device_authorization";

/// Step 1 of device code flow: request a device code + user code.
pub async fn request_device_code(
    http: &reqwest::Client,
    client_id: &str,
) -> AppResult<crate::api::models::DeviceAuthResponse> {
    let params = [("client_id", client_id), ("scope", "r_usr w_usr")];

    let response = http.post(DEVICE_AUTH_URL).form(&params).send().await?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::TidalApi {
            status: 401,
            message: format!("Device auth request failed: {}", body),
        });
    }

    let resp: crate::api::models::DeviceAuthResponse = response.json().await?;
    Ok(resp)
}

/// Step 2 of device code flow: poll for the token (call repeatedly with interval).
/// Returns Ok(Some(token)) when authorized, Ok(None) when still pending.
pub async fn poll_device_token(
    http: &reqwest::Client,
    client_id: &str,
    device_code: &str,
) -> AppResult<Option<TokenResponse>> {
    let params = [
        ("client_id", client_id),
        ("device_code", device_code),
        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ("scope", "r_usr w_usr"),
    ];

    let response = http.post(TOKEN_URL).form(&params).send().await?;
    let status = response.status();

    if status.is_success() {
        let token: TokenResponse = response.json().await?;
        return Ok(Some(token));
    }

    let body = response.text().await.unwrap_or_default();

    // "authorization_pending" means user hasn't approved yet
    if body.contains("authorization_pending") {
        return Ok(None);
    }

    // "expired_token" means the device code expired
    if body.contains("expired_token") {
        return Err(AppError::TidalApi {
            status: status.as_u16(),
            message: "Device code expired. Please try logging in again.".into(),
        });
    }

    Err(AppError::TidalApi {
        status: status.as_u16(),
        message: format!("Device token poll failed: {}", body),
    })
}

pub async fn client_credentials_token(
    http: &reqwest::Client,
    client_id: &str,
    client_secret: &str,
) -> AppResult<TokenResponse> {
    use base64::engine::general_purpose::STANDARD;

    let credentials = format!("{}:{}", client_id, client_secret);
    let b64_creds = STANDARD.encode(credentials.as_bytes());

    let params = [("grant_type", "client_credentials")];

    let response = http
        .post(TOKEN_URL)
        .header("Authorization", format!("Basic {}", b64_creds))
        .form(&params)
        .send()
        .await?;

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::TidalApi {
            status: 401,
            message: format!("Client credentials auth failed: {}", body),
        });
    }

    let token: TokenResponse = response.json().await?;
    Ok(token)
}
