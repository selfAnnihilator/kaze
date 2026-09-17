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
    pub display_name: Option<String>,
    pub avatar_key: Option<String>,
    pub avatar_public_id: Option<String>,
    pub avatar_url: Option<String>,
    pub avatar_version: Option<i64>,
    pub avatar_updated_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserProfile {
    pub id: String,
    pub username: String,
    pub created_at: i64,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub avatar_key: Option<String>,
    #[serde(default)]
    pub avatar_public_id: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub avatar_version: Option<i64>,
    #[serde(default)]
    pub avatar_updated_at: Option<i64>,
    #[serde(default)]
    pub avatar_data_url: Option<String>,
}

impl UserProfile {
    pub fn new(id: String, username: String, created_at: i64) -> Self {
        Self {
            id,
            username,
            created_at,
            display_name: None,
            avatar_key: None,
            avatar_public_id: None,
            avatar_url: None,
            avatar_version: None,
            avatar_updated_at: None,
            avatar_data_url: None,
        }
    }

    pub fn with_avatar(
        mut self,
        display_name: Option<String>,
        avatar_key: Option<String>,
        avatar_public_id: Option<String>,
        avatar_url: Option<String>,
        avatar_version: Option<i64>,
        avatar_updated_at: Option<i64>,
        avatar_data_url: Option<String>,
    ) -> Self {
        self.display_name = display_name;
        self.avatar_key = avatar_key;
        self.avatar_public_id = avatar_public_id;
        self.avatar_url = avatar_url;
        self.avatar_version = avatar_version;
        self.avatar_updated_at = avatar_updated_at;
        self.avatar_data_url = avatar_data_url;
        self
    }
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
    async fn update_avatar_metadata(
        &self,
        user_id: &str,
        avatar_key: Option<&str>,
        avatar_public_id: Option<&str>,
        avatar_url: Option<&str>,
        avatar_version: Option<i64>,
        avatar_updated_at: Option<i64>,
    ) -> AppResult<()>;
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

        // Registration does not implicitly transfer guest listening ownership.

        Ok(UserProfile {
            id: user_id,
            username: trimmed_name.to_string(),
            created_at: now,
            display_name: None,
            avatar_key: None,
            avatar_public_id: None,
            avatar_url: None,
            avatar_version: None,
            avatar_updated_at: None,
            avatar_data_url: None,
        })
    }

    async fn authenticate_user(&self, username: &str, password: &str) -> AppResult<UserProfile> {
        let trimmed_name = username.trim();
        let user: Option<UserRecord> = sqlx::query_as(
            "SELECT id, username, password_hash, created_at, display_name, avatar_key, avatar_public_id, avatar_url, avatar_version, avatar_updated_at FROM users WHERE username = ? COLLATE NOCASE"
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
                        display_name: record.display_name,
                        avatar_key: record.avatar_key,
                        avatar_public_id: record.avatar_public_id,
                        avatar_url: record.avatar_url,
                        avatar_version: record.avatar_version,
                        avatar_updated_at: record.avatar_updated_at,
                        avatar_data_url: None,
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
            "SELECT id, username, password_hash, created_at, display_name, avatar_key, avatar_public_id, avatar_url, avatar_version, avatar_updated_at FROM users WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(user.map(|r| UserProfile {
            id: r.id,
            username: r.username,
            created_at: r.created_at,
            display_name: r.display_name,
            avatar_key: r.avatar_key,
            avatar_public_id: r.avatar_public_id,
            avatar_url: r.avatar_url,
            avatar_version: r.avatar_version,
            avatar_updated_at: r.avatar_updated_at,
            avatar_data_url: None,
        }))
    }

    async fn update_avatar_metadata(
        &self,
        user_id: &str,
        avatar_key: Option<&str>,
        avatar_public_id: Option<&str>,
        avatar_url: Option<&str>,
        avatar_version: Option<i64>,
        avatar_updated_at: Option<i64>,
    ) -> AppResult<()> {
        sqlx::query(
            "UPDATE users SET avatar_key = ?, avatar_public_id = ?, avatar_url = ?, avatar_version = ?, avatar_updated_at = ? WHERE id = ?"
        )
        .bind(avatar_key)
        .bind(avatar_public_id)
        .bind(avatar_url)
        .bind(avatar_version)
        .bind(avatar_updated_at)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| AppError::Database(format!("Failed to update avatar metadata: {}", e)))?;

        Ok(())
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

        let _ = sqlx::query(
            "UPDATE OR IGNORE track_statistics SET user_id = ? WHERE user_id = 'default' OR user_id IS NULL"
        )
        .bind(user_id)
        .execute(&self.pool)
        .await;

        let _ = sqlx::query(
            "UPDATE OR IGNORE user_preferences SET user_id = ? WHERE user_id = 'default' OR user_id IS NULL"
        )
        .bind(user_id)
        .execute(&self.pool)
        .await;

        let _ = sqlx::query(
            "UPDATE OR IGNORE recommendation_sessions SET user_id = ? WHERE user_id = 'default' OR user_id IS NULL"
        )
        .bind(user_id)
        .execute(&self.pool)
        .await;

        Ok(())
    }
}
