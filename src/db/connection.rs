use crate::config::Config;
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;

pub type DatabasePool = PgPool;

#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Database connection failed: {0}")]
    ConnectionFailed(#[from] sqlx::Error),
    #[error("Database migration failed: {0}")]
    MigrationFailed(#[from] sqlx::migrate::MigrateError),
    #[error("Database operation failed: {0}")]
    OperationFailed(sqlx::Error),
}

pub async fn create_connection_pool(config: &Config) -> Result<DatabasePool, DatabaseError> {
    tracing::info!("Creating database connection pool");

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .min_connections(2)
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .connect(&config.database_url)
        .await
        .map_err(DatabaseError::ConnectionFailed)?;

    tracing::info!("Database connection pool created successfully");
    Ok(pool)
}

pub async fn run_migrations(pool: &DatabasePool) -> Result<(), DatabaseError> {
    tracing::info!("Running database migrations");

    sqlx::migrate!("./migrations").run(pool).await?;

    tracing::info!("Database migrations completed successfully");
    Ok(())
}

pub async fn health_check(pool: &DatabasePool) -> Result<(), DatabaseError> {
    sqlx::query("SELECT 1")
        .execute(pool)
        .await
        .map_err(DatabaseError::OperationFailed)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_health_check() {
        // This test requires a running PostgreSQL instance
        // Skip in CI/CD environments that don't have database setup
        if std::env::var("DATABASE_URL").is_err() {
            return;
        }

        let config = Config::from_env().expect("Failed to load config");
        let pool = create_connection_pool(&config)
            .await
            .expect("Failed to create pool");

        health_check(&pool).await.expect("Health check failed");
    }
}
