use crate::error::AppError;
use crate::slack::events::{ConversationsRepliesResponse, SlackMessage};

/// Fetch all messages in a thread
///
/// Calls Slack's `conversations.replies` API to get all messages in a thread.
/// This is used to find Spotify links shared in the conversation.
///
/// # Arguments
/// * `bot_token` - Slack bot token (xoxb-...)
/// * `channel_id` - Channel ID where the thread exists
/// * `thread_ts` - Thread timestamp (thread root message timestamp)
///
/// # Returns
/// Vector of messages in the thread, ordered chronologically
///
/// # Errors
/// - `SlackApi` if the API call fails or returns an error
pub async fn fetch_thread_messages(
    bot_token: &str,
    channel_id: &str,
    thread_ts: &str,
) -> Result<Vec<SlackMessage>, AppError> {
    tracing::info!(
        channel_id = channel_id,
        thread_ts = thread_ts,
        "Fetching thread messages from Slack"
    );

    let client = reqwest::Client::new();
    let url = "https://slack.com/api/conversations.replies";

    let response = client
        .get(url)
        .bearer_auth(bot_token)
        .query(&[("channel", channel_id), ("ts", thread_ts)])
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Failed to call Slack API: {:?}", e);
            AppError::SlackApi(format!("Failed to call conversations.replies: {}", e))
        })?;

    let api_response = response
        .json::<ConversationsRepliesResponse>()
        .await
        .map_err(|e| {
            tracing::error!("Failed to parse Slack API response: {:?}", e);
            AppError::SlackApi(format!("Failed to parse response: {}", e))
        })?;

    if !api_response.ok {
        let error_msg = api_response
            .error
            .unwrap_or_else(|| "Unknown error".to_string());
        tracing::error!(
            channel_id = channel_id,
            thread_ts = thread_ts,
            error = error_msg,
            "Slack API returned error"
        );
        return Err(AppError::SlackApi(format!(
            "conversations.replies failed: {}",
            error_msg
        )));
    }

    let messages = api_response.messages.unwrap_or_default();

    tracing::info!(
        channel_id = channel_id,
        thread_ts = thread_ts,
        message_count = messages.len(),
        "Successfully fetched thread messages"
    );

    Ok(messages)
}

/// Add a reaction to a Slack message
///
/// Calls Slack's `reactions.add` API to add an emoji reaction to a message.
/// Used for visual feedback (✅ success, ♻️ already saved, ❌ error).
///
/// # Arguments
/// * `bot_token` - Slack bot token (xoxb-...)
/// * `channel_id` - Channel ID where the message exists
/// * `timestamp` - Message timestamp
/// * `reaction` - Emoji name without colons (e.g., "white_check_mark" for ✅)
///
/// # Returns
/// Ok(()) if reaction was added successfully
///
/// # Errors
/// - `SlackApi` if the API call fails or returns an error
pub async fn add_reaction(
    bot_token: &str,
    channel_id: &str,
    timestamp: &str,
    reaction: &str,
) -> Result<(), AppError> {
    tracing::info!(
        channel_id = channel_id,
        timestamp = timestamp,
        reaction = reaction,
        "Adding reaction to message"
    );

    let client = reqwest::Client::new();
    let url = "https://slack.com/api/reactions.add";

    let response = client
        .post(url)
        .bearer_auth(bot_token)
        .json(&serde_json::json!({
            "channel": channel_id,
            "timestamp": timestamp,
            "name": reaction
        }))
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Failed to call Slack API: {:?}", e);
            AppError::SlackApi(format!("Failed to call reactions.add: {}", e))
        })?;

    let api_response: serde_json::Value = response.json().await.map_err(|e| {
        tracing::error!("Failed to parse Slack API response: {:?}", e);
        AppError::SlackApi(format!("Failed to parse response: {}", e))
    })?;

    let ok = api_response["ok"].as_bool().unwrap_or(false);
    if !ok {
        let error_msg = api_response["error"]
            .as_str()
            .unwrap_or("Unknown error")
            .to_string();

        // If the reaction already exists, that's fine
        if error_msg == "already_reacted" {
            tracing::debug!("Reaction already exists, ignoring");
            return Ok(());
        }

        tracing::error!(
            channel_id = channel_id,
            timestamp = timestamp,
            reaction = reaction,
            error = error_msg,
            "Slack API returned error"
        );
        return Err(AppError::SlackApi(format!(
            "reactions.add failed: {}",
            error_msg
        )));
    }

    tracing::info!(
        channel_id = channel_id,
        timestamp = timestamp,
        reaction = reaction,
        "Successfully added reaction"
    );

    Ok(())
}

// Note: Actual API testing would require mocking or integration tests with real Slack API
