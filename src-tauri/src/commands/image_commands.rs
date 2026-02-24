use crate::error::AppError;
use crate::AppState;
use base64::Engine;
use tauri::State;

/// Proxy an image URL through the backend to avoid CDN referer restrictions.
/// Returns a data URI (e.g. "data:image/jpeg;base64,...").
#[tauri::command]
pub async fn proxy_image(state: State<'_, AppState>, url: String) -> Result<String, AppError> {
    let response = reqwest::Client::new()
        .get(&url)
        .header("Accept", "image/jpeg,image/jpg,image/png,image/*")
        .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36")
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(AppError::Http(
            response
                .error_for_status()
                .expect_err("status was not success"),
        ));
    }

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/jpeg")
        .to_string();

    let bytes = response.bytes().await?;

    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:{};base64,{}", content_type, b64))
}
