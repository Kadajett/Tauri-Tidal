use crate::api::auth;
use crate::api::models::AuthStatus;
use crate::config::AppConfig;
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
    Ok(AuthStatus {
        authenticated: has_user_auth,
        user_id: config.user_id.clone(),
        country_code: config.country_code.clone(),
    })
}

#[tauri::command]
pub async fn login(state: State<'_, AppState>) -> Result<String, AppError> {
    let pkce = auth::PkceChallenge::generate();
    let config = state.tidal_client.config().read().await;
    let auth_url = auth::build_auth_url(&config.client_id, &pkce.challenge);
    drop(config);

    // Store verifier for later use
    *state.pkce_verifier.lock().await = Some(pkce.verifier);

    Ok(auth_url)
}

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

    let status = AuthStatus {
        authenticated: true,
        user_id: config.user_id.clone(),
        country_code: config.country_code.clone(),
    };

    Ok(status)
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
