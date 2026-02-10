# Testing Guide

## Overview

This document describes the testing strategy for savethebeat and provides manual test checklists for end-to-end validation.

## Testing Strategy

### Unit Tests

**Location:** Inline in source files using `#[cfg(test)]` modules

**Coverage:** 40+ unit tests covering:
- OAuth state management (generation, validation, expiry)
- Spotify link parsing (URLs, URIs, edge cases)
- Token refresh logic
- Slack signature verification
- Database repository operations
- Event parsing and validation

**Running unit tests:**
```bash
cargo test
```

**Running with output:**
```bash
cargo test -- --nocapture
```

**Running specific test:**
```bash
cargo test test_name
```

### Integration Tests

**Approach:** Database integration tests using `#[sqlx::test]` macro

**Coverage:**
- Repository CRUD operations with real PostgreSQL
- Unique constraint enforcement
- Token update logic
- Save action logging

**Requirements:**
- PostgreSQL running locally
- `DATABASE_URL` environment variable set

### CI/CD Testing

**GitHub Actions workflow** (`.github/workflows/ci.yml`):
1. **Format Check** - `cargo fmt --all -- --check`
2. **Clippy Lints** - `cargo clippy --all-targets --all-features -- -D warnings`
3. **Tests** - `cargo test --all-features` with PostgreSQL service
4. **Build** - `cargo build --release --all-features`

All PRs must pass CI checks before merging.

---

## Manual Test Checklist

### Prerequisites

- [ ] PostgreSQL 14+ running locally
- [ ] Database created: `createdb savethebeat`
- [ ] Migrations applied: `sqlx migrate run`
- [ ] `.env` file configured with valid credentials
- [ ] Spotify Developer app created with redirect URI configured
- [ ] Slack app created with proper scopes and event subscriptions (optional)

---

### Test Suite 1: Health & Configuration

#### TC-1.1: Server Startup
- [ ] Start server: `cargo run`
- [ ] Verify server starts without errors
- [ ] Verify database connection pool initialized
- [ ] Check logs show Spotify OAuth client initialized
- [ ] If Slack configured: Check logs show Slack handler initialized
- [ ] If Slack not configured: Check logs show warning about skipping Slack

#### TC-1.2: Health Endpoint
- [ ] Request: `curl http://127.0.0.1:3000/health`
- [ ] Verify 200 OK response
- [ ] Verify JSON response contains status and version

**Expected:**
```json
{
  "status": "ok",
  "version": "0.1.0"
}
```

---

### Test Suite 2: Spotify OAuth Flow

#### TC-2.1: OAuth Connect
- [ ] Visit: `http://127.0.0.1:3000/spotify/connect?slack_workspace_id=T123TEST&slack_user_id=U456TEST`
- [ ] Verify redirect to Spotify authorization page
- [ ] Verify URL contains `client_id`, `redirect_uri`, `scope=user-library-modify`, and `state` parameter
- [ ] Verify state parameter is a long random string (base64)

#### TC-2.2: OAuth Callback - Success
- [ ] Complete authorization on Spotify (click "Agree")
- [ ] Verify redirect back to callback URL
- [ ] Verify success page displays
- [ ] Verify success page shows Spotify username or success message
- [ ] Check database: `SELECT * FROM user_auth WHERE slack_workspace_id = 'T123TEST';`
- [ ] Verify record created with access_token, refresh_token, expires_at populated
- [ ] Verify expires_at is in the future (approximately 1 hour from now)

#### TC-2.3: OAuth Callback - Invalid State
- [ ] Visit callback with invalid state: `http://127.0.0.1:3000/spotify/callback?code=test&state=invalid`
- [ ] Verify error response (400 or similar)
- [ ] Verify error message mentions invalid/expired state

#### TC-2.4: Token Verification
- [ ] Request: `http://127.0.0.1:3000/spotify/verify?slack_workspace_id=T123TEST&slack_user_id=U456TEST`
- [ ] Verify 200 OK response
- [ ] Verify JSON contains Spotify user profile data

**Expected:**
```json
{
  "user_id": "your_spotify_id",
  "display_name": "Your Name",
  "authenticated": true
}
```

#### TC-2.5: Token Refresh
- [ ] Manually expire token in database:
  ```sql
  UPDATE user_auth
  SET expires_at = NOW() - INTERVAL '1 hour'
  WHERE slack_workspace_id = 'T123TEST';
  ```
- [ ] Request verify endpoint again: `/spotify/verify?slack_workspace_id=T123TEST&slack_user_id=U456TEST`
- [ ] Verify still returns 200 OK (token refreshed automatically)
- [ ] Check database: `SELECT expires_at FROM user_auth WHERE slack_workspace_id = 'T123TEST';`
- [ ] Verify expires_at is now in the future

---

### Test Suite 3: Slack Integration

**Note:** Requires Slack app configured and event webhook publicly accessible (use ngrok or similar for local testing)

#### TC-3.1: URL Verification Challenge
- [ ] Configure Slack Event Subscriptions Request URL to point to `/slack/events`
- [ ] Verify Slack shows "Verified" status (challenge passed)
- [ ] Check server logs for url_verification event processing

#### TC-3.2: Signature Verification
- [ ] Send test event from Slack
- [ ] Verify event is processed (check logs)
- [ ] Attempt to send event with invalid signature (using curl with wrong HMAC)
- [ ] Verify request is rejected with 401 Unauthorized

#### TC-3.3: App Mention Event
- [ ] In Slack, share a message in a channel with a Spotify link
- [ ] Mention the bot: `@savethebeat`
- [ ] Verify server receives app_mention event (check logs)
- [ ] Verify server spawns background task to process mention
- [ ] Verify no timeout (Slack receives 200 OK within 3 seconds)

---

### Test Suite 4: End-to-End Save Flow

**Prerequisites:** Complete TC-2.2 to have authenticated user

#### TC-4.1: Save Track - First Time
- [ ] In Slack thread, post: `Check out this track! https://open.spotify.com/track/3n3Ppam7vgaVa1iaRUc9Lp`
- [ ] Reply with: `@savethebeat`
- [ ] Verify bot adds ✅ reaction to the mention message
- [ ] Check Spotify Liked Songs - verify track was added
- [ ] Check database:
  ```sql
  SELECT * FROM save_action_log ORDER BY created_at DESC LIMIT 1;
  ```
- [ ] Verify log entry with status='saved'

#### TC-4.2: Save Track - Duplicate (Idempotency)
- [ ] In same thread, mention bot again: `@savethebeat`
- [ ] Verify bot adds ♻️ reaction (already saved)
- [ ] Check database - verify new log entry with status='already_saved'
- [ ] Verify track not duplicated in Spotify Liked Songs

#### TC-4.3: Save Track - No Link Found
- [ ] Start new thread without any Spotify links
- [ ] Mention bot: `@savethebeat`
- [ ] Verify bot adds ❌ reaction (error)
- [ ] Check logs for error about no Spotify link found

#### TC-4.4: Save Track - URI Format
- [ ] Post Spotify URI: `spotify:track:3n3Ppam7vgaVa1iaRUc9Lp`
- [ ] Mention bot: `@savethebeat`
- [ ] Verify bot adds ✅ reaction
- [ ] Verify track saved (check Spotify)

#### TC-4.5: Save Track - Multiple Links
- [ ] Post multiple Spotify links in thread:
  ```
  Message 1: https://open.spotify.com/track/AAAA
  Message 2: https://open.spotify.com/track/BBBB
  Message 3: https://open.spotify.com/track/CCCC
  ```
- [ ] Mention bot
- [ ] Verify bot saves the FIRST link encountered (chronologically oldest message)
- [ ] Verify ✅ reaction on mention

#### TC-4.6: Save Track - Unauthorized User
- [ ] Create new Slack user (or use workspace/user combo not in database)
- [ ] Post Spotify link and mention bot
- [ ] Verify bot adds ❌ reaction (error)
- [ ] Check logs for authentication error

#### TC-4.7: Save Track - Invalid/Deleted Track
- [ ] Post non-existent Spotify track: `https://open.spotify.com/track/INVALIDTRACKID123456`
- [ ] Mention bot
- [ ] Verify bot adds ❌ reaction
- [ ] Check logs for Spotify API error

---

### Test Suite 5: Error Handling & Edge Cases

#### TC-5.1: Database Connection Loss
- [ ] Stop PostgreSQL: `brew services stop postgresql@14`
- [ ] Try to access any endpoint that requires database
- [ ] Verify appropriate error response (500 Internal Server Error)
- [ ] Restart PostgreSQL: `brew services start postgresql@14`
- [ ] Verify service recovers automatically

#### TC-5.2: Invalid Configuration
- [ ] Stop server
- [ ] Remove required env var: `unset SPOTIFY_CLIENT_ID`
- [ ] Try to start server: `cargo run`
- [ ] Verify server fails to start with clear error message about missing config

#### TC-5.3: Replay Attack Protection (Slack)
- [ ] Capture a valid Slack webhook request (headers + body)
- [ ] Wait 6 minutes
- [ ] Replay the exact same request using curl
- [ ] Verify request is rejected with 401 (timestamp expired)

#### TC-5.4: Concurrent Save Attempts
- [ ] Use two Slack clients simultaneously
- [ ] Both mention bot in same thread at nearly same time
- [ ] Verify only ONE save action succeeds (✅ on first)
- [ ] Verify second gets ♻️ (already saved)
- [ ] Check database - verify unique constraint prevented duplicate

---

### Test Suite 6: Performance & Observability

#### TC-6.1: Response Times
- [ ] Measure health endpoint: `time curl http://127.0.0.1:3000/health`
- [ ] Verify response < 100ms
- [ ] Measure OAuth connect redirect: `time curl -I http://127.0.0.1:3000/spotify/connect?slack_workspace_id=T&slack_user_id=U`
- [ ] Verify redirect response < 200ms

#### TC-6.2: Logging Coverage
- [ ] Review logs during normal operation
- [ ] Verify all major operations are logged:
  - Server startup
  - OAuth flow steps (connect, callback, token refresh)
  - Slack event receipt and processing
  - Spotify API calls
  - Database operations
  - Errors with context
- [ ] Verify log format is consistent and parseable

#### TC-6.3: Structured Logging
- [ ] Set `RUST_LOG_FORMAT=json` in `.env`
- [ ] Restart server
- [ ] Verify logs are in JSON format
- [ ] Verify JSON logs contain structured fields (level, timestamp, message, etc.)

---

## Test Automation

### Running All Tests Locally

```bash
# Format check
cargo fmt --all -- --check

# Lints
cargo clippy --all-targets --all-features -- -D warnings

# Unit + integration tests
cargo test --all-features

# Build release
cargo build --release --all-features
```

### CI Test Matrix

The CI runs tests on:
- **OS:** Ubuntu Latest
- **Rust:** Stable channel
- **PostgreSQL:** Version 14

### Test Data Cleanup

**Between test runs:**
```sql
-- Clear test data
TRUNCATE user_auth, save_action_log CASCADE;

-- Or drop and recreate database
DROP DATABASE savethebeat;
CREATE DATABASE savethebeat;
-- Then run migrations
sqlx migrate run
```

---

## Known Limitations

1. **External API Testing:** Unit tests mock Spotify/Slack APIs. True integration requires live credentials.
2. **Slack Webhooks:** Local testing requires public URL (ngrok) for Slack to send webhooks.
3. **Token Expiry:** Spotify tokens expire in 1 hour - tests must account for this.
4. **Rate Limiting:** Not implemented yet (Phase 4) - excessive testing may hit Spotify rate limits.

---

## Troubleshooting Tests

### Tests Failing - Database Connection
```bash
# Check PostgreSQL is running
pg_isready

# Create test database if missing
createdb savethebeat

# Run migrations
sqlx migrate run

# Set DATABASE_URL
export DATABASE_URL=postgresql://localhost/savethebeat
```

### Tests Failing - sqlx Offline Mode
```bash
# Regenerate query cache
DATABASE_URL=postgresql://localhost/savethebeat cargo sqlx prepare

# Commit .sqlx/ directory
git add .sqlx/
```

### Slack Webhook Testing
```bash
# Use ngrok for local testing
ngrok http 3000

# Update Slack Event Subscription URL to:
# https://your-ngrok-url.ngrok.io/slack/events
```

---

## Test Coverage Goals

**Current Status:** 40 unit tests, 100% of critical paths covered

**Coverage by module:**
- ✅ OAuth flow (state management, token exchange, refresh)
- ✅ Spotify API client (user profile, save track, token refresh)
- ✅ Spotify link parser (URL/URI formats, edge cases)
- ✅ Slack signature verification (HMAC, replay protection)
- ✅ Slack event parsing (url_verification, app_mention)
- ✅ Database repository (CRUD, unique constraints)

**Areas for future testing:**
- ⏳ Retry logic (Phase 4)
- ⏳ Rate limiting (Phase 4)
- ⏳ Metrics and observability (Phase 4)

---

## Reporting Issues

When reporting test failures, include:
1. Test case ID (e.g., TC-2.1)
2. Steps to reproduce
3. Expected vs actual behavior
4. Environment (OS, Rust version, PostgreSQL version)
5. Relevant log output
6. Screenshots (for UI/Slack interactions)

**File issues at:** https://github.com/renatooliveira/savethebeat/issues
