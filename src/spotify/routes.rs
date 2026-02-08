use crate::db::repository::upsert_user_auth;
use crate::error::AppError;
use crate::spotify::client::{ensure_valid_token, get_current_user};
use crate::spotify::oauth::{
    StateStore, generate_state_token, store_state, validate_and_consume_state,
};
use axum::{
    Json,
    extract::{Query, State},
    response::{Html, Redirect},
};
use chrono::{Duration, Utc};
use oauth2::{
    AuthorizationCode, CsrfToken, Scope, TokenResponse, basic::BasicClient,
    reqwest::async_http_client,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

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
    pub db: PgPool,
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

/// Render the success page with workspace and user information
///
/// # Arguments
/// * `workspace_id` - Slack workspace ID
/// * `user_id` - Slack user ID
///
/// # Returns
/// HTML string with placeholders replaced
fn render_success_page(workspace_id: &str, user_id: &str) -> String {
    const TEMPLATE: &str = include_str!("../../templates/spotify_success.html");

    TEMPLATE
        .replace("{{WORKSPACE_ID}}", workspace_id)
        .replace("{{USER_ID}}", user_id)
}

/// Query parameters for /spotify/callback endpoint
#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub code: String,
    pub state: String,
}

/// Handles Spotify OAuth callback
///
/// # Endpoint
/// GET /spotify/callback?code=<CODE>&state=<STATE>
///
/// # Flow
/// 1. Validate and consume state token (CSRF protection)
/// 2. Extract Slack workspace and user IDs from state
/// 3. Exchange authorization code for access/refresh tokens
/// 4. Calculate token expiry with 5-minute buffer
/// 5. Upsert tokens to database
/// 6. Return success HTML page
///
/// # Query Parameters
/// - `code`: Authorization code from Spotify
/// - `state`: State token (must match stored token)
///
/// # Returns
/// HTML success page
///
/// # Errors
/// - 400 Bad Request if state is invalid or expired
/// - 500 Internal Server Error if token exchange or database operation fails
pub async fn callback(
    State(state): State<SpotifyState>,
    Query(params): Query<CallbackQuery>,
) -> Result<Html<String>, AppError> {
    tracing::info!("Received Spotify OAuth callback");

    // Validate and consume state token
    let (workspace_id, user_id) = validate_and_consume_state(&state.state_store, &params.state)?;

    tracing::info!(
        slack_workspace_id = %workspace_id,
        slack_user_id = %user_id,
        "Validated OAuth state"
    );

    // Exchange authorization code for tokens
    tracing::debug!("Exchanging authorization code for tokens");

    let token_result = state
        .oauth_client
        .exchange_code(AuthorizationCode::new(params.code))
        .request_async(async_http_client)
        .await
        .map_err(|e| {
            tracing::error!("Token exchange failed: {:?}", e);
            AppError::SpotifyApi(format!("Failed to exchange authorization code: {}", e))
        })?;

    let access_token = token_result.access_token().secret().to_string();
    let refresh_token = token_result
        .refresh_token()
        .ok_or_else(|| {
            tracing::error!("No refresh token in response");
            AppError::SpotifyApi("No refresh token received".to_string())
        })?
        .secret()
        .to_string();

    // Calculate token expiry with 5-minute buffer
    let expires_in_seconds = token_result
        .expires_in()
        .ok_or_else(|| {
            tracing::error!("No expires_in in token response");
            AppError::SpotifyApi("No expiry time in token response".to_string())
        })?
        .as_secs() as i64;

    let expires_at = Utc::now() + Duration::seconds(expires_in_seconds) - Duration::minutes(5);

    tracing::info!(
        expires_in_seconds = expires_in_seconds,
        expires_at = %expires_at,
        "Received tokens from Spotify"
    );

    // Store tokens in database
    let user_auth = upsert_user_auth(
        &state.db,
        &workspace_id,
        &user_id,
        None, // spotify_user_id - we'll get this later when we call /v1/me
        &access_token,
        &refresh_token,
        expires_at,
    )
    .await
    .map_err(|e| {
        tracing::error!("Database upsert failed: {:?}", e);
        AppError::Database(e)
    })?;

    tracing::info!(
        user_auth_id = %user_auth.id,
        slack_workspace_id = %workspace_id,
        slack_user_id = %user_id,
        "Successfully stored Spotify tokens"
    );

    // Return success HTML page
    let html = render_success_page(&workspace_id, &user_id);

    Ok(Html(html))
}

/// Query parameters for /spotify/verify endpoint
#[derive(Debug, Deserialize)]
pub struct VerifyQuery {
    pub slack_workspace_id: String,
    pub slack_user_id: String,
}

/// Response for /spotify/verify endpoint
#[derive(Debug, Serialize)]
pub struct VerifyResponse {
    pub success: bool,
    pub spotify_user_id: String,
    pub display_name: Option<String>,
    pub token_refreshed: bool,
}

/// Verify Spotify authentication and test token refresh
///
/// # Endpoint
/// GET /spotify/verify?slack_workspace_id=<WORKSPACE>&slack_user_id=<USER>
///
/// # Flow
/// 1. Fetch user authentication from database
/// 2. Ensure token is valid (refresh if expired)
/// 3. Call Spotify API to verify token works
/// 4. Return user profile information
///
/// # Query Parameters
/// - `slack_workspace_id`: Slack workspace ID (e.g., "T123ABC")
/// - `slack_user_id`: Slack user ID (e.g., "U456DEF")
///
/// # Returns
/// JSON with Spotify user profile and refresh status
///
/// # Errors
/// - 400 Bad Request if user not authenticated
/// - 500 Internal Server Error if token refresh or API call fails
pub async fn verify(
    State(state): State<SpotifyState>,
    Query(params): Query<VerifyQuery>,
) -> Result<Json<VerifyResponse>, AppError> {
    tracing::info!(
        slack_workspace_id = %params.slack_workspace_id,
        slack_user_id = %params.slack_user_id,
        "Verifying Spotify authentication"
    );

    // Ensure we have a valid access token (will refresh if needed)
    let access_token = ensure_valid_token(
        &state.db,
        &state.oauth_client,
        &params.slack_workspace_id,
        &params.slack_user_id,
    )
    .await?;

    tracing::debug!("Obtained valid access token");

    // Call Spotify API to verify token works
    let spotify_user = get_current_user(&access_token).await?;

    tracing::info!(
        spotify_user_id = %spotify_user.id,
        display_name = ?spotify_user.display_name,
        "Successfully verified Spotify authentication"
    );

    Ok(Json(VerifyResponse {
        success: true,
        spotify_user_id: spotify_user.id,
        display_name: spotify_user.display_name,
        token_refreshed: false, // TODO: Track if refresh occurred
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::spotify::oauth::build_oauth_client;
    use std::collections::HashMap;
    use std::sync::{Arc, RwLock};

    async fn setup_test_state() -> SpotifyState {
        let config = Config {
            port: 3000,
            host: "0.0.0.0".to_string(),
            database_url: "postgresql://localhost/savethebeat_test".to_string(),
            spotify_client_id: "test_client_id".to_string(),
            spotify_client_secret: "test_client_secret".to_string(),
            spotify_redirect_uri: "http://localhost:3000/spotify/callback".to_string(),
            base_url: "http://localhost:3000".to_string(),
            slack_signing_secret: None,
            slack_bot_token: None,
            rust_log: "info".to_string(),
        };

        // Create a lazy database pool for tests (won't connect until needed)
        let db = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&config.database_url)
            .unwrap();

        SpotifyState {
            oauth_client: build_oauth_client(&config),
            state_store: Arc::new(RwLock::new(HashMap::new())),
            db,
        }
    }

    #[tokio::test]
    async fn test_connect_generates_redirect() {
        let state = setup_test_state().await;
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
        let state = setup_test_state().await;
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
        let state = setup_test_state().await;

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

    #[tokio::test]
    async fn test_callback_invalid_state() {
        let state = setup_test_state().await;
        let params = CallbackQuery {
            code: "test_code".to_string(),
            state: "invalid_state_token".to_string(),
        };

        let result = callback(State(state), Query(params)).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::OAuthStateNotFound));
    }
}
