pub mod models;
pub mod repositories;

use crate::core::error::{AppError, AppResult};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;
use std::path::Path;
use std::str::FromStr;
use tracing::info;

/// Initializes and returns an optimized SQLite connection pool, executing pending migrations.
pub async fn init_db_pool<P: AsRef<Path>>(db_path: P) -> AppResult<SqlitePool> {
    let path = db_path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let conn_str = format!("sqlite://{}", path.to_string_lossy());
    info!(database = %conn_str, "Opening SQLite database");

    let options = SqliteConnectOptions::from_str(&conn_str)?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .busy_timeout(std::time::Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(10)
        .connect_with(options)
        .await
        .map_err(|e| AppError::Database(format!("Failed to connect to SQLite: {}", e)))?;

    info!("Running pending database migrations");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| AppError::Database(format!("Migration failed: {}", e)))?;

    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS sync_tombstones (
            id TEXT PRIMARY KEY NOT NULL,
            user_id TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            entity_id TEXT NOT NULL,
            deleted_at INTEGER NOT NULL
        );"
    )
    .execute(&pool)
    .await;

    info!("Database initialized and migrations applied successfully");
    Ok(pool)
}

/// Creates an in-memory SQLite pool for testing and ephemeral execution.
pub async fn create_in_memory_pool() -> AppResult<SqlitePool> {
    let options = SqliteConnectOptions::from_str("sqlite::memory:")?
        .foreign_keys(true)
        .busy_timeout(std::time::Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .map_err(|e| AppError::Database(format!("Failed to create in-memory database: {}", e)))?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| AppError::Database(format!("In-memory migration failed: {}", e)))?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS sync_tombstones (
            id TEXT PRIMARY KEY NOT NULL,
            user_id TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            entity_id TEXT NOT NULL,
            deleted_at INTEGER NOT NULL
        );"
    )
    .execute(&pool)
    .await
    .map_err(|e| AppError::Database(format!("Failed to ensure sync_tombstones: {}", e)))?;

    Ok(pool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_db_and_migrations() {
        let pool = create_in_memory_pool().await.expect("pool creation");
        let row: (i64,) = sqlx::query_as("SELECT count(*) FROM library_folders")
            .fetch_one(&pool)
            .await
            .expect("query execution");
        assert_eq!(row.0, 0);
    }
}
