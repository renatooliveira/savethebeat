# savethebeat

A Slack ↔ Spotify integration bot that allows users to save Spotify tracks to their library by mentioning the bot in Slack threads.

## Overview

When users share Spotify links in Slack conversations, they can mention `@savethebeat` to automatically save those tracks to their Spotify library. The bot handles OAuth authentication, token management, and seamless integration between Slack and Spotify.

## How It Works

1. **Share a Spotify track** in a Slack thread
2. **Mention the bot** with `@savethebeat`
3. **Bot saves the track** to your Spotify Liked Songs
4. **Get instant feedback** via emoji reaction:
   - ✅ Track saved successfully
   - ♻️ Track already saved (no duplicate)
   - ❌ Error occurred (no link found, auth issue, etc.)

**Example:**
```
[Slack Thread]
Alice: Check out this song! https://open.spotify.com/track/3n3Ppam7vgaVa1iaRUc9Lp
Bob: @savethebeat
[Bot adds ✅ reaction]
```

## Features

### Current (Phase 1-3 - MVP Complete! ✅)

**Spotify OAuth (Phase 1):**
- ✅ **OAuth Flow** - Users can connect their Spotify accounts
- ✅ **Secure Authentication** - CSRF-protected OAuth with state tokens
- ✅ **Token Persistence** - Access and refresh tokens stored in PostgreSQL
- ✅ **Token Refresh** - Automatic token renewal when expired
- ✅ **Spotify API Integration** - Token validation and user profile retrieval

**Slack Integration (Phase 2):**
- ✅ **Event Webhook** - Receive and process Slack app_mention events
- ✅ **Signature Verification** - HMAC-SHA256 signature verification with replay protection
- ✅ **Thread Resolution** - Fetch all messages in a thread via Slack API
- ✅ **Optional Configuration** - Slack integration enabled only when credentials are configured

**Track Saving (Phase 3 - MVP Core):**
- ✅ **Link Parsing** - Extract Spotify track IDs from URLs/URIs
- ✅ **Track Saving** - Save tracks to user's Liked Songs library
- ✅ **Slack Reactions** - Visual feedback (✅ saved, ♻️ already saved, ❌ error)
- ✅ **Idempotency** - Prevent duplicate saves via database unique constraint
- ✅ **Action Logging** - Track all save attempts with status and errors
- ✅ **End-to-End Flow** - Complete workflow from mention to save

**Infrastructure:**
- ✅ **Database Layer** - Repository pattern with compile-time checked queries (sqlx)
- ✅ **Error Handling** - Typed error variants with proper HTTP status codes
- ✅ **Structured Logging** - Comprehensive tracing throughout the application

### Upcoming

- ⏳ **Retry Logic** - Handle transient API failures (Phase 4)
- ⏳ **Rate Limiting** - Per-user rate limiting (Phase 4)
- ⏳ **Observability** - Correlation IDs and metrics (Phase 4)

## Tech Stack

- **Language:** Rust (Edition 2024)
- **Web Framework:** Axum 0.8
- **Runtime:** Tokio (async)
- **Database:** PostgreSQL with sqlx (compile-time checked queries)
- **OAuth:** oauth2 crate
- **Parsing:** regex crate for Spotify URL extraction
- **Security:** HMAC-SHA256 for Slack signature verification
- **Logging:** tracing + tracing-subscriber

## Prerequisites

- Rust 1.93 or later
- PostgreSQL 14+
- Spotify Developer Account (for OAuth credentials)
- Slack App (optional, for event integration)

## Setup

### 1. Install Dependencies

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install sqlx-cli
cargo install sqlx-cli --no-default-features --features postgres

# Install PostgreSQL (macOS)
brew install postgresql@14
brew services start postgresql@14
```

### 2. Create Spotify App

1. Go to [Spotify Developer Dashboard](https://developer.spotify.com/dashboard)
2. Create a new app
3. Add redirect URI: `http://127.0.0.1:3000/spotify/callback`
4. Note your Client ID and Client Secret

### 3. Create Slack App (Optional)

1. Go to [Slack API Apps](https://api.slack.com/apps)
2. Create a new app (from scratch)
3. Configure OAuth & Permissions:
   - Add Bot Token Scopes: `app_mentions:read`, `channels:history`, `groups:history`, `im:history`, `mpim:history`
   - Install app to workspace
   - Copy Bot User OAuth Token (starts with `xoxb-`)
4. Configure Event Subscriptions:
   - Enable Events
   - Request URL: `https://your-domain.com/slack/events`
   - Subscribe to bot events: `app_mention`
5. Note your Signing Secret from Basic Information

### 4. Database Setup

```bash
# Create database
createdb savethebeat

# Run migrations
sqlx migrate run
```

### 5. Environment Configuration

Create a `.env` file in the project root:

```bash
# Server
PORT=3000
HOST=0.0.0.0
RUST_LOG=info,savethebeat=debug
RUST_LOG_FORMAT=pretty

# Database
DATABASE_URL=postgresql://localhost/savethebeat

# Spotify OAuth (Required)
SPOTIFY_CLIENT_ID=your_spotify_client_id
SPOTIFY_CLIENT_SECRET=your_spotify_client_secret
SPOTIFY_REDIRECT_URI=http://127.0.0.1:3000/spotify/callback
BASE_URL=http://127.0.0.1:3000

# Slack Integration (Optional - Phase 2+)
SLACK_SIGNING_SECRET=your_signing_secret
SLACK_BOT_TOKEN=xoxb-your-bot-token
```

**Note:** Slack credentials are optional. If not provided, the server runs without Slack integration (Spotify OAuth still works).

### 6. Run the Application

```bash
# Development mode
cargo run

# Release mode
cargo run --release
```

The server will start on `http://127.0.0.1:3000`

## API Endpoints

### Health Check
```
GET /health
```

Returns server status and version.

### Spotify OAuth Flow

#### 1. Initiate Connection
```
GET /spotify/connect?slack_workspace_id=<WORKSPACE_ID>&slack_user_id=<USER_ID>
```

Redirects to Spotify for authorization.

#### 2. OAuth Callback
```
GET /spotify/callback?code=<CODE>&state=<STATE>
```

Handles Spotify OAuth callback, exchanges code for tokens, stores in database.

#### 3. Verify Authentication
```
GET /spotify/verify?slack_workspace_id=<WORKSPACE_ID>&slack_user_id=<USER_ID>
```

Verifies authentication, refreshes token if needed, returns Spotify user profile.

### Slack Events (Phase 2)

#### Webhook Endpoint
```
POST /slack/events
```

Receives Slack event webhooks with signature verification.

**Headers:**
- `X-Slack-Request-Timestamp` - Request timestamp
- `X-Slack-Signature` - HMAC-SHA256 signature

**Event Types:**
- `url_verification` - Initial challenge for endpoint setup
- `event_callback` - Actual events (e.g., app_mention)

**Security:**
- HMAC-SHA256 signature verification
- 5-minute timestamp window (replay protection)
- Constant-time comparison (timing attack protection)

## Development

### Run Tests

```bash
# All tests
cargo test

# With output
cargo test -- --nocapture

# Specific test
cargo test test_name
```

### Code Quality

```bash
# Format code
cargo fmt

# Lint
cargo clippy --all-targets --all-features -- -D warnings

# Check without building
cargo check
```

### Database Migrations

```bash
# Create new migration
sqlx migrate add migration_name

# Run migrations
sqlx migrate run

# Revert last migration
sqlx migrate revert

# Generate sqlx offline cache (for CI)
DATABASE_URL=postgresql://localhost/savethebeat cargo sqlx prepare
```

## Project Structure

```
savethebeat/
├── src/
│   ├── main.rs              # Entry point
│   ├── lib.rs               # Application setup and router
│   ├── config.rs            # Configuration from environment
│   ├── error.rs             # Error types
│   ├── telemetry.rs         # Logging setup
│   ├── db/
│   │   ├── mod.rs          # Database pool initialization
│   │   ├── models.rs       # Database models
│   │   └── repository.rs   # CRUD operations
│   ├── spotify/
│   │   ├── mod.rs          # Module exports
│   │   ├── oauth.rs        # OAuth client and state management
│   │   ├── client.rs       # Spotify API client
│   │   └── routes.rs       # HTTP handlers
│   ├── slack/
│   │   ├── mod.rs          # Module exports
│   │   ├── verification.rs # Signature verification
│   │   ├── events.rs       # Event types and structures
│   │   ├── client.rs       # Slack API client
│   │   └── routes.rs       # HTTP handlers
│   └── routes/
│       └── mod.rs          # Route aggregation
├── migrations/             # SQL migration files
├── templates/             # HTML templates
├── .sqlx/                # sqlx offline query cache
├── Cargo.toml            # Dependencies
└── .env                  # Environment variables (not in git)
```

## CI/CD

GitHub Actions workflow (`.github/workflows/ci.yml`):
- ✅ Format check (`cargo fmt`)
- ✅ Clippy lints (`cargo clippy`)
- ✅ Tests with PostgreSQL (`cargo test`)
- ✅ Release build (`cargo build --release`)

## Documentation

- **[PLAN.md](./PLAN.md)** - Detailed implementation plan and roadmap
- **[AGENTS.md](./AGENTS.md)** - Coding standards and best practices

## Contributing

This is a personal project, but issues and suggestions are welcome!

### Workflow

All changes go through Pull Requests:
1. Create feature branch: `git checkout -b feature/your-feature`
2. Make changes and commit
3. Push branch: `git push -u origin feature/your-feature`
4. Create PR via GitHub CLI or web interface
5. Wait for review and CI checks

## Current Status

**Phase 1:** ✅ COMPLETE (8/8 sub-phases)
**Phase 2:** ✅ COMPLETE
**Phase 3:** ✅ COMPLETE (MVP!)

### Completed Phases

**Phase 1: Spotify OAuth & Token Management**
- ✅ Database Setup
- ✅ Repository Layer
- ✅ OAuth Infrastructure
- ✅ Spotify Connect Endpoint
- ✅ Spotify Callback Endpoint
- ✅ Token Refresh Helper
- ✅ Application State Setup
- ✅ Testing & Verification

**Phase 2: Slack Integration**
- ✅ Slack event webhook endpoint
- ✅ Signature verification (HMAC-SHA256)
- ✅ url_verification handling
- ✅ app_mention event processing
- ✅ Thread message fetching

**Phase 3: Parse + Save + Feedback (MVP Core)**
- ✅ Spotify link parser (regex-based, supports multiple URL formats)
- ✅ First link selection logic
- ✅ Idempotency via database unique constraint
- ✅ Spotify save to Liked Songs API
- ✅ Slack reactions for feedback (✅/♻️/❌)
- ✅ Save action logging

**What works now (MVP Complete!):**
- 🎵 Users can mention `@savethebeat` in threads to save Spotify tracks
- 🔗 Bot automatically finds and parses Spotify links
- 💾 Tracks saved to user's Liked Songs library
- ✅ Instant visual feedback via Slack reactions
- ♻️ Idempotency prevents duplicate saves
- 🔐 Secure OAuth with automatic token refresh
- 📊 All actions logged to database for tracking
- 🛡️ HMAC-SHA256 signature verification with replay protection

**Next steps:**
- Phase 4: Retries, rate limiting, and observability
- Phase 5: Testing & rollout

## License

MIT License - see LICENSE file for details

## Author

Renato Oliveira
