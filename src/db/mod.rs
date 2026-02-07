pub mod models;

use sqlx::{postgres::PgPoolOptions, PgPool};

pub async fn init_pool(database_url: &str) -> anyhow::Result<PgPool> {
    tracing::info!("Initializing database connection pool");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    tracing::info!("Database connection pool initialized successfully");

    Ok(pool)
}
