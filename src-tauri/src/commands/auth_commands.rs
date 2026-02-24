use crate::api::auth;
use crate::api::models::{AuthStatus, DeviceAuthResponse};
use crate::error::AppError;
use tauri::State;

use crate::AppState;

#[tauri::command]
pub async fn check_auth_status(state: State<'_, AppState>) -> Result<AuthStatus, AppError> {
    let config = state.tidal_client.config().read().await;
    // Only report authenticated when we have a user-level token (user_id present),
    // not just a client credentials token (catalog-only access)
    let has_user_auth =
        config.user_id.is_some() && config.access_token.is_some() && !config.is_token_expired();
    let display_name = config.display_name.clone();
    let user_id = config.user_id.clone();
    let country_code = config.country_code.clone();
    drop(config);

    // If authenticated but no display name cached, fetch it from the API
    let display_name = if has_user_auth && display_name.is_none() {
        match state.tidal_client.get_user_profile().await {
            Ok((username, first_name, last_name)) => {
                // Build display name: prefer "firstName lastName", fall back to username
                let name = match (&first_name, &last_name) {
                    (Some(f), Some(l)) if !f.is_empty() => Some(format!("{} {}", f, l)),
                    (Some(f), _) if !f.is_empty() => Some(f.clone()),
                    _ => username.clone(),
                };
                // Cache the display name in config
                if let Some(ref n) = name {
                    let mut config = state.tidal_client.config().write().await;
                    config.display_name = Some(n.clone());
                    let _ = config.save();
                }
                name
            }
            Err(e) => {
                log::warn!("Failed to fetch user profile: {}", e);
                None
            }
        }
    } else {
        display_name
    };

    Ok(AuthStatus {
        authenticated: has_user_auth,
        user_id,
        display_name,
        country_code,
    })
}

/// Device code flow step 1: get a device code + user code.
/// Returns the device auth response so the frontend can show the code and open the URL.
#[tauri::command]
pub async fn login(state: State<'_, AppState>) -> Result<DeviceAuthResponse, AppError> {
    let config = state.tidal_client.config().read().await;
    let client_id = config.client_id.clone();
    drop(config);

    let device_auth =
        auth::request_device_code(state.tidal_client.http_client(), &client_id).await?;

    log::info!(
        "Device auth: user_code={}, verification_uri={}",
        device_auth.user_code,
        device_auth.verification_uri
    );

    // Store the device code for polling
    *state.pkce_verifier.lock().await = Some(device_auth.device_code.clone());

    Ok(device_auth)
}

/// Device code flow step 2: poll for authorization.
/// Call this repeatedly from the frontend until it returns an AuthStatus with authenticated=true.
#[tauri::command]
pub async fn poll_login(state: State<'_, AppState>) -> Result<AuthStatus, AppError> {
    let device_code = state
        .pkce_verifier
        .lock()
        .await
        .clone()
        .ok_or(AppError::AuthRequired)?;

    let config = state.tidal_client.config().read().await;
    let client_id = config.client_id.clone();
    drop(config);

    let result =
        auth::poll_device_token(state.tidal_client.http_client(), &client_id, &device_code).await?;

    match result {
        Some(token_response) => {
            // Clear the stored device code
            *state.pkce_verifier.lock().await = None;

            let mut config = state.tidal_client.config().write().await;
            config.access_token = Some(token_response.access_token);
            if let Some(rt) = token_response.refresh_token {
                config.refresh_token = Some(rt);
            }
            config.expires_at = Some(
                chrono::Utc::now() + chrono::Duration::seconds(token_response.expires_in as i64),
            );
            if let Some(user_id) = &token_response.user_id {
                config.user_id = Some(user_id.to_string());
            }
            config.save()?;
            let user_id = config.user_id.clone();
            let country_code = config.country_code.clone();
            drop(config);

            // Fetch user profile to get display name
            let display_name = match state.tidal_client.get_user_profile().await {
                Ok((username, first_name, last_name)) => {
                    let name = match (&first_name, &last_name) {
                        (Some(f), Some(l)) if !f.is_empty() => Some(format!("{} {}", f, l)),
                        (Some(f), _) if !f.is_empty() => Some(f.clone()),
                        _ => username.clone(),
                    };
                    if let Some(ref n) = name {
                        let mut config = state.tidal_client.config().write().await;
                        config.display_name = Some(n.clone());
                        let _ = config.save();
                    }
                    name
                }
                Err(e) => {
                    log::warn!("Failed to fetch user profile after login: {}", e);
                    None
                }
            };

            Ok(AuthStatus {
                authenticated: true,
                user_id,
                display_name,
                country_code,
            })
        }
        None => {
            // Still pending, user hasn't authorized yet
            Ok(AuthStatus {
                authenticated: false,
                user_id: None,
                display_name: None,
                country_code: "US".into(),
            })
        }
    }
}

/// Legacy PKCE callback handler (kept for compatibility, may not work with all client IDs)
#[tauri::command]
pub async fn handle_auth_callback(
    state: State<'_, AppState>,
    code: String,
) -> Result<AuthStatus, AppError> {
    let verifier = state
        .pkce_verifier
        .lock()
        .await
        .take()
        .ok_or(AppError::AuthRequired)?;

    let token_response = auth::exchange_code(
        state.tidal_client.http_client(),
        state.tidal_client.config(),
        &code,
        &verifier,
    )
    .await?;

    let mut config = state.tidal_client.config().write().await;
    config.access_token = Some(token_response.access_token);
    if let Some(rt) = token_response.refresh_token {
        config.refresh_token = Some(rt);
    }
    config.expires_at =
        Some(chrono::Utc::now() + chrono::Duration::seconds(token_response.expires_in as i64));
    if let Some(user_id) = &token_response.user_id {
        config.user_id = Some(user_id.to_string());
    }
    config.save()?;
    let user_id = config.user_id.clone();
    let country_code = config.country_code.clone();
    drop(config);

    Ok(AuthStatus {
        authenticated: true,
        user_id,
        display_name: None,
        country_code,
    })
}

#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> Result<(), AppError> {
    // Stop any active playback
    let mut player = state.audio_player.write().await;
    player.stop();
    drop(player);

    // Clear user auth fields from config, keep client_id/secret/country/quality
    let mut config = state.tidal_client.config().write().await;
    config.access_token = None;
    config.refresh_token = None;
    config.expires_at = None;
    config.user_id = None;
    config.display_name = None;
    config.save()?;

    Ok(())
}

#[tauri::command]
pub async fn init_client_credentials(state: State<'_, AppState>) -> Result<(), AppError> {
    let config = state.tidal_client.config().read().await;
    let client_id = config.client_id.clone();
    let client_secret = config.client_secret.clone();
    drop(config);

    let token_response = auth::client_credentials_token(
        state.tidal_client.http_client(),
        &client_id,
        &client_secret,
    )
    .await?;

    let mut config = state.tidal_client.config().write().await;
    config.access_token = Some(token_response.access_token);
    config.expires_at =
        Some(chrono::Utc::now() + chrono::Duration::seconds(token_response.expires_in as i64));
    config.save()?;

    Ok(())
}
