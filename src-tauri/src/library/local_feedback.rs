//! Songs from the user's own folders can't be liked or disliked, on any device.
//! Only online songs and Kaze's downloads (files in Kaze's folder) take a rating.
//!
//! Existing ratings are cleared in two ways:
//! - **This device's own files** lose their like, dislike and Liked Songs entry.
//!   A tombstone carries the removal to the cloud.
//! - **Another device's files** (cloud songs with provider `local`) leave this
//!   device's Liked Songs, but nothing is uploaded. A Kaze download from another
//!   device arrives labelled `local` too, so clearing it in the cloud would wipe
//!   a valid like. The device that owns the file clears it.
//!
//! The cleanup runs at startup and in every sync, after the pull. A pull writes
//! the cloud's ratings back, so cleaning only once would be undone.

use crate::cloud::sync_manager::SyncManager;
use crate::core::error::AppResult;
use crate::database::repositories::LibraryScope;
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

/// Shown when a like or dislike is refused.
pub const REFUSED: &str = "Songs from your own folders can't be liked or disliked.";

/// Kaze's download folder, set once the core starts (cloud sync has no core handle).
static DOWNLOADS_DIR: RwLock<Option<PathBuf>> = RwLock::new(None);

pub fn set_downloads_dir(dir: &Path) {
    *DOWNLOADS_DIR.write().unwrap_or_else(|e| e.into_inner()) = Some(dir.to_path_buf());
}

pub fn downloads_dir() -> Option<PathBuf> {
    DOWNLOADS_DIR.read().unwrap_or_else(|e| e.into_inner()).clone()
}

/// SQL for "track `t` is one of this device's files outside Kaze's folder", with its binds.
fn own_file_condition(downloads_dir: &Path) -> (String, i64, String) {
    let (outside, len, prefix) = LibraryScope::parse(Some("local"), downloads_dir)
        .condition("t.file_path")
        .expect("the local scope always has a condition");
    (format!("t.file_path NOT LIKE 'online://%' AND {outside}"), len, prefix)
}

/// Whether `track_id` is a file from a local folder, on this device or another one.
pub async fn is_local_file(pool: &SqlitePool, track_id: &str, downloads_dir: &Path) -> AppResult<bool> {
    let (own, len, prefix) = own_file_condition(downloads_dir);
    let sql = format!(
        "SELECT EXISTS(SELECT 1 FROM tracks t WHERE t.id = ? AND {own})
             OR (EXISTS(SELECT 1 FROM external_tracks WHERE id = ? AND provider = 'local')
                 AND NOT EXISTS(SELECT 1 FROM tracks WHERE id = ? AND file_path NOT LIKE 'online://%'))"
    );
    let local: bool = sqlx::query_scalar(&sql)
        .bind(track_id).bind(len).bind(&prefix).bind(track_id).bind(track_id)
        .fetch_one(pool)
        .await?;
    Ok(local)
}

async fn liked_playlist(pool: &SqlitePool, user_id: &str) -> AppResult<Option<String>> {
    Ok(sqlx::query_scalar("SELECT id FROM playlists WHERE name = 'Liked Songs' AND is_smart_mix = 0 AND user_id = ? LIMIT 1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?)
}

/// This device's local files that still carry a rating or a Liked Songs entry.
async fn rated_own_files(pool: &SqlitePool, user_id: &str, liked: &str, downloads_dir: &Path) -> AppResult<Vec<String>> {
    let (own, len, prefix) = own_file_condition(downloads_dir);
    let sql = format!(
        "SELECT t.id FROM tracks t WHERE {own} AND (
             t.id IN (SELECT track_id FROM track_statistics WHERE user_id = ? AND manual_like != 0)
             OR t.id IN (SELECT track_id FROM playlist_tracks WHERE playlist_id = ?)
             OR t.id IN (SELECT track_id FROM recommendation_feedback WHERE user_id = ? AND manual_like != 0))"
    );
    Ok(sqlx::query_scalar(&sql)
        .bind(len).bind(&prefix).bind(user_id).bind(liked).bind(user_id)
        .fetch_all(pool)
        .await?)
}

/// Clears ratings on local files (see the module notes). Returns how many songs changed.
pub async fn clear(pool: &SqlitePool, user_id: &str, downloads_dir: &Path) -> AppResult<usize> {
    let liked = liked_playlist(pool, user_id).await?.unwrap_or_default();
    let own = rated_own_files(pool, user_id, &liked, downloads_dir).await?;
    let stamp = chrono::Utc::now().timestamp_millis();
    let mut tx = pool.begin().await?;
    for id in &own {
        sqlx::query("UPDATE track_statistics SET manual_like = 0 WHERE user_id = ? AND track_id = ?")
            .bind(user_id).bind(id).execute(&mut *tx).await?;
        sqlx::query("UPDATE recommendation_feedback SET manual_like = 0, updated_at = ? WHERE user_id = ? AND track_id = ? AND manual_like != 0")
            .bind(stamp).bind(user_id).bind(id).execute(&mut *tx).await?;
        sqlx::query("DELETE FROM playlist_tracks WHERE playlist_id = ? AND track_id = ?")
            .bind(&liked).bind(id).execute(&mut *tx).await?;
    }
    let foreign = sqlx::query(
        "DELETE FROM playlist_tracks WHERE playlist_id = ? AND track_id IN (
             SELECT e.id FROM external_tracks e WHERE e.provider = 'local'
             AND NOT EXISTS(SELECT 1 FROM tracks t WHERE t.id = e.id AND t.file_path NOT LIKE 'online://%'))",
    )
    .bind(&liked)
    .execute(&mut *tx)
    .await?
    .rows_affected() as usize;
    tx.commit().await?;
    for id in &own {
        SyncManager::record_tombstone(pool, user_id, "playlist_song", &format!("{liked}:{id}")).await?;
    }
    if !own.is_empty() || foreign > 0 {
        tracing::info!(own = own.len(), foreign, "cleared likes and dislikes on local files");
    }
    Ok(own.len() + foreign)
}

/// The startup cleanup, for every profile on this device (signed out included).
pub async fn clear_every_profile(pool: SqlitePool, downloads_dir: PathBuf) {
    let users: Vec<String> = sqlx::query_scalar(
        "SELECT user_id FROM track_statistics WHERE user_id IS NOT NULL UNION SELECT user_id FROM playlists WHERE name = 'Liked Songs' AND user_id IS NOT NULL",
    )
    .fetch_all(&pool)
    .await
    .unwrap_or_default();
    for user in users {
        if let Err(error) = clear(&pool, &user, &downloads_dir).await {
            tracing::warn!(%error, "couldn't clear ratings on local files");
        }
    }
}

#[cfg(test)]
#[path = "local_feedback_tests.rs"]
mod tests;
