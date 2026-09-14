use crate::core::error::{AppError, AppResult};
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserRecord {
    pub id: String,
    pub username: String,
    pub password_hash: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: String,
    pub username: String,
    pub created_at: i64,
}

pub fn hash_password(password: &str, salt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt.as_bytes());
    hasher.update(b":");
    hasher.update(password.as_bytes());
    format!("{}:{:x}", salt, hasher.finalize())
}

pub fn verify_password(password: &str, stored_hash: &str) -> bool {
    let parts: Vec<&str> = stored_hash.split(':').collect();
    if parts.len() != 2 {
        return false;
    }
    let salt = parts[0];
    let computed = hash_password(password, salt);
    computed == stored_hash
}

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn create_user(&self, username: &str, password: &str) -> AppResult<UserProfile>;
    async fn authenticate_user(&self, username: &str, password: &str) -> AppResult<UserProfile>;
    async fn get_user_by_id(&self, id: &str) -> AppResult<Option<UserProfile>>;
    async fn claim_guest_data_for_user(&self, user_id: &str) -> AppResult<()>;
}

#[derive(Clone)]
pub struct SqliteUserRepository {
    pool: SqlitePool,
}

impl SqliteUserRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for SqliteUserRepository {
    async fn create_user(&self, username: &str, password: &str) -> AppResult<UserProfile> {
        let trimmed_name = username.trim();
        if trimmed_name.is_empty() {
            return Err(AppError::Validation("Username cannot be empty".to_string()));
        }
        if password.len() < 3 {
            return Err(AppError::Validation("Password must be at least 3 characters".to_string()));
        }

        let existing: Option<(String,)> = sqlx::query_as("SELECT id FROM users WHERE username = ? COLLATE NOCASE")
            .bind(trimmed_name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

        if existing.is_some() {
            return Err(AppError::Validation("Username is already taken".to_string()));
        }

        let user_id = Uuid::new_v4().to_string();
        let salt = Uuid::new_v4().to_string().replace('-', "");
        let password_hash = hash_password(password, &salt);
        let now = Utc::now().timestamp();

        sqlx::query(
            "INSERT INTO users (id, username, password_hash, created_at) VALUES (?, ?, ?, ?)"
        )
        .bind(&user_id)
        .bind(trimmed_name)
        .bind(&password_hash)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to insert user: {}", e)))?;

        let _ = self.claim_guest_data_for_user(&user_id).await;

        Ok(UserProfile {
            id: user_id,
            username: trimmed_name.to_string(),
            created_at: now,
        })
    }

    async fn authenticate_user(&self, username: &str, password: &str) -> AppResult<UserProfile> {
        let trimmed_name = username.trim();
        let user: Option<UserRecord> = sqlx::query_as(
            "SELECT id, username, password_hash, created_at FROM users WHERE username = ? COLLATE NOCASE"
        )
        .bind(trimmed_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        match user {
            Some(record) => {
                if verify_password(password, &record.password_hash) {
                    Ok(UserProfile {
                        id: record.id,
                        username: record.username,
                        created_at: record.created_at,
                    })
                } else {
                    Err(AppError::Validation("Invalid username or password".to_string()))
                }
            }
            None => Err(AppError::Validation("Invalid username or password".to_string())),
        }
    }

    async fn get_user_by_id(&self, id: &str) -> AppResult<Option<UserProfile>> {
        let user: Option<UserRecord> = sqlx::query_as(
            "SELECT id, username, password_hash, created_at FROM users WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(user.map(|r| UserProfile {
            id: r.id,
            username: r.username,
            created_at: r.created_at,
        }))
    }

    async fn claim_guest_data_for_user(&self, user_id: &str) -> AppResult<()> {
        let _ = sqlx::query(
            "UPDATE playlists SET user_id = ? WHERE user_id = 'default' OR user_id IS NULL"
        )
        .bind(user_id)
        .execute(&self.pool)
        .await;

        let _ = sqlx::query(
            "UPDATE playback_history SET user_id = ? WHERE user_id = 'default' OR user_id IS NULL"
        )
        .bind(user_id)
        .execute(&self.pool)
        .await;

        let _ = sqlx::query(
            "UPDATE yearly_stats_archive SET user_id = ? WHERE user_id = 'default' OR user_id IS NULL"
        )
        .bind(user_id)
        .execute(&self.pool)
        .await;

        Ok(())
    }
}
