use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use std::sync::Arc;
use tokio::sync::RwLock;

const BASE_URL: &str = "https://openapi.tidal.com/v2";
const JSONAPI_CONTENT_TYPE: &str = "application/vnd.api+json";

pub struct TidalClient {
    http: reqwest::Client,
    config: Arc<RwLock<AppConfig>>,
}

impl TidalClient {
    pub fn new(config: Arc<RwLock<AppConfig>>) -> AppResult<Self> {
        let http = reqwest::Client::builder()
            .user_agent("MacTidal/0.1.0")
            .build()?;

        Ok(Self { http, config })
    }

    pub fn config(&self) -> &Arc<RwLock<AppConfig>> {
        &self.config
    }

    async fn auth_headers(&self) -> AppResult<HeaderMap> {
        let config = self.config.read().await;
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static(JSONAPI_CONTENT_TYPE));

        if let Some(token) = &config.access_token {
            let auth_value = format!("Bearer {}", token);
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&auth_value).map_err(|e| AppError::Config(e.to_string()))?,
            );
        }

        Ok(headers)
    }

    async fn client_credentials_headers(&self) -> AppResult<HeaderMap> {
        let config = self.config.read().await;
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static(JSONAPI_CONTENT_TYPE));

        // For catalog-only access, use client credentials
        if let Some(token) = &config.access_token {
            let auth_value = format!("Bearer {}", token);
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&auth_value).map_err(|e| AppError::Config(e.to_string()))?,
            );
        }

        Ok(headers)
    }

    pub async fn get(&self, path: &str) -> AppResult<reqwest::Response> {
        let url = format!("{}{}", BASE_URL, path);
        let headers = self.auth_headers().await?;

        let response = self.http.get(&url).headers(headers).send().await?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            // Try refreshing the token
            self.refresh_token().await?;
            let headers = self.auth_headers().await?;
            let response = self.http.get(&url).headers(headers).send().await?;
            self.check_response(response).await
        } else {
            self.check_response(response).await
        }
    }

    pub async fn get_with_query(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> AppResult<reqwest::Response> {
        let url = format!("{}{}", BASE_URL, path);
        let headers = self.auth_headers().await?;

        let response = self
            .http
            .get(&url)
            .headers(headers)
            .query(query)
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            self.refresh_token().await?;
            let headers = self.auth_headers().await?;
            let response = self
                .http
                .get(&url)
                .headers(headers)
                .query(query)
                .send()
                .await?;
            self.check_response(response).await
        } else {
            self.check_response(response).await
        }
    }

    pub async fn post(&self, path: &str, body: &serde_json::Value) -> AppResult<reqwest::Response> {
        let url = format!("{}{}", BASE_URL, path);
        let mut headers = self.auth_headers().await?;
        headers.insert(CONTENT_TYPE, HeaderValue::from_static(JSONAPI_CONTENT_TYPE));

        let response = self
            .http
            .post(&url)
            .headers(headers)
            .json(body)
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            self.refresh_token().await?;
            let mut headers = self.auth_headers().await?;
            headers.insert(CONTENT_TYPE, HeaderValue::from_static(JSONAPI_CONTENT_TYPE));
            let response = self
                .http
                .post(&url)
                .headers(headers)
                .json(body)
                .send()
                .await?;
            self.check_response(response).await
        } else {
            self.check_response(response).await
        }
    }

    pub async fn post_with_query(
        &self,
        path: &str,
        query: &[(&str, &str)],
        body: &serde_json::Value,
    ) -> AppResult<reqwest::Response> {
        let url = format!("{}{}", BASE_URL, path);
        let mut headers = self.auth_headers().await?;
        headers.insert(CONTENT_TYPE, HeaderValue::from_static(JSONAPI_CONTENT_TYPE));

        let response = self
            .http
            .post(&url)
            .headers(headers)
            .query(query)
            .json(body)
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            self.refresh_token().await?;
            let mut headers = self.auth_headers().await?;
            headers.insert(CONTENT_TYPE, HeaderValue::from_static(JSONAPI_CONTENT_TYPE));
            let response = self
                .http
                .post(&url)
                .headers(headers)
                .query(query)
                .json(body)
                .send()
                .await?;
            self.check_response(response).await
        } else {
            self.check_response(response).await
        }
    }

    pub async fn delete(&self, path: &str) -> AppResult<reqwest::Response> {
        let url = format!("{}{}", BASE_URL, path);
        let headers = self.auth_headers().await?;

        let response = self.http.delete(&url).headers(headers).send().await?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            self.refresh_token().await?;
            let headers = self.auth_headers().await?;
            let response = self.http.delete(&url).headers(headers).send().await?;
            self.check_response(response).await
        } else {
            self.check_response(response).await
        }
    }

    pub async fn delete_with_body(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> AppResult<reqwest::Response> {
        let url = format!("{}{}", BASE_URL, path);
        let mut headers = self.auth_headers().await?;
        headers.insert(CONTENT_TYPE, HeaderValue::from_static(JSONAPI_CONTENT_TYPE));

        let response = self
            .http
            .delete(&url)
            .headers(headers)
            .json(body)
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            self.refresh_token().await?;
            let mut headers = self.auth_headers().await?;
            headers.insert(CONTENT_TYPE, HeaderValue::from_static(JSONAPI_CONTENT_TYPE));
            let response = self
                .http
                .delete(&url)
                .headers(headers)
                .json(body)
                .send()
                .await?;
            self.check_response(response).await
        } else {
            self.check_response(response).await
        }
    }

    pub async fn get_stream_url(&self, url: &str) -> AppResult<reqwest::Response> {
        let headers = self.client_credentials_headers().await?;
        let response = self.http.get(url).headers(headers).send().await?;
        self.check_response(response).await
    }

    async fn check_response(&self, response: reqwest::Response) -> AppResult<reqwest::Response> {
        let status = response.status();
        if status.is_success() {
            Ok(response)
        } else if status == reqwest::StatusCode::UNAUTHORIZED {
            Err(AppError::AuthRequired)
        } else if status == reqwest::StatusCode::NOT_FOUND {
            Err(AppError::NotFound("Resource not found".into()))
        } else {
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".into());
            Err(AppError::TidalApi {
                status: status.as_u16(),
                message,
            })
        }
    }

    async fn refresh_token(&self) -> AppResult<()> {
        let mut config = self.config.write().await;

        let refresh_token = config
            .refresh_token
            .as_ref()
            .ok_or(AppError::AuthRequired)?
            .clone();

        // Per Tidal docs: refresh token flow only needs grant_type + refresh_token
        let params = [
            ("grant_type", "refresh_token"),
            ("refresh_token", &refresh_token),
        ];

        let response = self
            .http
            .post("https://auth.tidal.com/v1/oauth2/token")
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AppError::TokenExpired);
        }

        let token_response: crate::api::models::TokenResponse = response.json().await?;

        config.access_token = Some(token_response.access_token);
        if let Some(rt) = token_response.refresh_token {
            config.refresh_token = Some(rt);
        }
        config.expires_at =
            Some(chrono::Utc::now() + chrono::Duration::seconds(token_response.expires_in as i64));
        config.save()?;

        Ok(())
    }

    pub fn http_client(&self) -> &reqwest::Client {
        &self.http
    }
}
