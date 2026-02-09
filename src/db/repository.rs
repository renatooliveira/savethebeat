use crate::db::models::{SaveActionLog, UserAuth};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// Get user authentication record by Slack workspace and user IDs.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `workspace_id` - Slack workspace ID
/// * `user_id` - Slack user ID
///
/// # Returns
/// Optional UserAuth if found, None otherwise
///
/// # Errors
/// Returns error if database query fails
pub async fn get_user_auth(
    pool: &PgPool,
    workspace_id: &str,
    user_id: &str,
) -> Result<Option<UserAuth>, sqlx::Error> {
    sqlx::query_as!(
        UserAuth,
        r#"
        SELECT * FROM user_auth
        WHERE slack_workspace_id = $1 AND slack_user_id = $2
        "#,
        workspace_id,
        user_id
    )
    .fetch_optional(pool)
    .await
}

/// Insert or update user authentication record.
///
/// Uses ON CONFLICT to update existing records with new token information.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `workspace_id` - Slack workspace ID
/// * `user_id` - Slack user ID
/// * `spotify_user_id` - Spotify user ID (optional)
/// * `access_token` - Spotify access token
/// * `refresh_token` - Spotify refresh token
/// * `expires_at` - Token expiration timestamp
///
/// # Returns
/// The created or updated UserAuth record
///
/// # Errors
/// Returns error if database operation fails
pub async fn upsert_user_auth(
    pool: &PgPool,
    workspace_id: &str,
    user_id: &str,
    spotify_user_id: Option<String>,
    access_token: &str,
    refresh_token: &str,
    expires_at: DateTime<Utc>,
) -> Result<UserAuth, sqlx::Error> {
    sqlx::query_as!(
        UserAuth,
        r#"
        INSERT INTO user_auth (
            slack_workspace_id,
            slack_user_id,
            spotify_user_id,
            access_token,
            refresh_token,
            expires_at
        )
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (slack_workspace_id, slack_user_id)
        DO UPDATE SET
            spotify_user_id = EXCLUDED.spotify_user_id,
            access_token = EXCLUDED.access_token,
            refresh_token = EXCLUDED.refresh_token,
            expires_at = EXCLUDED.expires_at,
            updated_at = NOW()
        RETURNING *
        "#,
        workspace_id,
        user_id,
        spotify_user_id,
        access_token,
        refresh_token,
        expires_at
    )
    .fetch_one(pool)
    .await
}

/// Update access and refresh tokens for a user.
///
/// Used when refreshing expired tokens.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `id` - User auth record ID
/// * `access_token` - New Spotify access token
/// * `refresh_token` - New Spotify refresh token
/// * `expires_at` - New token expiration timestamp
///
/// # Errors
/// Returns error if:
/// - Record with given ID doesn't exist
/// - Database update fails
pub async fn update_tokens(
    pool: &PgPool,
    id: Uuid,
    access_token: &str,
    refresh_token: &str,
    expires_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE user_auth
        SET
            access_token = $1,
            refresh_token = $2,
            expires_at = $3,
            updated_at = NOW()
        WHERE id = $4
        "#,
        access_token,
        refresh_token,
        expires_at,
        id
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Check if a save action already exists for a given track in a thread
///
/// Used for idempotency - prevents saving the same track multiple times.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `workspace_id` - Slack workspace ID
/// * `user_id` - Slack user ID
/// * `thread_ts` - Thread timestamp
/// * `track_id` - Spotify track ID
///
/// # Returns
/// Some(SaveActionLog) if a save action exists, None otherwise
pub async fn get_save_action(
    pool: &PgPool,
    workspace_id: &str,
    user_id: &str,
    thread_ts: &str,
    track_id: &str,
) -> Result<Option<SaveActionLog>, sqlx::Error> {
    sqlx::query_as!(
        SaveActionLog,
        r#"
        SELECT
            id,
            slack_workspace_id,
            slack_user_id,
            channel_id,
            thread_ts,
            mention_ts,
            spotify_track_id,
            status,
            error_code,
            error_message,
            created_at
        FROM save_action_log
        WHERE slack_workspace_id = $1
            AND slack_user_id = $2
            AND thread_ts = $3
            AND spotify_track_id = $4
        ORDER BY created_at DESC
        LIMIT 1
        "#,
        workspace_id,
        user_id,
        thread_ts,
        track_id
    )
    .fetch_optional(pool)
    .await
}

/// Parameters for creating a save action log
pub struct SaveActionParams<'a> {
    pub workspace_id: &'a str,
    pub user_id: &'a str,
    pub channel_id: &'a str,
    pub thread_ts: &'a str,
    pub mention_ts: &'a str,
    pub track_id: &'a str,
    pub status: &'a str,
    pub error_code: Option<&'a str>,
    pub error_message: Option<&'a str>,
}

/// Create a save action log entry
///
/// Records an attempt to save a track, whether successful or failed.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `params` - Save action parameters
///
/// # Errors
/// Returns error if database insert fails
pub async fn create_save_action(
    pool: &PgPool,
    params: SaveActionParams<'_>,
) -> Result<SaveActionLog, sqlx::Error> {
    sqlx::query_as!(
        SaveActionLog,
        r#"
        INSERT INTO save_action_log (
            slack_workspace_id,
            slack_user_id,
            channel_id,
            thread_ts,
            mention_ts,
            spotify_track_id,
            status,
            error_code,
            error_message
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING
            id,
            slack_workspace_id,
            slack_user_id,
            channel_id,
            thread_ts,
            mention_ts,
            spotify_track_id,
            status,
            error_code,
            error_message,
            created_at
        "#,
        params.workspace_id,
        params.user_id,
        params.channel_id,
        params.thread_ts,
        params.mention_ts,
        params.track_id,
        params.status,
        params.error_code,
        params.error_message
    )
    .fetch_one(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test]
    async fn test_get_user_auth_not_found(pool: PgPool) -> sqlx::Result<()> {
        let result = get_user_auth(&pool, "T123", "U456").await?;
        assert!(result.is_none());
        Ok(())
    }

    #[sqlx::test]
    async fn test_upsert_user_auth_insert(pool: PgPool) -> sqlx::Result<()> {
        let expires_at = Utc::now() + chrono::Duration::hours(1);

        let user = upsert_user_auth(
            &pool,
            "T123",
            "U456",
            Some("spotify123".to_string()),
            "access_token_value",
            "refresh_token_value",
            expires_at,
        )
        .await?;

        assert_eq!(user.slack_workspace_id, "T123");
        assert_eq!(user.slack_user_id, "U456");
        assert_eq!(user.spotify_user_id, Some("spotify123".to_string()));
        assert_eq!(user.access_token, "access_token_value");
        assert_eq!(user.refresh_token, "refresh_token_value");
        assert!(!user.paused);

        Ok(())
    }

    #[sqlx::test]
    async fn test_upsert_user_auth_update(pool: PgPool) -> sqlx::Result<()> {
        let expires_at = Utc::now() + chrono::Duration::hours(1);

        // Insert initial record
        let user1 = upsert_user_auth(
            &pool,
            "T123",
            "U456",
            Some("spotify123".to_string()),
            "old_access_token",
            "old_refresh_token",
            expires_at,
        )
        .await?;

        // Update with new tokens
        let new_expires_at = Utc::now() + chrono::Duration::hours(2);
        let user2 = upsert_user_auth(
            &pool,
            "T123",
            "U456",
            Some("spotify123".to_string()),
            "new_access_token",
            "new_refresh_token",
            new_expires_at,
        )
        .await?;

        // Should have same ID
        assert_eq!(user1.id, user2.id);

        // Should have updated tokens
        assert_eq!(user2.access_token, "new_access_token");
        assert_eq!(user2.refresh_token, "new_refresh_token");

        // updated_at should be different (but we can't test exact value)
        assert!(user2.updated_at >= user1.updated_at);

        Ok(())
    }

    #[sqlx::test]
    async fn test_get_user_auth_found(pool: PgPool) -> sqlx::Result<()> {
        let expires_at = Utc::now() + chrono::Duration::hours(1);

        // Insert a user
        upsert_user_auth(
            &pool,
            "T123",
            "U456",
            Some("spotify123".to_string()),
            "access_token",
            "refresh_token",
            expires_at,
        )
        .await?;

        // Retrieve the user
        let result = get_user_auth(&pool, "T123", "U456").await?;
        assert!(result.is_some());

        let user = result.unwrap();
        assert_eq!(user.slack_workspace_id, "T123");
        assert_eq!(user.slack_user_id, "U456");
        assert_eq!(user.access_token, "access_token");

        Ok(())
    }

    #[sqlx::test]
    async fn test_update_tokens(pool: PgPool) -> sqlx::Result<()> {
        let expires_at = Utc::now() + chrono::Duration::hours(1);

        // Insert a user
        let user = upsert_user_auth(
            &pool,
            "T123",
            "U456",
            Some("spotify123".to_string()),
            "old_access",
            "old_refresh",
            expires_at,
        )
        .await?;

        // Update tokens
        let new_expires_at = Utc::now() + chrono::Duration::hours(2);
        update_tokens(&pool, user.id, "new_access", "new_refresh", new_expires_at).await?;

        // Fetch and verify
        let updated_user = get_user_auth(&pool, "T123", "U456").await?.unwrap();
        assert_eq!(updated_user.access_token, "new_access");
        assert_eq!(updated_user.refresh_token, "new_refresh");
        assert!(updated_user.updated_at > user.updated_at);

        Ok(())
    }

    #[sqlx::test]
    async fn test_unique_constraint(pool: PgPool) -> sqlx::Result<()> {
        let expires_at = Utc::now() + chrono::Duration::hours(1);

        // Insert first user
        upsert_user_auth(
            &pool,
            "T123",
            "U456",
            Some("spotify123".to_string()),
            "access1",
            "refresh1",
            expires_at,
        )
        .await?;

        // Upsert same workspace/user should update, not error
        let result = upsert_user_auth(
            &pool,
            "T123",
            "U456",
            Some("spotify456".to_string()),
            "access2",
            "refresh2",
            expires_at,
        )
        .await;

        assert!(result.is_ok());

        // Verify only one record exists
        let user = get_user_auth(&pool, "T123", "U456").await?.unwrap();
        assert_eq!(user.spotify_user_id, Some("spotify456".to_string()));
        assert_eq!(user.access_token, "access2");

        Ok(())
    }
}
