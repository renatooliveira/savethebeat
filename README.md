# savethebeat

A Slack ↔ Spotify integration bot that allows users to save Spotify tracks to their library by mentioning the bot in Slack threads.

## Overview

When users share Spotify links in Slack conversations, they can mention `@savethebeat` to automatically save those tracks to their Spotify library. The bot handles OAuth authentication, token management, and seamless integration between Slack and Spotify.

## Features

### Current (Phase 1 - OAuth Complete)

- ✅ **Spotify OAuth Flow** - Users can connect their Spotify accounts
- ✅ **Secure Authentication** - CSRF-protected OAuth with state tokens
- ✅ **Token Persistence** - Access and refresh tokens stored in PostgreSQL
- ✅ **Database Layer** - Repository pattern with compile-time checked queries (sqlx)
- ✅ **Error Handling** - Typed error variants with proper HTTP status codes
- ✅ **Structured Logging** - Comprehensive tracing throughout the application

### Upcoming

- ⏳ **Token Refresh** - Automatic token renewal (Phase 1.6)
- ⏳ **Slack Integration** - Handle `app_mention` events (Phase 2)
- ⏳ **Link Parsing** - Extract Spotify URLs from messages (Phase 3)
- ⏳ **Track Saving** - Save tracks to user's library (Phase 3)

## Tech Stack

- **Language:** Rust (Edition 2024)
- **Web Framework:** Axum 0.8
- **Runtime:** Tokio (async)
- **Database:** PostgreSQL with sqlx (compile-time checked queries)
- **OAuth:** oauth2 crate
- **Logging:** tracing + tracing-subscriber

## Prerequisites

- Rust 1.93 or later
- PostgreSQL 14+
- Spotify Developer Account (for OAuth credentials)

## Setup

### 1. Install Dependencies

\`\`\`bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install sqlx-cli
cargo install sqlx-cli --no-default-features --features postgres

# Install PostgreSQL (macOS)
brew install postgresql@14
brew services start postgresql@14
\`\`\`

### 2. Create Spotify App

1. Go to [Spotify Developer Dashboard](https://developer.spotify.com/dashboard)
2. Create a new app
3. Add redirect URI: \`http://127.0.0.1:3000/spotify/callback\`
4. Note your Client ID and Client Secret

### 3. Database Setup

\`\`\`bash
# Create database
createdb savethebeat

# Run migrations
sqlx migrate run
\`\`\`

### 4. Environment Configuration

Create a \`.env\` file in the project root:

\`\`\`bash
# Server
PORT=3000
HOST=0.0.0.0
RUST_LOG=info,savethebeat=debug
RUST_LOG_FORMAT=pretty

# Database
DATABASE_URL=postgresql://localhost/savethebeat

# Spotify OAuth
SPOTIFY_CLIENT_ID=your_spotify_client_id
SPOTIFY_CLIENT_SECRET=your_spotify_client_secret
SPOTIFY_REDIRECT_URI=http://127.0.0.1:3000/spotify/callback
BASE_URL=http://127.0.0.1:3000

# Slack (Phase 2+)
# SLACK_SIGNING_SECRET=
# SLACK_BOT_TOKEN=
\`\`\`

### 5. Run the Application

\`\`\`bash
# Development mode
cargo run

# Release mode
cargo run --release
\`\`\`

The server will start on \`http://127.0.0.1:3000\`

## API Endpoints

### Health Check
\`\`\`
GET /health
\`\`\`

Returns server status and version.

### Spotify OAuth Flow

#### 1. Initiate Connection
\`\`\`
GET /spotify/connect?slack_workspace_id=<WORKSPACE_ID>&slack_user_id=<USER_ID>
\`\`\`

Redirects to Spotify for authorization.

#### 2. OAuth Callback
\`\`\`
GET /spotify/callback?code=<CODE>&state=<STATE>
\`\`\`

Handles Spotify OAuth callback, exchanges code for tokens, stores in database.

## Development

### Run Tests

\`\`\`bash
# All tests
cargo test

# With output
cargo test -- --nocapture

# Specific test
cargo test test_name
\`\`\`

### Code Quality

\`\`\`bash
# Format code
cargo fmt

# Lint
cargo clippy --all-targets --all-features -- -D warnings

# Check without building
cargo check
\`\`\`

### Database Migrations

\`\`\`bash
# Create new migration
sqlx migrate add migration_name

# Run migrations
sqlx migrate run

# Revert last migration
sqlx migrate revert

# Generate sqlx offline cache (for CI)
DATABASE_URL=postgresql://localhost/savethebeat cargo sqlx prepare
\`\`\`

## Project Structure

\`\`\`
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
│   │   └── routes.rs       # HTTP handlers
│   └── routes/
│       └── mod.rs          # Route aggregation
├── migrations/             # SQL migration files
├── templates/             # HTML templates
├── .sqlx/                # sqlx offline query cache
├── Cargo.toml            # Dependencies
└── .env                  # Environment variables (not in git)
\`\`\`

## CI/CD

GitHub Actions workflow (\`.github/workflows/ci.yml\`):
- ✅ Format check (\`cargo fmt\`)
- ✅ Clippy lints (\`cargo clippy\`)
- ✅ Tests with PostgreSQL (\`cargo test\`)
- ✅ Release build (\`cargo build --release\`)

## Documentation

- **[PLAN.md](./PLAN.md)** - Detailed implementation plan and roadmap
- **[AGENTS.md](./AGENTS.md)** - Coding standards and best practices

## Contributing

This is a personal project, but issues and suggestions are welcome!

### Workflow

All changes go through Pull Requests:
1. Create feature branch: \`git checkout -b feature/your-feature\`
2. Make changes and commit
3. Push branch: \`git push -u origin feature/your-feature\`
4. Create PR via GitHub CLI or web interface
5. Wait for review and CI checks

## Current Status

**Phase 1 Progress:** 5/8 sub-phases complete

- ✅ Phase 1.1: Database Setup
- ✅ Phase 1.2: Repository Layer
- ✅ Phase 1.3: OAuth Infrastructure
- ✅ Phase 1.4: Spotify Connect Endpoint
- ✅ Phase 1.5: Spotify Callback Endpoint
- ⏳ Phase 1.6: Token Refresh Helper
- ✅ Phase 1.7: Application State Setup
- ⏳ Phase 1.8: Testing & Verification

**What works now:**
- Users can complete OAuth flow to connect Spotify
- Tokens are stored securely in PostgreSQL
- CSRF protection via state tokens
- Comprehensive error handling and logging

**Next steps:**
- Implement token refresh logic
- End-to-end testing
- Slack integration

## License

MIT License - see LICENSE file for details

## Author

Renato Oliveira
