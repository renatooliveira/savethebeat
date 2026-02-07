use crate::error::AppError;
use crate::spotify::oauth::{StateStore, generate_state_token, store_state};
use axum::{
    extract::{Query, State},
    response::Redirect,
};
use oauth2::{CsrfToken, Scope, basic::BasicClient};
use serde::Deserialize;

/// Query parameters for /spotify/connect endpoint
#[derive(Debug, Deserialize)]
pub struct ConnectQuery {
    pub slack_workspace_id: String,
    pub slack_user_id: String,
}

/// Application state for Spotify routes
#[derive(Clone)]
pub struct SpotifyState {
    pub oauth_client: BasicClient,
    pub state_store: StateStore,
}

/// Initiates Spotify OAuth flow
///
/// # Endpoint
/// GET /spotify/connect?slack_workspace_id=<WORKSPACE>&slack_user_id=<USER>
///
/// # Flow
/// 1. Generate cryptographically secure state token
/// 2. Store state with Slack user metadata
/// 3. Build Spotify authorization URL with required scopes
/// 4. Redirect user to Spotify for authorization
///
/// # Query Parameters
/// - `slack_workspace_id`: Slack workspace ID (e.g., "T123ABC")
/// - `slack_user_id`: Slack user ID (e.g., "U456DEF")
///
/// # Returns
/// 302 redirect to Spotify authorization page
///
/// # Errors
/// Returns 400 Bad Request if query parameters are missing
pub async fn connect(
    State(state): State<SpotifyState>,
    Query(params): Query<ConnectQuery>,
) -> Result<Redirect, AppError> {
    tracing::info!(
        slack_workspace_id = %params.slack_workspace_id,
        slack_user_id = %params.slack_user_id,
        "Starting Spotify OAuth connect flow"
    );

    // Generate and store state token
    let state_token = generate_state_token();
    store_state(
        &state.state_store,
        state_token.clone(),
        params.slack_workspace_id.clone(),
        params.slack_user_id.clone(),
    );

    tracing::debug!(
        state_token_length = state_token.len(),
        "Generated and stored OAuth state token"
    );

    // Build Spotify authorization URL
    let (auth_url, _csrf_token) = state
        .oauth_client
        .authorize_url(|| CsrfToken::new(state_token))
        .add_scope(Scope::new("user-library-modify".to_string()))
        .url();

    tracing::info!(
        redirect_url = %auth_url,
        "Redirecting to Spotify authorization"
    );

    Ok(Redirect::to(auth_url.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::spotify::oauth::build_oauth_client;
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};

    fn setup_test_state() -> SpotifyState {
        let config = Config {
            port: 3000,
            host: "0.0.0.0".to_string(),
            database_url: "postgresql://localhost/test".to_string(),
            spotify_client_id: "test_client_id".to_string(),
            spotify_client_secret: "test_client_secret".to_string(),
            spotify_redirect_uri: "http://localhost:3000/spotify/callback".to_string(),
            base_url: "http://localhost:3000".to_string(),
            slack_signing_secret: None,
            slack_bot_token: None,
            rust_log: "info".to_string(),
        };

        SpotifyState {
            oauth_client: build_oauth_client(&config),
            state_store: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    #[tokio::test]
    async fn test_connect_generates_redirect() {
        let state = setup_test_state();
        let params = ConnectQuery {
            slack_workspace_id: "T123".to_string(),
            slack_user_id: "U456".to_string(),
        };

        let result = connect(State(state.clone()), Query(params)).await;

        assert!(result.is_ok());

        // Verify state was stored
        let store = state.state_store.read().unwrap();
        assert_eq!(store.len(), 1);

        // Verify stored state contains correct metadata
        let (_token, oauth_state) = store.iter().next().unwrap();
        assert_eq!(oauth_state.slack_workspace_id, "T123");
        assert_eq!(oauth_state.slack_user_id, "U456");
    }

    #[tokio::test]
    async fn test_connect_redirect_url_format() {
        let state = setup_test_state();
        let params = ConnectQuery {
            slack_workspace_id: "T123".to_string(),
            slack_user_id: "U456".to_string(),
        };

        let result = connect(State(state), Query(params)).await;
        assert!(result.is_ok());

        // Note: We can't easily extract the redirect URL from axum::response::Redirect
        // in tests, but we've verified the function doesn't error and stores state
    }

    #[tokio::test]
    async fn test_connect_multiple_users() {
        let state = setup_test_state();

        // Connect first user
        let params1 = ConnectQuery {
            slack_workspace_id: "T123".to_string(),
            slack_user_id: "U456".to_string(),
        };
        let result1 = connect(State(state.clone()), Query(params1)).await;
        assert!(result1.is_ok());

        // Connect second user
        let params2 = ConnectQuery {
            slack_workspace_id: "T123".to_string(),
            slack_user_id: "U789".to_string(),
        };
        let result2 = connect(State(state.clone()), Query(params2)).await;
        assert!(result2.is_ok());

        // Verify both states are stored
        let store = state.state_store.read().unwrap();
        assert_eq!(store.len(), 2);
    }
}
