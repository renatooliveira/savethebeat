// Integration tests for savethebeat
//
// These tests verify end-to-end functionality of the application.
// They require a running PostgreSQL instance and valid test credentials.
//
// Run with: cargo test --test integration_test

use savethebeat::config::Config;
use sqlx::PgPool;

/// Test helper to create a test database pool
///
/// This uses the DATABASE_URL from the environment
async fn setup_test_db() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set for integration tests");

    PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database")
}

/// Test helper to clean up test data
async fn cleanup_test_data(pool: &PgPool, workspace_id: &str, user_id: &str) {
    sqlx::query!(
        "DELETE FROM save_action_log WHERE slack_workspace_id = $1 AND slack_user_id = $2",
        workspace_id,
        user_id
    )
    .execute(pool)
    .await
    .ok();

    sqlx::query!(
        "DELETE FROM user_auth WHERE slack_workspace_id = $1 AND slack_user_id = $2",
        workspace_id,
        user_id
    )
    .execute(pool)
    .await
    .ok();
}

#[tokio::test]
#[ignore] // Ignored by default - requires DATABASE_URL and test setup
async fn test_database_connection() {
    // Test that we can connect to the database
    let pool = setup_test_db().await;

    // Verify connection with a simple query
    let result = sqlx::query!("SELECT 1 as value")
        .fetch_one(&pool)
        .await
        .expect("Failed to execute test query");

    assert_eq!(result.value, Some(1));
}

#[tokio::test]
#[ignore] // Ignored by default - requires full environment setup
async fn test_user_auth_crud_operations() {
    let pool = setup_test_db().await;

    // Test data
    let workspace_id = "T_INTEGRATION_TEST";
    let user_id = "U_INTEGRATION_TEST";

    // Cleanup any existing test data
    cleanup_test_data(&pool, workspace_id, user_id).await;

    // Test: Create user auth
    let user_auth = savethebeat::db::repository::upsert_user_auth(
        &pool,
        workspace_id,
        user_id,
        Some("spotify_test_user".to_string()),
        "test_access_token",
        "test_refresh_token",
        chrono::Utc::now() + chrono::Duration::hours(1),
    )
    .await
    .expect("Failed to create user_auth");

    assert_eq!(user_auth.slack_workspace_id, workspace_id);
    assert_eq!(user_auth.slack_user_id, user_id);
    assert_eq!(user_auth.access_token, "test_access_token");

    // Test: Retrieve user auth
    let retrieved = savethebeat::db::repository::get_user_auth(&pool, workspace_id, user_id)
        .await
        .expect("Failed to get user_auth")
        .expect("User auth not found");

    assert_eq!(retrieved.id, user_auth.id);
    assert_eq!(retrieved.access_token, "test_access_token");

    // Test: Update tokens
    let new_expiry = chrono::Utc::now() + chrono::Duration::hours(2);
    savethebeat::db::repository::update_tokens(
        &pool,
        user_auth.id,
        "new_access_token",
        "new_refresh_token",
        new_expiry,
    )
    .await
    .expect("Failed to update tokens");

    // Verify update
    let updated = savethebeat::db::repository::get_user_auth(&pool, workspace_id, user_id)
        .await
        .expect("Failed to get updated user_auth")
        .expect("User auth not found after update");

    assert_eq!(updated.access_token, "new_access_token");
    assert_eq!(updated.refresh_token, "new_refresh_token");

    // Cleanup
    cleanup_test_data(&pool, workspace_id, user_id).await;
}

#[tokio::test]
#[ignore] // Ignored by default - requires full environment setup
async fn test_save_action_log_idempotency() {
    let pool = setup_test_db().await;

    // Test data
    let workspace_id = "T_IDEMPOTENCY_TEST";
    let user_id = "U_IDEMPOTENCY_TEST";
    let thread_ts = "1234567890.123456";
    let track_id = "test_track_123";

    // Cleanup
    cleanup_test_data(&pool, workspace_id, user_id).await;

    // Test: First save action
    let params1 = savethebeat::db::repository::SaveActionParams {
        workspace_id,
        user_id,
        channel_id: "C_TEST",
        thread_ts,
        mention_ts: "1234567891.123456",
        track_id,
        status: "saved",
        error_code: None,
        error_message: None,
    };

    let action1 = savethebeat::db::repository::create_save_action(&pool, params1)
        .await
        .expect("Failed to create first save action");

    assert_eq!(action1.status, "saved");

    // Test: Duplicate save attempt (should fail due to unique constraint)
    let params2 = savethebeat::db::repository::SaveActionParams {
        workspace_id,
        user_id,
        channel_id: "C_TEST",
        thread_ts, // Same thread
        mention_ts: "1234567892.123456", // Different mention
        track_id, // Same track - should violate unique constraint
        status: "saved",
        error_code: None,
        error_message: None,
    };

    let result = savethebeat::db::repository::create_save_action(&pool, params2).await;

    // Should fail with unique violation
    assert!(result.is_err());
    if let Err(e) = result {
        let error_message = e.to_string();
        assert!(
            error_message.contains("unique") || error_message.contains("duplicate"),
            "Expected unique constraint error, got: {}",
            error_message
        );
    }

    // Test: Check for existing save action
    let existing = savethebeat::db::repository::get_save_action(
        &pool,
        workspace_id,
        user_id,
        thread_ts,
        track_id,
    )
    .await
    .expect("Failed to check existing save action")
    .expect("Save action not found");

    assert_eq!(existing.id, action1.id);

    // Cleanup
    cleanup_test_data(&pool, workspace_id, user_id).await;
}

#[test]
fn test_config_from_env() {
    // Test that config can be loaded from environment
    // This is a basic smoke test - full config test requires all env vars

    // Set minimal required env vars
    unsafe {
        std::env::set_var("DATABASE_URL", "postgresql://localhost/test");
        std::env::set_var("SPOTIFY_CLIENT_ID", "test_client");
        std::env::set_var("SPOTIFY_CLIENT_SECRET", "test_secret");
        std::env::set_var(
            "SPOTIFY_REDIRECT_URI",
            "http://localhost:3000/spotify/callback",
        );
        std::env::set_var("BASE_URL", "http://localhost:3000");
    }

    // This would normally load from .env file, but we set vars above
    let config = Config::from_env().expect("Failed to load config");

    assert_eq!(config.database_url, "postgresql://localhost/test");
    assert_eq!(config.spotify_client_id, "test_client");
    assert!(config.port > 0);
}

// Example of how to test the Spotify link parser
// (this is already well-tested in unit tests, but showing integration pattern)
#[test]
fn test_spotify_parser_integration() {
    use savethebeat::spotify::parser::{extract_track_id, find_first_track};

    // Test various URL formats
    let test_cases = vec![
        (
            "https://open.spotify.com/track/3n3Ppam7vgaVa1iaRUc9Lp",
            Some("3n3Ppam7vgaVa1iaRUc9Lp"),
        ),
        (
            "https://open.spotify.com/track/7qiZfU4dY1lWllzX7mPBI",
            Some("7qiZfU4dY1lWllzX7mPBI"),
        ),
        ("spotify:track:abc123xyz", Some("abc123xyz")),
        ("no track here", None),
    ];

    for (input, expected) in test_cases {
        let result = extract_track_id(input);
        assert_eq!(
            result.as_deref(),
            expected,
            "Failed for input: {}",
            input
        );
    }

    // Test find_first_track with multiple messages
    let messages = vec![
        "Message without link".to_string(),
        "Check this out: https://open.spotify.com/track/FIRST".to_string(),
        "And this: https://open.spotify.com/track/SECOND".to_string(),
    ];

    let first = find_first_track(&messages);
    assert_eq!(first, Some("FIRST".to_string()));
}

// Note: Full end-to-end tests that actually call Spotify/Slack APIs
// would require:
// 1. Test credentials with appropriate scopes
// 2. Mocking external services or using test environments
// 3. Careful cleanup to avoid affecting production data
//
// For MVP, we rely on:
// - Unit tests for business logic
// - Integration tests for database operations (above)
// - Manual testing checklist (see TESTING.md) for external APIs
