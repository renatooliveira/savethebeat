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

// Note: Actual API testing would require mocking or integration tests with real Slack API
