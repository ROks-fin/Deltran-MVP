use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;

pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await?;

    Ok(pool)
}

pub async fn health_check(pool: &PgPool) -> Result<(), std::io::Error> {
    match sqlx::query("SELECT 1").execute(pool).await {
        Ok(_) => Ok(()),
        Err(e) => Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Database health check failed: {}", e),
        )),
    }
}
