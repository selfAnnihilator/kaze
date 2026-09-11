use serde::Serialize;
use thiserror::Error;

/// Comprehensive application error hierarchy.
#[derive(Debug, Error, Serialize)]
#[serde(tag = "type", content = "details")]
pub enum AppError {
    #[error("Playback error: {0}")]
    Playback(String),

    #[error("Library error: {0}")]
    Library(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Metadata provider error: {0}")]
    Metadata(String),

    #[error("Recommendation error: {0}")]
    Recommendation(String),

    #[error("External API error ({provider}): {message}")]
    ExternalApi { provider: String, message: String },

    #[error("Entity not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Internal(err.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
