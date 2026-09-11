use crate::core::error::{AppError, AppResult};
use async_trait::async_trait;
use chrono::Utc;
use sqlx::SqlitePool;

#[async_trait]
pub trait SettingsRepository: Send + Sync {
    async fn get_setting(&self, key: &str) -> AppResult<Option<String>>;
    async fn set_setting(&self, key: &str, value: &str) -> AppResult<()>;
}

#[derive(Clone)]
pub struct SqliteSettingsRepository {
    pool: SqlitePool,
}

impl SqliteSettingsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SettingsRepository for SqliteSettingsRepository {
    async fn get_setting(&self, key: &str) -> AppResult<Option<String>> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT value FROM application_settings WHERE key = ?"
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to fetch setting {}: {}", key, e)))?;

        Ok(row.map(|r| r.0))
    }

    async fn set_setting(&self, key: &str, value: &str) -> AppResult<()> {
        let now = Utc::now().timestamp();
        sqlx::query(
            "INSERT INTO application_settings (key, value, updated_at) VALUES (?, ?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at"
        )
        .bind(key)
        .bind(value)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to set setting {}: {}", key, e)))?;

        Ok(())
    }
}
