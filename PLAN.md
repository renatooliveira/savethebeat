# savethebeat Implementation Plan

## Project Overview

Slack ↔ Spotify integration bot that allows users to save Spotify tracks to their library by mentioning the bot in Slack threads.

## Multi-Phase Implementation

### Phase 0: Foundations ✅ COMPLETED

**Goal:** Running service with basic infrastructure

**Implemented:**
- Axum 0.8 web server with /health endpoint
- Config system (dotenvy + envy)
- Error handling (AppError)
- Structured logging with tracing
- Rust edition 2024

### Phase 1: Persistence + Spotify Auth 🚧 IN PROGRESS

**Goal:** Connect Slack user to Spotify; tokens stored + refresh works

**Status:** Phases 1.1-1.5, 1.7 Complete ✅ | Phase 1.6, 1.8 Pending

#### Phase 1.1: Database Setup ✅
- [x] Add sqlx, uuid, chrono dependencies
- [x] Add oauth2, reqwest, rand, base64 dependencies
- [x] Create database models (UserAuth, SaveActionLog)
- [x] Create initial migration with user_auth and save_action_log tables
- [x] Update Config to require database_url and Spotify OAuth fields
- [x] DB pool initialization module
- [x] PostgreSQL database created and migrated
- [x] sqlx offline mode setup (.sqlx/ query cache)

**PR:** #2 - Implement Phase 1.2: Repository layer with CRUD operations

#### Phase 1.2: Repository Layer ✅
- [x] Create `src/db/repository.rs` with CRUD operations:
  - [x] `get_user_auth(pool, workspace_id, user_id) -> Result<Option<UserAuth>>`
  - [x] `upsert_user_auth(...) -> Result<UserAuth>`
  - [x] `update_tokens(...) -> Result<()>`
- [x] Comprehensive #[sqlx::test] tests
- [x] Unique constraint handling

**PR:** #2 - Implement Phase 1.2: Repository layer with CRUD operations

#### Phase 1.3: OAuth Infrastructure ✅
- [x] Update `src/error.rs` with OAuth/DB error variants
- [x] Create `src/spotify/mod.rs`
- [x] Create `src/spotify/oauth.rs` with:
  - [x] OAuth client builder (`build_oauth_client`)
  - [x] State store (Arc<RwLock<HashMap>>)
  - [x] State generation (`generate_state_token` - 32 bytes, base64)
  - [x] State validation (`validate_and_consume_state`)
  - [x] CSRF protection (one-time use, 10-minute TTL)
- [x] Unit tests for all OAuth functions

**PR:** #4 - Phase 1.3: OAuth Infrastructure

#### Phase 1.4: Spotify Connect Endpoint ✅
- [x] Create `src/spotify/routes.rs`
- [x] Implement `GET /spotify/connect?slack_workspace_id=X&slack_user_id=Y`
  - [x] Generate and store state token
  - [x] Build Spotify authorization URL
  - [x] Redirect to Spotify with `user-library-modify` scope
- [x] Wire routes in `src/routes/mod.rs`
- [x] Unit tests for connect endpoint

**PR:** #5 - Phase 1.4: Spotify Connect Endpoint

#### Phase 1.5: Spotify Callback Endpoint ✅
- [x] Implement `GET /spotify/callback?code=X&state=Y`
  - [x] Validate and consume state token
  - [x] Exchange authorization code for tokens
  - [x] Calculate token expiry with 5-minute buffer
  - [x] Store tokens in database via `upsert_user_auth`
  - [x] Return styled HTML success page
- [x] Create `templates/spotify_success.html` template
- [x] Use `include_str!` for compile-time template embedding
- [x] Unit tests for callback validation

**PR:** #6 - Phase 1.5: Spotify Callback Endpoint

#### Phase 1.6: Token Refresh Helper ⏳ PENDING
- [ ] Create `src/spotify/client.rs` with:
  - [ ] `refresh_access_token()` - refresh expired tokens
  - [ ] `ensure_valid_token()` - get valid token (refresh if needed)

#### Phase 1.7: Application State Setup ✅
- [x] Update `src/lib.rs` with SpotifyState
- [x] Initialize DB pool, OAuth client, StateStore
- [x] Pass state to routes via `with_state()`
- [x] Wire Spotify routes into main router

**Completed in:** PRs #5 and #6

#### Phase 1.8: Testing & Verification ⏳ PENDING
- [ ] Complete OAuth flow end-to-end
- [ ] Verify token refresh works
- [ ] Test Spotify API calls with tokens
- [ ] All CI checks pass

**Definition of Done:**
- ✅ User can complete Spotify OAuth flow (/connect → Spotify → /callback)
- ✅ Tokens persist in database across restarts (user_auth table)
- ⏳ Expired tokens automatically refresh (Phase 1.6 pending)
- ⏳ Spotify API call succeeds with refreshed token (Phase 1.8 pending)

**Completed:**
- OAuth flow fully functional (connect + callback)
- State management with CSRF protection
- Database persistence with upsert logic
- Token expiry tracking with 5-minute buffer
- Styled HTML success page

**Remaining:**
- Token refresh functionality
- End-to-end testing with real Spotify API

---

### Phase 2: Slack Events + Thread Resolution

**Goal:** Receive `app_mention` and fetch thread messages

**Tasks:**
1. Implement `POST /slack/events` with signature verification
2. Handle Slack `url_verification`
3. Handle `event_callback` for `app_mention`
4. Extract event metadata
5. Implement Slack client for `conversations.replies`

**DoD:**
- Slack endpoint passes verification
- Bot receives mention events
- Thread messages can be fetched

---

### Phase 3: Parse + Save + Feedback (MVP Core)

**Goal:** Mention in thread saves first Spotify track and confirms

**Tasks:**
1. Implement Spotify link parser (unit tests)
2. Implement "first link selection" logic
3. Implement idempotency (unique constraint handling)
4. Implement Spotify save to Liked Songs
5. Implement Slack reactions (✅/♻️/❌)
6. Implement ephemeral error messages

**DoD:**
- Replying `@bot` in thread saves the track
- Repeat mention yields ♻️ (no duplicate save)
- Failures produce ❌ + actionable error message

---

### Phase 4: Retries + Rate Limiting + Observability

**Goal:** Stable operation under real API conditions

**Tasks:**
1. Ensure Slack handler ACKs quickly (async job)
2. Implement retry policy for Spotify (401/429/5xx)
3. Add per-user rate limiting
4. Improve logs (correlation ID, outcome, latency)

**DoD:**
- No Slack timeouts
- Transient failures typically succeed
- Logs allow tracing mention to outcome

---

### Phase 5: Testing & Rollout

**Goal:** Confidence and safe release

**Tasks:**
1. Integration tests
2. Document local dev steps
3. Manual test checklist

**DoD:**
- CI runs tests reliably
- Human can run end-to-end locally

---

## Database Schema

### user_auth
Stores Spotify OAuth tokens with auto-refresh support

```sql
CREATE TABLE user_auth (
    id UUID PRIMARY KEY,
    slack_workspace_id TEXT NOT NULL,
    slack_user_id TEXT NOT NULL,
    spotify_user_id TEXT,
    access_token TEXT NOT NULL,
    refresh_token TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    paused BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_user_auth_slack
    ON user_auth(slack_workspace_id, slack_user_id);
```

### save_action_log
Tracks Spotify track save operations with deduplication

```sql
CREATE TABLE save_action_log (
    id UUID PRIMARY KEY,
    slack_workspace_id TEXT NOT NULL,
    slack_user_id TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    thread_ts TEXT NOT NULL,
    mention_ts TEXT NOT NULL,
    spotify_track_id TEXT NOT NULL,
    status TEXT CHECK (status IN ('saved', 'already_saved', 'failed')),
    error_code TEXT,
    error_message TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_save_log_unique
    ON save_action_log(slack_workspace_id, slack_user_id,
                       thread_ts, spotify_track_id);
```

---

## OAuth Flow (Phase 1)

### Connect
```
GET /spotify/connect?slack_workspace_id=T123&slack_user_id=U456
→ Generate state token
→ Store state: {workspace: T123, user: U456, created: now}
→ Redirect to Spotify with state
```

### Callback
```
GET /spotify/callback?code=xyz&state=abc
→ Validate state (exists, not expired)
→ Exchange code for tokens
→ Store tokens in user_auth
→ Return success page
```

### Token Refresh
```
ensure_valid_token(pool, oauth_client, "T123", "U456")
→ Fetch user_auth
→ If expired: refresh and update DB
→ Return valid access_token
```

---

## Environment Variables

```bash
# Server
PORT=3000
HOST=0.0.0.0
RUST_LOG=info,savethebeat=debug
RUST_LOG_FORMAT=pretty

# Database (Phase 1+)
DATABASE_URL=postgresql://localhost/savethebeat

# Spotify OAuth (Phase 1+)
SPOTIFY_CLIENT_ID=...
SPOTIFY_CLIENT_SECRET=...
SPOTIFY_REDIRECT_URI=http://127.0.0.1:3000/spotify/callback
BASE_URL=http://127.0.0.1:3000

# Slack (Phase 2+)
SLACK_SIGNING_SECRET=...
SLACK_BOT_TOKEN=...
```

---

## Development Setup

### Prerequisites
- Rust 1.93+ (edition 2024)
- PostgreSQL 14+
- sqlx-cli

### Initial Setup
```bash
# Install dependencies
cargo install sqlx-cli --no-default-features --features postgres

# Create database
createdb savethebeat

# Run migrations
sqlx migrate run

# Set up environment
cp .env.example .env
# Edit .env with your Spotify credentials

# Run the service
cargo run
```

### CI Commands
```bash
cargo fmt
cargo clippy
cargo test
cargo run
```

---

## Architecture Decisions

### OAuth State Management
**Choice:** In-memory HashMap with Arc<RwLock>

**Rationale:**
- State tokens are ephemeral (5-10 min TTL)
- No need for DB persistence
- Faster than DB lookups
- Simpler for MVP

### Token Security (MVP)
**Choice:** Plaintext tokens in database

**Acceptable because:**
- Database access is restricted
- PostgreSQL connection uses SSL in production
- Simpler for initial development

**Future:** Application-level encryption

### Repository Pattern
**Choice:** Separate data access layer

**Rationale:**
- Cleaner separation of concerns
- Easier to test
- Can swap implementations if needed

---

## Security Considerations

**Implemented:**
- ✅ CSRF protection via state tokens (32 bytes random, one-time use)
- ✅ State TTL (check expiry on validation)
- ✅ Secrets in environment variables
- ✅ Token refresh buffer (5min before expiry)

**Acceptable for MVP:**
- Tokens stored plaintext in DB (DB access restricted)
- In-memory state store (single-instance deployment)

**Future Enhancements:**
- Encrypt tokens at application level
- Redis-backed state store for horizontal scaling
- PKCE flow (OAuth 2.1)
- Token revocation endpoint

---

## Module Structure

```
src/
├── main.rs              # Entry point
├── lib.rs               # App orchestration
├── config.rs            # Configuration
├── error.rs             # Error handling
├── telemetry.rs         # Logging/tracing
├── db/
│   ├── mod.rs          # Pool initialization
│   ├── models.rs       # Database models
│   └── repository.rs   # CRUD operations
├── spotify/
│   ├── mod.rs          # Module exports
│   ├── oauth.rs        # OAuth flow
│   ├── client.rs       # API client
│   └── routes.rs       # Endpoints
└── routes/
    └── mod.rs          # Route aggregation

migrations/
└── 20260206000001_initial_schema.sql
```

---

## Testing Strategy

### Unit Tests
- State token generation/validation
- Spotify link parsing
- Token expiry calculation

### Integration Tests
- Repository CRUD operations (use `#[sqlx::test]` macro)
- OAuth flow (with test database)

### Manual Testing
1. Complete OAuth flow
2. Verify tokens in database
3. Manually expire token
4. Trigger refresh
5. Verify Spotify API call succeeds

---

## Future Enhancements (Post-MVP)

- Durable queue (apalis/redis) for background jobs
- Playlist routing (save to specific playlists)
- `@bot 2` / "nth link selection"
- Thread-first-link caching via Redis
- Multi-workspace install OAuth
- Horizontal scaling support
- Application-level token encryption
