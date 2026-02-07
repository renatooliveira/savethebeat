pub mod config;
pub mod error;
pub mod routes;
pub mod telemetry;

use axum::Router;
use std::net::SocketAddr;
use tower_http::trace::TraceLayer;

pub async fn run(config: config::Config) -> anyhow::Result<()> {
    telemetry::init_tracing(&config.rust_log);

    let app = Router::new()
        .merge(routes::routes())
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
