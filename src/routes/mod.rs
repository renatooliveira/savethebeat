use axum::{Json, Router, routing::get};
use serde_json::json;

pub fn routes() -> Router {
    Router::new().route("/health", get(health))
}

/// Build Spotify OAuth routes
///
/// Requires SpotifyState to be provided via with_state
///
/// # Routes
/// - GET /spotify/connect - Initiate OAuth flow
/// - GET /spotify/callback - Handle OAuth callback
/// - GET /spotify/verify - Verify authentication and test token refresh
pub fn spotify_routes() -> Router<crate::spotify::routes::SpotifyState> {
    use crate::spotify::routes::{callback, connect, verify};

    Router::new()
        .route("/spotify/connect", get(connect))
        .route("/spotify/callback", get(callback))
        .route("/spotify/verify", get(verify))
}

async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_endpoint() {
        let app = routes();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(body["status"], "ok");
        assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
    }
}
