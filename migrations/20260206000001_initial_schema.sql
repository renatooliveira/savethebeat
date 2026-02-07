-- User authentication table
CREATE TABLE user_auth (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slack_workspace_id TEXT NOT NULL,
    slack_user_id TEXT NOT NULL,
    spotify_user_id TEXT,
    access_token TEXT NOT NULL,
    refresh_token TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    paused BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for lookups by Slack identity
CREATE UNIQUE INDEX idx_user_auth_slack ON user_auth(slack_workspace_id, slack_user_id);
CREATE INDEX idx_user_auth_workspace ON user_auth(slack_workspace_id);

-- Save action log table
CREATE TABLE save_action_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slack_workspace_id TEXT NOT NULL,
    slack_user_id TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    thread_ts TEXT NOT NULL,
    mention_ts TEXT NOT NULL,
    spotify_track_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('saved', 'already_saved', 'failed')),
    error_code TEXT,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for lookups and deduplication
CREATE INDEX idx_save_log_user ON save_action_log(slack_workspace_id, slack_user_id);
CREATE UNIQUE INDEX idx_save_log_unique ON save_action_log(
    slack_workspace_id,
    slack_user_id,
    thread_ts,
    spotify_track_id
);

-- Update trigger for user_auth.updated_at
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_user_auth_updated_at
    BEFORE UPDATE ON user_auth
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();
