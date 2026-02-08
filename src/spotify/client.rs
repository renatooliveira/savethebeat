use crate::db::models::UserAuth;
use crate::db::repository::{get_user_auth, update_tokens};
use crate::error::AppError;
use chrono::{Duration, Utc};
use oauth2::{RefreshToken, TokenResponse, basic::BasicClient, reqwest::async_http_client};
use sqlx::PgPool;

/// Refresh an expired Spotify access token
///
/// Exchanges the refresh token for a new access token using the OAuth2 client.
/// Updates the database with the new token and expiry time.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `oauth_client` - Configured OAuth2 client for Spotify
/// * `user_auth` - User authentication record containing refresh token
///
/// # Returns
/// The new access token as a String
///
/// # Errors
/// Returns error if:
/// - Token refresh request fails (network, invalid refresh token)
/// - Database update fails
/// - Response missing required fields
pub async fn refresh_access_token(
    pool: &PgPool,
    oauth_client: &BasicClient,
    user_auth: &UserAuth,
) -> Result<String, AppError> {
    tracing::info!(
        user_auth_id = %user_auth.id,
        slack_workspace_id = %user_auth.slack_workspace_id,
        slack_user_id = %user_auth.slack_user_id,
        "Refreshing Spotify access token"
    );

    // Exchange refresh token for new access token
    let refresh_token = RefreshToken::new(user_auth.refresh_token.clone());

    let token_result = oauth_client
        .exchange_refresh_token(&refresh_token)
        .request_async(async_http_client)
        .await
        .map_err(|e| {
            tracing::error!(
                user_auth_id = %user_auth.id,
                error = ?e,
                "Token refresh request failed"
            );
            AppError::SpotifyApi(format!("Failed to refresh access token: {}", e))
        })?;

    let new_access_token = token_result.access_token().secret().to_string();

    // Get new refresh token (Spotify may rotate it)
    let new_refresh_token = token_result
        .refresh_token()
        .map(|t| t.secret().to_string())
        .unwrap_or_else(|| user_auth.refresh_token.clone()); // Keep old if not rotated

    // Calculate new expiry time with 5-minute buffer
    let expires_in_seconds = token_result
        .expires_in()
        .ok_or_else(|| {
            tracing::error!("No expires_in in token refresh response");
            AppError::SpotifyApi("No expiry time in token refresh response".to_string())
        })?
        .as_secs() as i64;

    let new_expires_at = Utc::now() + Duration::seconds(expires_in_seconds) - Duration::minutes(5);

    tracing::debug!(
        user_auth_id = %user_auth.id,
        expires_in_seconds = expires_in_seconds,
        new_expires_at = %new_expires_at,
        refresh_token_rotated = token_result.refresh_token().is_some(),
        "Received new tokens from Spotify"
    );

    // Update database with new tokens
    update_tokens(
        pool,
        user_auth.id,
        &new_access_token,
        &new_refresh_token,
        new_expires_at,
    )
    .await
    .map_err(|e| {
        tracing::error!(
            user_auth_id = %user_auth.id,
            error = ?e,
            "Failed to update tokens in database"
        );
        AppError::Database(e)
    })?;

    tracing::info!(
        user_auth_id = %user_auth.id,
        slack_workspace_id = %user_auth.slack_workspace_id,
        slack_user_id = %user_auth.slack_user_id,
        "Successfully refreshed and stored access token"
    );

    Ok(new_access_token)
}

/// Ensure a valid access token, refreshing if necessary
///
/// Checks if the current access token is still valid (not expired).
/// If expired or close to expiry, refreshes the token automatically.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `oauth_client` - Configured OAuth2 client for Spotify
/// * `workspace_id` - Slack workspace ID
/// * `user_id` - Slack user ID
///
/// # Returns
/// A valid access token as a String
///
/// # Errors
/// Returns error if:
/// - User not found in database
/// - Token refresh fails
/// - Database operations fail
pub async fn ensure_valid_token(
    pool: &PgPool,
    oauth_client: &BasicClient,
    workspace_id: &str,
    user_id: &str,
) -> Result<String, AppError> {
    // Fetch user authentication record
    let user_auth = get_user_auth(pool, workspace_id, user_id)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| {
            tracing::warn!(
                slack_workspace_id = workspace_id,
                slack_user_id = user_id,
                "User not found in database"
            );
            AppError::BadRequest("User not authenticated with Spotify".to_string())
        })?;

    // Check if token is expired or will expire soon (within 5 minutes)
    let now = Utc::now();
    let buffer = Duration::minutes(5);
    let needs_refresh = user_auth.expires_at <= now + buffer;

    if needs_refresh {
        tracing::info!(
            user_auth_id = %user_auth.id,
            expires_at = %user_auth.expires_at,
            now = %now,
            "Access token expired or expiring soon, refreshing"
        );

        refresh_access_token(pool, oauth_client, &user_auth).await
    } else {
        tracing::debug!(
            user_auth_id = %user_auth.id,
            expires_at = %user_auth.expires_at,
            "Access token still valid, using existing token"
        );

        Ok(user_auth.access_token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::db::repository::upsert_user_auth;
    use crate::spotify::oauth::build_oauth_client;
    use chrono::Utc;

    #[sqlx::test]
    async fn test_ensure_valid_token_not_found(pool: PgPool) -> sqlx::Result<()> {
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

        let oauth_client = build_oauth_client(&config);

        let result = ensure_valid_token(&pool, &oauth_client, "T123", "U456").await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::BadRequest(_)));

        Ok(())
    }

    #[sqlx::test]
    async fn test_ensure_valid_token_still_valid(pool: PgPool) -> sqlx::Result<()> {
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

        let oauth_client = build_oauth_client(&config);

        // Create user with token that expires in 1 hour (still valid)
        let expires_at = Utc::now() + Duration::hours(1);
        let user_auth = upsert_user_auth(
            &pool,
            "T123",
            "U456",
            Some("spotify_user_id".to_string()),
            "valid_access_token",
            "valid_refresh_token",
            expires_at,
        )
        .await?;

        let result = ensure_valid_token(&pool, &oauth_client, "T123", "U456").await;

        // Should return existing token without refreshing
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "valid_access_token");

        // Verify token wasn't updated in database
        let updated_user = get_user_auth(&pool, "T123", "U456").await?.unwrap();
        assert_eq!(updated_user.access_token, user_auth.access_token);
        assert_eq!(updated_user.expires_at, user_auth.expires_at);

        Ok(())
    }

    #[sqlx::test]
    async fn test_ensure_valid_token_expired(pool: PgPool) -> sqlx::Result<()> {
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

        let oauth_client = build_oauth_client(&config);

        // Create user with expired token
        let expires_at = Utc::now() - Duration::hours(1);
        upsert_user_auth(
            &pool,
            "T123",
            "U456",
            Some("spotify_user_id".to_string()),
            "expired_access_token",
            "valid_refresh_token",
            expires_at,
        )
        .await?;

        let result = ensure_valid_token(&pool, &oauth_client, "T123", "U456").await;

        // Will fail because we can't actually refresh with test credentials
        // But we can verify it attempted to refresh
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AppError::SpotifyApi(_)));

        Ok(())
    }
}
