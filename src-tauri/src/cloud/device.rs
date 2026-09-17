use crate::core::error::{AppError, AppResult};
use sqlx::SqlitePool;

/// Returns the persistent device ID for this installation, generating a random UUID v4 on first run.
/// Hardware fingerprinting (CPU serials, MAC addresses, machine GUIDs) is explicitly avoided.
pub async fn get_or_create_device_id(pool: &SqlitePool) -> AppResult<String> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT value FROM application_settings WHERE key = 'device_id'"
    )
    .fetch_optional(pool)
    .await
    .map_err(|e| AppError::Database(format!("Failed to query device_id setting: {}", e)))?;

    if let Some((device_id,)) = row {
        if !device_id.trim().is_empty() {
            return Ok(device_id);
        }
    }

    let new_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp();
    sqlx::query(
        "INSERT INTO application_settings (key, value, updated_at) VALUES ('device_id', ?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
         WHERE trim(application_settings.value) = ''"
    )
    .bind(&new_id)
    .bind(now)
    .execute(pool)
    .await
    .map_err(|e| AppError::Database(format!("Failed to store device_id setting: {}", e)))?;

    sqlx::query_scalar("SELECT value FROM application_settings WHERE key = 'device_id'")
        .fetch_one(pool).await
        .map_err(|e| AppError::Database(format!("Failed to read device_id: {}", e)))
}

/// Returns a friendly, non-invasive device name based on the operating system.
pub fn get_device_name() -> String {
    let os = std::env::consts::OS;
    match os {
        "linux" => "Linux Desktop".to_string(),
        "windows" => "Windows PC".to_string(),
        "macos" => "macOS Desktop".to_string(),
        other => format!("{} Desktop", other),
    }
}

/// Returns the compiled client application version.
pub fn get_client_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
