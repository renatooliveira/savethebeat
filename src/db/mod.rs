pub mod models;
pub mod repository;

use sqlx::{PgPool, postgres::PgPoolOptions};

pub async fn init_pool(database_url: &str) -> anyhow::Result<PgPool> {
    tracing::info!("Initializing database connection pool");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    tracing::info!("Running database migrations");
    sqlx::migrate!().run(&pool).await?;
    tracing::info!("Database migrations completed successfully");

    tracing::info!("Database connection pool initialized successfully");

    Ok(pool)
}
