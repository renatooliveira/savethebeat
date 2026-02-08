use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use thiserror::Error;

/// Application-specific errors with HTTP status code mappings
#[derive(Debug, Error)]
pub enum AppError {
    #[error("OAuth state not found")]
    OAuthStateNotFound,

    #[error("OAuth state expired")]
    OAuthStateExpired,

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Spotify API error: {0}")]
    SpotifyApi(String),

    #[error("Invalid request: {0}")]
    BadRequest(String),

    #[error("Slack signature invalid: {0}")]
    SignatureInvalid(String),

    #[error("Slack signature missing")]
    SignatureMissing,

    #[error("Slack signature expired: {0}")]
    SignatureExpired(String),

    #[error("Slack API error: {0}")]
    SlackApi(String),

    #[error("Internal server error: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            AppError::OAuthStateNotFound => {
                tracing::warn!("OAuth state not found");
                (StatusCode::BAD_REQUEST, "Invalid or expired OAuth state")
            }
            AppError::OAuthStateExpired => {
                tracing::warn!("OAuth state expired");
                (
                    StatusCode::BAD_REQUEST,
                    "OAuth state expired, please try again",
                )
            }
            AppError::Database(err) => {
                tracing::error!("Database error: {:?}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
            AppError::SpotifyApi(msg) => {
                tracing::error!("Spotify API error: {}", msg);
                (StatusCode::BAD_GATEWAY, "Spotify API error")
            }
            AppError::BadRequest(msg) => {
                tracing::warn!("Bad request: {}", msg);
                (StatusCode::BAD_REQUEST, msg.as_str())
            }
            AppError::SignatureInvalid(msg) => {
                tracing::warn!("Invalid Slack signature: {}", msg);
                (StatusCode::UNAUTHORIZED, "Invalid signature")
            }
            AppError::SignatureMissing => {
                tracing::warn!("Slack signature headers missing");
                (StatusCode::UNAUTHORIZED, "Missing signature headers")
            }
            AppError::SignatureExpired(msg) => {
                tracing::warn!("Slack signature expired: {}", msg);
                (StatusCode::UNAUTHORIZED, "Signature expired")
            }
            AppError::SlackApi(msg) => {
                tracing::error!("Slack API error: {}", msg);
                (StatusCode::BAD_GATEWAY, "Slack API error")
            }
            AppError::Internal(err) => {
                tracing::error!("Internal error: {:?}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
        };

        (status, Json(json!({ "error": error_message }))).into_response()
    }
}
