# Agent Guidelines for savethebeat

This document provides coding standards and best practices for AI agents and developers working on the savethebeat project.

---

## ⚠️ CRITICAL: Workflow Rules

**ALL changes MUST go through Pull Requests. NO direct pushes to main.**

1. **Create a feature branch** for your changes
2. **Commit your changes** to the feature branch
3. **Push the branch** to GitHub
4. **Create a Pull Request** using `gh` CLI or GitHub web interface
5. **Assign @renatooliveira as reviewer** (REQUIRED)
6. **Wait for approval** before merging

**Example workflow:**
```bash
# Create feature branch
git checkout -b feature/phase-1-2-repository

# Make changes and commit
git add .
git commit -m "Implement Phase 1.2: Repository layer"

# Push branch
git push -u origin feature/phase-1-2-repository

# Create PR with reviewer assigned
gh pr create --title "Implement Phase 1.2: Repository layer" \
             --body "Add CRUD operations for user_auth table" \
             --assignee renatooliveira \
             --reviewer renatooliveira
```

**This rule applies to:**
- ✅ Code changes
- ✅ Documentation updates
- ✅ Configuration changes
- ✅ Database migrations
- ✅ Everything else

**Documentation updates (update whenever necessary):**
- Update `PLAN.md` to mark completed tasks with ✅
- Update `AGENTS.md` if new patterns or practices are introduced
- Update `README.md` if setup instructions change
- Add inline code documentation (`///`) for new public functions
- Update this file's "Last Updated" section when significant changes are made

**No direct pushes to main. All changes via PR.**

---

## Project Context

**savethebeat** is a Slack ↔ Spotify integration bot built with Rust, using:
- **Framework:** Axum 0.8 (async web server)
- **Runtime:** Tokio (async runtime)
- **Database:** PostgreSQL with sqlx (compile-time checked queries)
- **Edition:** Rust 2024

**Core Functionality:** Users mention the bot in Slack threads containing Spotify track links, and the bot saves those tracks to the user's Spotify library.

---

## General Rust Best Practices

### Code Style

1. **Follow Rust idioms:**
   - Use `?` operator for error propagation
   - Prefer `impl Trait` over concrete types in function signatures when appropriate
   - Use pattern matching instead of multiple if-let chains
   - Prefer iterator chains over explicit loops

2. **Formatting:**
   - Always run `cargo fmt` before committing
   - Use default rustfmt settings (no custom config)

3. **Linting:**
   - Run `cargo clippy` and fix all warnings
   - Treat clippy warnings as errors in CI

4. **Naming conventions:**
   - Types: `PascalCase` (e.g., `UserAuth`, `OAuthState`)
   - Functions/variables: `snake_case` (e.g., `get_user_auth`, `access_token`)
   - Constants: `SCREAMING_SNAKE_CASE` (e.g., `MAX_RETRIES`, `DEFAULT_TIMEOUT`)
   - Lifetimes: single lowercase letter (e.g., `'a`, `'b`)

### Error Handling

1. **Use appropriate error types:**
   - Application-level: `anyhow::Result<T>` for quick error propagation
   - Library-level: Custom errors with `thiserror` for typed errors
   - Don't use `unwrap()` or `expect()` in production code (tests are OK)

2. **Error context:**
   - Add context with `.context()` or `.with_context()` from anyhow
   - Example: `.context("Failed to connect to database")?`

3. **HTTP error responses:**
   - Convert errors to appropriate HTTP status codes
   - Never leak sensitive information in error responses
   - Log detailed errors, return generic messages to users

### Async/Await

1. **Async functions:**
   - Use `async fn` for async operations
   - Prefer `async move` in closures when capturing owned values

2. **Tokio patterns:**
   - Use `tokio::spawn` for background tasks
   - Use `tokio::select!` for concurrent operations
   - Avoid blocking operations in async context (use `spawn_blocking` if needed)

3. **Database operations:**
   - All sqlx operations are async
   - Use connection pooling (PgPool) - never create connections manually
   - Prefer transactions for multi-step operations

---

## Project-Specific Conventions

### Module Organization

```
src/
├── main.rs              # Minimal entry point - just calls lib::run()
├── lib.rs               # App orchestration - initialize state, build router
├── config.rs            # Config struct with environment variable loading
├── error.rs             # Custom error types with IntoResponse implementations
├── telemetry.rs         # Tracing/logging initialization
├── db/
│   ├── mod.rs          # DB pool initialization
│   ├── models.rs       # Database models (FromRow, Serialize)
│   └── repository.rs   # Data access layer (CRUD functions)
├── spotify/
│   ├── mod.rs          # Module exports
│   ├── oauth.rs        # OAuth client setup, state management
│   ├── client.rs       # Spotify API client, token refresh
│   └── routes.rs       # HTTP route handlers for /spotify/*
└── routes/
    └── mod.rs          # Route aggregation and /health endpoint
```

**Rules:**
- Each module should have a clear, single responsibility
- Re-export public items through parent `mod.rs`
- Keep route handlers thin - delegate to service functions
- Database logic stays in `db/repository.rs`

### Database Patterns (sqlx)

1. **Queries:**
   - Use `sqlx::query!()` macro for compile-time verification
   - Use `sqlx::query_as!()` when mapping to structs
   - Never use string interpolation - always use parameter binding

   ```rust
   // Good
   sqlx::query_as!(
       UserAuth,
       "SELECT * FROM user_auth WHERE slack_workspace_id = $1 AND slack_user_id = $2",
       workspace_id,
       user_id
   )
   .fetch_optional(&pool)
   .await?

   // Bad - SQL injection risk
   let query = format!("SELECT * FROM user_auth WHERE id = '{}'", id);
   ```

2. **Transactions:**
   - Use transactions for multi-step operations that must be atomic
   - Always commit or rollback explicitly

   ```rust
   let mut tx = pool.begin().await?;

   // Multiple operations...
   sqlx::query!("INSERT INTO ...").execute(&mut *tx).await?;
   sqlx::query!("UPDATE ...").execute(&mut *tx).await?;

   tx.commit().await?;
   ```

3. **Models:**
   - Derive `FromRow` for database models
   - Derive `Serialize` if returned in API responses
   - Match database column names exactly (snake_case)
   - Use `Option<T>` for nullable columns
   - Use `chrono::DateTime<Utc>` for timestamps
   - Use `uuid::Uuid` for UUID columns

### HTTP Handlers (Axum)

1. **Function signature:**
   - Use extractors for request data (State, Query, Json, etc.)
   - Return `Result<impl IntoResponse, AppError>`

   ```rust
   async fn handler(
       State(state): State<AppState>,
       Query(params): Query<QueryParams>,
   ) -> Result<Json<Response>, AppError> {
       // handler logic
       Ok(Json(response))
   }
   ```

2. **State management:**
   - Use `State<AppState>` extractor for shared state
   - AppState should be `Clone` and contain Arc-wrapped resources
   - Pass database pool, OAuth client, etc. via AppState

3. **Response types:**
   - Use `Json<T>` for JSON responses
   - Use `Redirect` for HTTP redirects
   - Use `Html<String>` for HTML responses
   - Implement `IntoResponse` for custom response types

### OAuth & Security

1. **State tokens:**
   - Generate cryptographically secure random tokens (32 bytes)
   - Use `base64::engine::general_purpose::URL_SAFE_NO_PAD` for encoding
   - Store with TTL and remove after use (one-time tokens)

2. **Token storage:**
   - Never log access tokens or refresh tokens
   - Use placeholder strings in logs: `access_token: <redacted>`
   - Store tokens in database (plaintext acceptable for MVP)

3. **Token refresh:**
   - Check token expiry with 5-minute buffer
   - Refresh proactively before expiration
   - Update database immediately after refresh
   - Handle refresh failures gracefully

### Logging & Tracing

1. **Use structured logging:**
   ```rust
   tracing::info!(
       slack_workspace_id = %workspace_id,
       slack_user_id = %user_id,
       "Starting OAuth flow"
   );
   ```

2. **Log levels:**
   - `error!`: Critical failures requiring attention
   - `warn!`: Unexpected but handled conditions
   - `info!`: Significant events (OAuth flow, token refresh, API calls)
   - `debug!`: Detailed debugging information
   - `trace!`: Very verbose debugging

3. **What to log:**
   - API request/response status (not bodies with sensitive data)
   - Database operations (query type, duration)
   - OAuth flow steps
   - Token refresh events
   - Error conditions with context

4. **What NOT to log:**
   - Access tokens or refresh tokens
   - API secrets or passwords
   - Full message bodies from Slack
   - User credentials

### Testing

1. **Unit tests:**
   - Place in same file as code under `#[cfg(test)] mod tests`
   - Test pure functions and business logic
   - Use descriptive test names: `test_state_token_generation_creates_unique_tokens`

2. **Integration tests:**
   - Use `#[sqlx::test]` macro for database tests
   - Each test gets a fresh database transaction
   - Mock external APIs (Spotify, Slack) in tests

   ```rust
   #[sqlx::test]
   async fn test_get_user_auth(pool: PgPool) -> sqlx::Result<()> {
       // Test implementation
       Ok(())
   }
   ```

3. **Test organization:**
   - Unit tests: in `src/` files
   - Integration tests: in `tests/` directory
   - Test utilities: in `tests/common/mod.rs`

---

## Common Patterns

### Repository Functions

```rust
// Get single record (returns Option)
pub async fn get_user_auth(
    pool: &PgPool,
    workspace_id: &str,
    user_id: &str,
) -> Result<Option<UserAuth>, sqlx::Error> {
    sqlx::query_as!(
        UserAuth,
        "SELECT * FROM user_auth
         WHERE slack_workspace_id = $1 AND slack_user_id = $2",
        workspace_id,
        user_id
    )
    .fetch_optional(pool)
    .await
}

// Upsert (insert or update)
pub async fn upsert_user_auth(
    pool: &PgPool,
    workspace_id: &str,
    user_id: &str,
    // ... other params
) -> Result<UserAuth, sqlx::Error> {
    sqlx::query_as!(
        UserAuth,
        "INSERT INTO user_auth (slack_workspace_id, slack_user_id, ...)
         VALUES ($1, $2, ...)
         ON CONFLICT (slack_workspace_id, slack_user_id)
         DO UPDATE SET access_token = $3, updated_at = NOW()
         RETURNING *",
        workspace_id,
        user_id,
        // ... other args
    )
    .fetch_one(pool)
    .await
}
```

### Error Handling in Handlers

```rust
async fn handler(
    State(state): State<AppState>,
) -> Result<Json<Response>, AppError> {
    let user = get_user_auth(&state.db, "T123", "U456")
        .await
        .context("Failed to fetch user from database")?
        .ok_or_else(|| anyhow::anyhow!("User not found"))?;

    // Use the user...

    Ok(Json(response))
}
```

### State Management

```rust
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub oauth_state: StateStore,
    pub oauth_client: BasicClient,
}

// In lib.rs
let state = AppState {
    db: init_pool(&config.database_url).await?,
    oauth_state: Arc::new(RwLock::new(HashMap::new())),
    oauth_client: build_oauth_client(&config),
};

let app = Router::new()
    .route("/health", get(health))
    .merge(spotify_routes())
    .with_state(state);
```

---

## Code Review Checklist

Before submitting code, ensure:

- [ ] `cargo fmt` has been run
- [ ] `cargo clippy` passes with no warnings
- [ ] `cargo test` passes
- [ ] No `unwrap()` or `expect()` in production code
- [ ] Error messages are user-friendly (no stack traces in responses)
- [ ] Sensitive data is not logged (tokens, secrets)
- [ ] Database queries use parameter binding (no SQL injection)
- [ ] Functions have clear, single responsibilities
- [ ] Complex logic has unit tests
- [ ] Public functions have doc comments (`///`)
- [ ] Async operations use proper error handling
- [ ] Database operations use connection pool, not individual connections

---

## Git Commit Practices

1. **Commit messages:**
   - Use imperative mood: "Add feature" not "Added feature"
   - First line: brief summary (50 chars or less)
   - Blank line, then detailed description if needed
   - Reference issue numbers if applicable

2. **Commit organization:**
   - Each commit should be a logical unit of work
   - Don't mix unrelated changes
   - Squash WIP commits before pushing

3. **Branch strategy:**
   - `main`: stable, working code
   - Feature branches: `feature/oauth-flow`, `fix/token-refresh`
   - **NEVER push directly to main**

4. **Pull Request Workflow (REQUIRED):**
   - **Always create a PR for all changes**
   - Branch from main: `git checkout -b feature/your-feature`
   - Make your commits on the feature branch
   - Push branch: `git push -u origin feature/your-feature`
   - Create PR via GitHub CLI or web interface
   - **Always assign @renatooliveira as reviewer**
   - Wait for approval before merging

   **Using GitHub CLI (gh):**
   ```bash
   # After committing changes
   git push -u origin feature/your-feature

   # Create PR and assign reviewer
   gh pr create --title "Your PR title" \
                --body "Description of changes" \
                --assignee renatooliveira \
                --reviewer renatooliveira
   ```

   **Important:** This applies to all code changes, documentation updates, and configuration changes. No exceptions.

---

## Performance Considerations

1. **Database:**
   - Use indexes for frequently queried columns
   - Batch operations when possible
   - Use `EXPLAIN ANALYZE` to check query performance
   - Connection pool size: 5-10 for most use cases

2. **Async:**
   - Don't block the async runtime (use `spawn_blocking` for CPU-intensive work)
   - Use buffered streams for large data sets
   - Cancel background tasks properly

3. **Memory:**
   - Avoid cloning large structs unnecessarily
   - Use references when possible
   - Be mindful of lifetime constraints

---

## Documentation

1. **Code comments:**
   - Use `///` for public API documentation
   - Use `//` for inline explanations of complex logic
   - Don't comment obvious code

2. **Doc comments should include:**
   - What the function does
   - Parameters and their constraints
   - Return value meaning
   - Errors that can occur
   - Examples for complex APIs

   ```rust
   /// Refreshes an expired Spotify access token.
   ///
   /// # Arguments
   /// * `pool` - Database connection pool
   /// * `oauth_client` - Configured OAuth2 client
   /// * `user_auth` - User authentication record with refresh token
   ///
   /// # Returns
   /// The new access token if refresh succeeds
   ///
   /// # Errors
   /// Returns error if:
   /// - Refresh token is invalid
   /// - Network request fails
   /// - Database update fails
   pub async fn refresh_access_token(
       pool: &PgPool,
       oauth_client: &BasicClient,
       user_auth: &UserAuth,
   ) -> Result<String> {
       // implementation
   }
   ```

---

## Anti-Patterns to Avoid

1. **Don't use `.unwrap()` in production:**
   ```rust
   // Bad
   let config = Config::from_env().unwrap();

   // Good
   let config = Config::from_env()
       .context("Failed to load configuration")?;
   ```

2. **Don't ignore errors:**
   ```rust
   // Bad
   let _ = save_to_db(&pool, data).await;

   // Good
   save_to_db(&pool, data).await
       .context("Failed to save data")?;
   ```

3. **Don't create database connections manually:**
   ```rust
   // Bad
   let conn = PgConnection::connect(&url).await?;

   // Good - use the pool
   let result = sqlx::query!("SELECT ...").fetch_one(&pool).await?;
   ```

4. **Don't use string formatting for SQL:**
   ```rust
   // Bad - SQL injection vulnerability
   let query = format!("SELECT * FROM users WHERE id = '{}'", user_id);

   // Good - parameterized query
   sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user_id)
   ```

5. **Don't mix concerns:**
   ```rust
   // Bad - HTTP handler doing database operations directly
   async fn handler() -> Result<Json<Response>> {
       let result = sqlx::query!("SELECT ...").fetch_one(&pool).await?;
       // process result...
   }

   // Good - use repository layer
   async fn handler(State(state): State<AppState>) -> Result<Json<Response>> {
       let user = get_user(&state.db, user_id).await?;
       // process user...
   }
   ```

---

## Quick Reference

### Common Commands

```bash
# Format code
cargo fmt

# Check for errors
cargo check

# Run linter
cargo clippy

# Run tests
cargo test

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Run in debug mode
cargo run

# Run in release mode
cargo run --release

# Database migrations
sqlx migrate run
sqlx migrate revert

# Generate sqlx metadata (for CI)
cargo sqlx prepare
```

### Useful Cargo.toml Features

```toml
# Development profile with faster compilation
[profile.dev]
opt-level = 0
debug = true

# Release profile with optimizations
[profile.release]
opt-level = 3
lto = true
codegen-units = 1

# Test profile
[profile.test]
opt-level = 0
```

---

## Resources

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Axum Documentation](https://docs.rs/axum/)
- [sqlx Documentation](https://docs.rs/sqlx/)
- [Tokio Documentation](https://docs.rs/tokio/)
- [OAuth2 Crate](https://docs.rs/oauth2/)

---

**Last Updated:** Phase 1.5 (Spotify OAuth Flow Complete)

**Current Focus:** Token refresh helper (Phase 1.6) and end-to-end testing (Phase 1.8)
