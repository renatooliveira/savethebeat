use crate::error::AppError;
use crate::slack::client::fetch_thread_messages;
use crate::slack::events::{MentionEvent, SlackEventRequest};
use crate::slack::verification::verify_slack_signature;
use axum::{
    Json,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
};

/// Application state for Slack routes
#[derive(Clone)]
pub struct SlackState {
    pub signing_secret: String,
    pub bot_token: String,
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

            // Fetch thread messages to find Spotify links
            let messages =
                fetch_thread_messages(&state.bot_token, &mention.channel_id, &mention.thread_ts)
                    .await?;

            tracing::info!(message_count = messages.len(), "Fetched thread messages");

            // Log messages for now (Phase 3 will parse Spotify links and save tracks)
            for msg in &messages {
                tracing::debug!(
                    ts = %msg.ts,
                    user = ?msg.user,
                    text_preview = %msg.text.chars().take(50).collect::<String>(),
                    "Thread message"
                );
            }

            // Return 200 OK immediately (Slack expects response within 3 seconds)
            Ok((StatusCode::OK, Json(serde_json::json!({ "status": "ok" }))))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_state() -> SlackState {
        SlackState {
            signing_secret: "test_secret".to_string(),
            bot_token: "xoxb-test-token".to_string(),
        }
    }

    #[test]
    fn test_slack_state_creation() {
        let state = create_test_state();
        assert_eq!(state.signing_secret, "test_secret");
        assert_eq!(state.bot_token, "xoxb-test-token");
    }
}
