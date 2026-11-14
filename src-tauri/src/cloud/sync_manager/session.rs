//! The signed-in cloud session and deletion tombstones, stored locally.
use super::*;

impl SyncManager {
    pub async fn get_active_session(pool: &SqlitePool) -> AppResult<Option<CloudSessionMetadata>> {
        let record: Option<CloudSessionMetadata> = sqlx::query_as(
            "SELECT user_id, username, session_id, device_id, device_name,
                    idle_expires_at, absolute_expires_at, last_cloud_validation_at,
                    worker_url, synced_at, authenticated_before, created_at
             FROM cloud_sessions ORDER BY created_at DESC LIMIT 1",
        )
        .fetch_optional(pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to fetch cloud session: {}", e)))?;

        Ok(record)
    }

    pub async fn save_session(pool: &SqlitePool, metadata: &CloudSessionMetadata) -> AppResult<()> {
        sqlx::query(
            "INSERT INTO cloud_sessions (
                user_id, username, session_id, device_id, device_name,
                idle_expires_at, absolute_expires_at, last_cloud_validation_at,
                worker_url, synced_at, authenticated_before, created_at
             )
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(user_id) DO UPDATE SET
                 username = excluded.username,
                 session_id = excluded.session_id,
                 device_id = excluded.device_id,
                 device_name = excluded.device_name,
                 idle_expires_at = excluded.idle_expires_at,
                 absolute_expires_at = excluded.absolute_expires_at,
                 last_cloud_validation_at = excluded.last_cloud_validation_at,
                 worker_url = excluded.worker_url,
                 synced_at = excluded.synced_at,
                 authenticated_before = excluded.authenticated_before",
        )
        .bind(&metadata.user_id)
        .bind(&metadata.username)
        .bind(&metadata.session_id)
        .bind(&metadata.device_id)
        .bind(&metadata.device_name)
        .bind(metadata.idle_expires_at)
        .bind(metadata.absolute_expires_at)
        .bind(metadata.last_cloud_validation_at)
        .bind(&metadata.worker_url)
        .bind(metadata.synced_at)
        .bind(metadata.authenticated_before)
        .bind(metadata.created_at)
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to save cloud session: {}", e)))?;

        Ok(())
    }

    pub async fn update_validation_timestamp(
        pool: &SqlitePool,
        user_id: &str,
        timestamp: i64,
    ) -> AppResult<()> {
        sqlx::query("UPDATE cloud_sessions SET last_cloud_validation_at = ? WHERE user_id = ?")
            .bind(timestamp)
            .bind(user_id)
            .execute(pool)
            .await
            .map_err(|e| {
                AppError::Database(format!("Failed to update validation timestamp: {}", e))
            })?;

        Ok(())
    }

    pub async fn delete_session(pool: &SqlitePool, user_id: &str) -> AppResult<()> {
        let _ = crate::cloud::credentials::delete_session_token(user_id);
        sqlx::query("DELETE FROM cloud_sessions WHERE user_id = ?")
            .bind(user_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to delete cloud session: {}", e)))?;

        Ok(())
    }

    pub async fn record_tombstone(
        pool: &SqlitePool,
        user_id: &str,
        entity_type: &str,
        entity_id: &str,
    ) -> AppResult<()> {
        let id = format!("{}:{}:{}", user_id, entity_type, entity_id);
        let now = Utc::now().timestamp();
        sqlx::query(
            "INSERT INTO sync_tombstones (id, user_id, entity_type, entity_id, deleted_at)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET deleted_at = excluded.deleted_at",
        )
        .bind(&id)
        .bind(user_id)
        .bind(entity_type)
        .bind(entity_id)
        .bind(now)
        .execute(pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to record sync tombstone: {}", e)))?;

        Ok(())
    }

    pub async fn get_tombstones(
        pool: &SqlitePool,
        user_id: &str,
    ) -> AppResult<Vec<(String, String, String)>> {
        let rows = sqlx::query_as::<_, (String, String, String)>(
            "SELECT id, entity_type, entity_id FROM sync_tombstones WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        Ok(rows)
    }

    pub async fn clear_tombstones(pool: &SqlitePool, user_id: &str) -> AppResult<()> {
        sqlx::query("DELETE FROM sync_tombstones WHERE user_id = ?")
            .bind(user_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to clear sync tombstones: {}", e)))?;

        Ok(())
    }
}
