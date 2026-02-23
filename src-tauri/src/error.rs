use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Audio error: {0}")]
    Audio(String),

    #[error("Decode error: {0}")]
    Decode(String),

    #[error("Authentication required")]
    AuthRequired,

    #[error("Token expired")]
    TokenExpired,

    #[error("Tidal API error: {status} - {message}")]
    TidalApi { status: u16, message: String },

    #[error("Config error: {0}")]
    Config(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("AppError", 2)?;
        state.serialize_field("kind", &self.kind())?;
        state.serialize_field("message", &self.to_string())?;
        state.end()
    }
}

impl AppError {
    fn kind(&self) -> &str {
        match self {
            AppError::Http(_) => "http",
            AppError::Json(_) => "json",
            AppError::Audio(_) => "audio",
            AppError::Decode(_) => "decode",
            AppError::AuthRequired => "auth_required",
            AppError::TokenExpired => "token_expired",
            AppError::TidalApi { .. } => "tidal_api",
            AppError::Config(_) => "config",
            AppError::NotFound(_) => "not_found",
            AppError::Io(_) => "io",
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;
