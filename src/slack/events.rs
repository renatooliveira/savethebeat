use serde::{Deserialize, Serialize};

/// Top-level Slack event request
///
/// Slack sends different types of events:
/// - `url_verification`: Initial challenge when configuring the endpoint
/// - `event_callback`: Actual events like app_mention
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum SlackEventRequest {
    #[serde(rename = "url_verification")]
    UrlVerification { challenge: String },

    #[serde(rename = "event_callback")]
    EventCallback {
        team_id: String,
        event: SlackEvent,
        event_id: String,
        event_time: i64,
    },
}

/// Slack event types we handle
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum SlackEvent {
    #[serde(rename = "app_mention")]
    AppMention {
        user: String,
        text: String,
        ts: String,
        channel: String,
        #[serde(default)]
        thread_ts: Option<String>,
    },
}

/// Response for url_verification challenge
#[derive(Debug, Serialize)]
pub struct UrlVerificationResponse {
    pub challenge: String,
}

/// Slack API response for conversations.replies
#[derive(Debug, Deserialize)]
pub struct ConversationsRepliesResponse {
    pub ok: bool,
    pub messages: Option<Vec<SlackMessage>>,
    pub error: Option<String>,
}

/// Slack message structure
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SlackMessage {
    pub ts: String,
    pub user: Option<String>,
    pub text: String,
    #[serde(default)]
    pub thread_ts: Option<String>,
}

/// Event metadata extracted from app_mention
#[derive(Debug, Clone)]
pub struct MentionEvent {
    pub workspace_id: String,
    pub user_id: String,
    pub channel_id: String,
    pub thread_ts: String,
    pub mention_ts: String,
    pub text: String,
}

impl MentionEvent {
    /// Extract metadata from app_mention event
    pub fn from_event_callback(team_id: String, event: &SlackEvent) -> Option<Self> {
        match event {
            SlackEvent::AppMention {
                user,
                text,
                ts,
                channel,
                thread_ts,
            } => Some(MentionEvent {
                workspace_id: team_id,
                user_id: user.clone(),
                channel_id: channel.clone(),
                thread_ts: thread_ts.clone().unwrap_or_else(|| ts.clone()),
                mention_ts: ts.clone(),
                text: text.clone(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_url_verification() {
        let json = r#"{
            "type": "url_verification",
            "challenge": "3eZbrw1aBm2rZgRNFdxV2595E9CY3gmdALWMmHkvFXO7tYXAYM8P"
        }"#;

        let event: SlackEventRequest = serde_json::from_str(json).unwrap();
        match event {
            SlackEventRequest::UrlVerification { challenge } => {
                assert_eq!(
                    challenge,
                    "3eZbrw1aBm2rZgRNFdxV2595E9CY3gmdALWMmHkvFXO7tYXAYM8P"
                );
            }
            _ => panic!("Expected UrlVerification"),
        }
    }

    #[test]
    fn test_deserialize_app_mention() {
        let json = r#"{
            "type": "event_callback",
            "team_id": "T123ABC",
            "event_id": "Ev123ABC",
            "event_time": 1234567890,
            "event": {
                "type": "app_mention",
                "user": "U123ABC",
                "text": "<@U456DEF> save this track",
                "ts": "1234567890.123456",
                "channel": "C123ABC",
                "thread_ts": "1234567890.000000"
            }
        }"#;

        let event: SlackEventRequest = serde_json::from_str(json).unwrap();
        match event {
            SlackEventRequest::EventCallback {
                team_id,
                event,
                event_id,
                event_time,
            } => {
                assert_eq!(team_id, "T123ABC");
                assert_eq!(event_id, "Ev123ABC");
                assert_eq!(event_time, 1234567890);

                match event {
                    SlackEvent::AppMention {
                        user,
                        text,
                        ts,
                        channel,
                        thread_ts,
                    } => {
                        assert_eq!(user, "U123ABC");
                        assert_eq!(text, "<@U456DEF> save this track");
                        assert_eq!(ts, "1234567890.123456");
                        assert_eq!(channel, "C123ABC");
                        assert_eq!(thread_ts, Some("1234567890.000000".to_string()));
                    }
                }
            }
            _ => panic!("Expected EventCallback"),
        }
    }

    #[test]
    fn test_mention_event_from_event_callback() {
        let event = SlackEvent::AppMention {
            user: "U123ABC".to_string(),
            text: "<@U456DEF> save this".to_string(),
            ts: "1234567890.123456".to_string(),
            channel: "C123ABC".to_string(),
            thread_ts: Some("1234567890.000000".to_string()),
        };

        let mention = MentionEvent::from_event_callback("T123ABC".to_string(), &event).unwrap();
        assert_eq!(mention.workspace_id, "T123ABC");
        assert_eq!(mention.user_id, "U123ABC");
        assert_eq!(mention.channel_id, "C123ABC");
        assert_eq!(mention.thread_ts, "1234567890.000000");
        assert_eq!(mention.mention_ts, "1234567890.123456");
    }

    #[test]
    fn test_mention_event_no_thread_ts() {
        let event = SlackEvent::AppMention {
            user: "U123ABC".to_string(),
            text: "<@U456DEF> save this".to_string(),
            ts: "1234567890.123456".to_string(),
            channel: "C123ABC".to_string(),
            thread_ts: None,
        };

        let mention = MentionEvent::from_event_callback("T123ABC".to_string(), &event).unwrap();
        // When no thread_ts, should use message ts as thread_ts
        assert_eq!(mention.thread_ts, "1234567890.123456");
        assert_eq!(mention.mention_ts, "1234567890.123456");
    }
}
