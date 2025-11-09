use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use crate::config::DatabaseConfig;
use crate::errors::Result;
use tracing::info;

pub type DbPool = Pool<Postgres>;

pub async fn create_pool(config: &DatabaseConfig) -> Result<DbPool> {
    info!("Connecting to database...");

    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .connect(&config.url)
        .await?;

    info!("Database connection pool created successfully");

    // Test the connection
    sqlx::query("SELECT 1")
        .fetch_one(&pool)
        .await?;

    info!("Database connection verified");

    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Only run with database available
    async fn test_database_connection() {
        let config = DatabaseConfig {
            url: "postgresql://deltran:deltran_secure_pass_2024@localhost:5432/deltran".to_string(),
            max_connections: 5,
            min_connections: 2,
        };

        let pool = create_pool(&config).await;
        assert!(pool.is_ok());
    }
}
