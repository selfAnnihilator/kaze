//! Download records as the UI receives them: each task plus the track ids that
//! show it (online aliases and the saved file's library row) and whether its
//! file is still on disk.

use crate::core::error::AppResult;
use crate::database::models::DownloadTaskRecord;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::Path;

pub async fn download_rows(
    pool: &SqlitePool,
    tasks: Vec<DownloadTaskRecord>,
) -> AppResult<Vec<serde_json::Value>> {
    let aliases = online_aliases(pool).await?;
    let mut rows = Vec::with_capacity(tasks.len());
    for task in tasks {
        let mut ids = task
            .source_track_id
            .as_ref()
            .and_then(|id| aliases.get(id))
            .cloned()
            .unwrap_or_default();
        let saved = task.destination_path.clone().filter(|path| Path::new(path).is_file());
        if let Some(path) = &saved {
            // The Downloads list shows (and removes) the download from its own rows.
            ids.extend(library_row(pool, path).await?);
        }
        let Ok(mut value) = serde_json::to_value(task) else { continue };
        value["track_ids"] = serde_json::json!(ids);
        value["file_present"] = serde_json::json!(saved.is_some());
        rows.push(value);
    }
    Ok(rows)
}

/// Every saved id of each JioSaavn recording, keyed by recording id.
async fn online_aliases(pool: &SqlitePool) -> AppResult<HashMap<String, Vec<String>>> {
    let pairs: Vec<(String, String)> = sqlx::query_as(
        "SELECT track_id, provider_id FROM online_track_sources \
         UNION SELECT id, provider_id FROM external_tracks WHERE provider = 'jiosaavn'",
    )
    .fetch_all(pool)
    .await?;
    let mut aliases: HashMap<String, Vec<String>> = HashMap::new();
    for (id, source) in pairs {
        aliases.entry(source).or_default().push(id);
    }
    Ok(aliases)
}

/// The library row of a saved file (stored paths are canonical).
async fn library_row(pool: &SqlitePool, path: &str) -> AppResult<Option<String>> {
    let canonical = Path::new(path)
        .canonicalize()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.to_string());
    Ok(sqlx::query_scalar("SELECT id FROM tracks WHERE file_path IN (?, ?) LIMIT 1")
        .bind(path)
        .bind(&canonical)
        .fetch_optional(pool)
        .await?)
}
