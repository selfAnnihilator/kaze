use crate::core::error::{AppError, AppResult};
use crate::database::models::LibraryFolderRecord;
use async_trait::async_trait;
use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

#[async_trait]
pub trait LibraryFolderRepository: Send + Sync {
    async fn get_all(&self) -> AppResult<Vec<LibraryFolderRecord>>;
    async fn add_folder(&self, path: &str) -> AppResult<LibraryFolderRecord>;
    async fn remove_folder(&self, id: &str) -> AppResult<()>;
    async fn update_last_scanned(&self, id: &str, timestamp: i64) -> AppResult<()>;
}

#[derive(Clone)]
pub struct SqliteFolderRepository {
    pool: SqlitePool,
}

impl SqliteFolderRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl LibraryFolderRepository for SqliteFolderRepository {
    async fn get_all(&self) -> AppResult<Vec<LibraryFolderRecord>> {
        let records = sqlx::query_as::<_, LibraryFolderRecord>(
            "SELECT id, path, added_at, last_scanned_at, enabled FROM library_folders WHERE enabled = 1"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to fetch library folders: {}", e)))?;

        Ok(records)
    }

    async fn add_folder(&self, path: &str) -> AppResult<LibraryFolderRecord> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp();

        sqlx::query(
            "INSERT INTO library_folders (id, path, added_at, enabled) VALUES (?, ?, ?, 1)"
        )
        .bind(&id)
        .bind(path)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to insert library folder: {}", e)))?;

        Ok(LibraryFolderRecord {
            id,
            path: path.to_string(),
            added_at: now,
            last_scanned_at: None,
            enabled: 1,
        })
    }

    async fn remove_folder(&self, id: &str) -> AppResult<()> {
        sqlx::query("DELETE FROM library_folders WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to remove library folder: {}", e)))?;

        Ok(())
    }

    async fn update_last_scanned(&self, id: &str, timestamp: i64) -> AppResult<()> {
        sqlx::query("UPDATE library_folders SET last_scanned_at = ? WHERE id = ?")
            .bind(timestamp)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to update scan timestamp: {}", e)))?;

        Ok(())
    }
}
