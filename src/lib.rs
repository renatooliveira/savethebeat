pub mod config;
pub mod db;
pub mod error;
pub mod routes;
pub mod slack;
pub mod spotify;
pub mod telemetry;

use axum::Router;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tower_http::trace::TraceLayer;

pub async fn run(config: config::Config) -> anyhow::Result<()> {
    telemetry::init_tracing(&config.rust_log);

    // Initialize database connection pool
    let db = db::init_pool(&config.database_url).await?;
    tracing::info!("Database connection pool initialized");

    // Initialize Spotify OAuth client
    let oauth_client = spotify::oauth::build_oauth_client(&config);
    tracing::info!("Initialized Spotify OAuth client");

    // Initialize Spotify OAuth state
    let spotify_state = spotify::routes::SpotifyState {
        oauth_client: oauth_client.clone(),
        state_store: Arc::new(RwLock::new(HashMap::new())),
        db: db.clone(),
    };

    // Build application router
    let spotify_router = routes::spotify_routes().with_state(spotify_state);

    let mut app = Router::new().merge(routes::routes()).merge(spotify_router);

    // Add Slack routes if configured
    if let (Some(signing_secret), Some(bot_token)) =
        (&config.slack_signing_secret, &config.slack_bot_token)
    {
        let slack_state = slack::routes::SlackState {
            signing_secret: signing_secret.clone(),
            bot_token: bot_token.clone(),
            db: db.clone(),
            oauth_client: oauth_client.clone(),
            base_url: config.base_url.clone(),
        };

        let slack_router = routes::slack_routes().with_state(slack_state);
        app = app.merge(slack_router);
        tracing::info!("Initialized Slack event handler");
    } else {
        tracing::warn!("Slack credentials not configured, skipping Slack integration");
    }

    let app = app.layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
