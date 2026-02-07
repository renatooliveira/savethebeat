pub mod config;
pub mod db;
pub mod error;
pub mod routes;
pub mod spotify;
pub mod telemetry;

use axum::Router;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use tower_http::trace::TraceLayer;

pub async fn run(config: config::Config) -> anyhow::Result<()> {
    telemetry::init_tracing(&config.rust_log);

    // Initialize Spotify OAuth state
    let spotify_state = spotify::routes::SpotifyState {
        oauth_client: spotify::oauth::build_oauth_client(&config),
        state_store: Arc::new(RwLock::new(HashMap::new())),
    };

    tracing::info!("Initialized Spotify OAuth client");

    // Build application router
    let spotify_router = routes::spotify_routes().with_state(spotify_state);

    let app = Router::new()
        .merge(routes::routes())
        .merge(spotify_router)
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
