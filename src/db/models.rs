use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserAuth {
    pub id: Uuid,
    pub slack_workspace_id: String,
    pub slack_user_id: String,
    pub spotify_user_id: Option<String>,
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: DateTime<Utc>,
    pub paused: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SaveActionLog {
    pub id: Uuid,
    pub slack_workspace_id: String,
    pub slack_user_id: String,
    pub channel_id: String,
    pub thread_ts: String,
    pub mention_ts: String,
    pub spotify_track_id: String,
    pub status: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}
