use crate::db::repository::{SaveActionParams, create_save_action, get_save_action};
use crate::error::AppError;
use crate::slack::client::{add_reaction, fetch_thread_messages, post_message};
use crate::slack::events::{MentionEvent, SlackEventRequest};
use crate::slack::verification::verify_slack_signature;
use crate::spotify::client::{ensure_valid_token, save_track};
use crate::spotify::parser::find_first_track;
use axum::{
    Json,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use oauth2::basic::BasicClient;
use sqlx::PgPool;

/// Application state for Slack routes
#[derive(Clone)]
pub struct SlackState {
    pub signing_secret: String,
    pub bot_token: String,
    pub db: PgPool,
    pub oauth_client: BasicClient,
    pub base_url: String,
}

/// Handle Slack events webhook
///
/// # Endpoint
/// POST /slack/events
///
/// # Flow
/// 1. Verify request signature (HMAC-SHA256)
/// 2. Parse event payload
/// 3. Handle url_verification challenge (initial setup)
/// 4. Handle event_callback for app_mention events
/// 5. Fetch thread messages
/// 6. Log event (actual track saving is Phase 3)
///
/// # Headers
/// - `X-Slack-Request-Timestamp`: Request timestamp
/// - `X-Slack-Signature`: HMAC-SHA256 signature
///
/// # Returns
/// - 200 OK with challenge for url_verification
/// - 200 OK for successful event processing
/// - 401 Unauthorized for invalid signature
/// - 400 Bad Request for invalid payload
///
/// # Errors
/// - Signature verification failures
/// - Invalid JSON payload
/// - Slack API errors
pub async fn handle_slack_events(
    State(state): State<SlackState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    tracing::info!("Received Slack event webhook");

    // Extract signature headers
    let timestamp = headers
        .get("X-Slack-Request-Timestamp")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::SignatureMissing)?;

    let signature = headers
        .get("X-Slack-Signature")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::SignatureMissing)?;

    // Verify signature
    verify_slack_signature(&state.signing_secret, timestamp, &body, signature)?;

    // Parse event payload
    let event_request: SlackEventRequest = serde_json::from_slice(&body).map_err(|e| {
        tracing::error!("Failed to parse Slack event: {:?}", e);
        AppError::BadRequest(format!("Invalid JSON: {}", e))
    })?;

    match event_request {
        SlackEventRequest::UrlVerification { challenge } => {
            tracing::info!(challenge = %challenge, "Handling url_verification challenge");
            Ok((
                StatusCode::OK,
                Json(serde_json::json!({ "challenge": challenge })),
            ))
        }

        SlackEventRequest::EventCallback {
            team_id,
            event,
            event_id,
            event_time,
        } => {
            tracing::info!(
                team_id = %team_id,
                event_id = %event_id,
                event_time = event_time,
                "Handling event_callback"
            );

            // Extract mention event metadata
            let mention = MentionEvent::from_event_callback(team_id, &event).ok_or_else(|| {
                tracing::warn!("Unsupported event type");
                AppError::BadRequest("Unsupported event type".to_string())
            })?;

            tracing::info!(
                workspace_id = %mention.workspace_id,
                user_id = %mention.user_id,
                channel_id = %mention.channel_id,
                thread_ts = %mention.thread_ts,
                "Processing app_mention event"
            );

            // Process the mention in a background task (Slack expects response within 3 seconds)
            tokio::spawn(async move {
                if let Err(e) = process_mention(state, mention).await {
                    tracing::error!("Failed to process mention: {:?}", e);
                }
            });

            // Return 200 OK immediately
            Ok((StatusCode::OK, Json(serde_json::json!({ "status": "ok" }))))
        }
    }
}

/// Process an app_mention event
///
/// This runs in a background task to avoid blocking the Slack event response.
///
/// # Flow
/// 1. Fetch thread messages
/// 2. Find first Spotify track link
/// 3. Check if already saved (idempotency)
/// 4. Get valid Spotify token (refresh if needed)
/// 5. Save track to Spotify library
/// 6. Add Slack reaction based on result
/// 7. Log the action to database
async fn process_mention(state: SlackState, mention: MentionEvent) -> Result<(), AppError> {
    tracing::info!(
        workspace_id = %mention.workspace_id,
        user_id = %mention.user_id,
        channel_id = %mention.channel_id,
        thread_ts = %mention.thread_ts,
        "Processing mention in background"
    );

    // Check if this is a "connect" command
    let text_lower = mention.text.to_lowercase();
    if text_lower.contains("connect") {
        tracing::info!("Detected 'connect' command, sending OAuth link via DM");

        // Build OAuth URL
        let oauth_url = format!(
            "{}/spotify/connect?slack_workspace_id={}&slack_user_id={}",
            state.base_url, mention.workspace_id, mention.user_id
        );

        // Send DM to user (use user_id as channel for DM)
        let message = format!("Click here to connect your Spotify account: {}", oauth_url);

        post_message(&state.bot_token, &mention.user_id, &message).await?;

        tracing::info!("Sent OAuth connection link via DM to user");
        return Ok(());
    }

    // Fetch thread messages to find Spotify links
    let messages =
        fetch_thread_messages(&state.bot_token, &mention.channel_id, &mention.thread_ts).await?;

    tracing::info!(message_count = messages.len(), "Fetched thread messages");

    // Extract message text
    let message_texts: Vec<String> = messages.iter().map(|m| m.text.clone()).collect();

    // Find first Spotify track link
    let track_id = match find_first_track(&message_texts) {
        Some(id) => id,
        None => {
            tracing::warn!("No Spotify track links found in thread");
            add_reaction(
                &state.bot_token,
                &mention.channel_id,
                &mention.mention_ts,
                "x",
            )
            .await?;
            return Ok(());
        }
    };

    tracing::info!(track_id = %track_id, "Found Spotify track");

    // Check if already saved (idempotency)
    if let Some(existing) = get_save_action(
        &state.db,
        &mention.workspace_id,
        &mention.user_id,
        &mention.thread_ts,
        &track_id,
    )
    .await?
    {
        tracing::info!(
            track_id = %track_id,
            status = %existing.status,
            "Track already processed"
        );

        // Add "recycle" reaction for already saved
        add_reaction(
            &state.bot_token,
            &mention.channel_id,
            &mention.mention_ts,
            "recycle",
        )
        .await?;

        // Log as already_saved
        create_save_action(
            &state.db,
            SaveActionParams {
                workspace_id: &mention.workspace_id,
                user_id: &mention.user_id,
                channel_id: &mention.channel_id,
                thread_ts: &mention.thread_ts,
                mention_ts: &mention.mention_ts,
                track_id: &track_id,
                status: "already_saved",
                error_code: None,
                error_message: None,
            },
        )
        .await?;

        return Ok(());
    }

    // Get valid Spotify access token (refresh if needed)
    let access_token = match ensure_valid_token(
        &state.db,
        &state.oauth_client,
        &mention.workspace_id,
        &mention.user_id,
    )
    .await
    {
        Ok(token) => token,
        Err(e) => {
            tracing::error!("Failed to get valid token: {:?}", e);
            add_reaction(
                &state.bot_token,
                &mention.channel_id,
                &mention.mention_ts,
                "x",
            )
            .await?;

            create_save_action(
                &state.db,
                SaveActionParams {
                    workspace_id: &mention.workspace_id,
                    user_id: &mention.user_id,
                    channel_id: &mention.channel_id,
                    thread_ts: &mention.thread_ts,
                    mention_ts: &mention.mention_ts,
                    track_id: &track_id,
                    status: "failed",
                    error_code: Some("auth_error"),
                    error_message: Some(&format!("Failed to authenticate: {}", e)),
                },
            )
            .await?;

            return Err(e);
        }
    };

    // Save track to Spotify library
    match save_track(&access_token, &track_id).await {
        Ok(()) => {
            tracing::info!(track_id = %track_id, "Successfully saved track");

            // Add success reaction
            add_reaction(
                &state.bot_token,
                &mention.channel_id,
                &mention.mention_ts,
                "white_check_mark",
            )
            .await?;

            // Log successful save
            create_save_action(
                &state.db,
                SaveActionParams {
                    workspace_id: &mention.workspace_id,
                    user_id: &mention.user_id,
                    channel_id: &mention.channel_id,
                    thread_ts: &mention.thread_ts,
                    mention_ts: &mention.mention_ts,
                    track_id: &track_id,
                    status: "saved",
                    error_code: None,
                    error_message: None,
                },
            )
            .await?;

            Ok(())
        }
        Err(e) => {
            tracing::error!(track_id = %track_id, error = ?e, "Failed to save track");

            // Add error reaction
            add_reaction(
                &state.bot_token,
                &mention.channel_id,
                &mention.mention_ts,
                "x",
            )
            .await?;

            // Log failure
            create_save_action(
                &state.db,
                SaveActionParams {
                    workspace_id: &mention.workspace_id,
                    user_id: &mention.user_id,
                    channel_id: &mention.channel_id,
                    thread_ts: &mention.thread_ts,
                    mention_ts: &mention.mention_ts,
                    track_id: &track_id,
                    status: "failed",
                    error_code: Some("spotify_error"),
                    error_message: Some(&format!("Failed to save: {}", e)),
                },
            )
            .await?;

            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::spotify::oauth::build_oauth_client;

    async fn create_test_state() -> SlackState {
        let config = Config {
            port: 3000,
            host: "0.0.0.0".to_string(),
            database_url: "postgresql://localhost/savethebeat_test".to_string(),
            spotify_client_id: "test_client_id".to_string(),
            spotify_client_secret: "test_client_secret".to_string(),
            spotify_redirect_uri: "http://localhost:3000/spotify/callback".to_string(),
            base_url: "http://localhost:3000".to_string(),
            slack_signing_secret: Some("test_secret".to_string()),
            slack_bot_token: Some("xoxb-test-token".to_string()),
            rust_log: "info".to_string(),
        };

        let db = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy(&config.database_url)
            .unwrap();

        SlackState {
            signing_secret: "test_secret".to_string(),
            bot_token: "xoxb-test-token".to_string(),
            db,
            oauth_client: build_oauth_client(&config),
            base_url: "http://localhost:3000".to_string(),
        }
    }

    #[tokio::test]
    async fn test_slack_state_creation() {
        let state = create_test_state().await;
        assert_eq!(state.signing_secret, "test_secret");
        assert_eq!(state.bot_token, "xoxb-test-token");
    }
}
