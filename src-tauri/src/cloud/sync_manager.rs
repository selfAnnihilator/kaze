//! Two-way cloud sync: pull, reconcile, push. Session storage, the outgoing
//! payload and applying pulled state live in child modules.
use super::models::*;
use crate::core::error::{AppError, AppResult};
use chrono::Utc;
use sqlx::SqlitePool;

mod incoming;
mod outgoing;
mod session;

#[derive(Debug, Clone, Default)]
pub struct SyncReport {
    pub cloud_playlists_returned: usize,
    pub cloud_playlist_songs_returned: usize,
    pub local_playlists_before: usize,
    pub playlists_inserted_updated: usize,
    pub playlist_memberships_inserted_updated: usize,
    pub pending_local_changes_uploaded: usize,
    pub local_playlists_after: usize,
    pub synced_at: i64,
}

pub struct SyncManager;

impl SyncManager {
    /// Performs deterministic two-way synchronization with Cloudflare D1
    /// Flow: PULL cloud state first -> RECONCILE locally -> PUSH legitimate changes
    pub async fn sync_with_cloud(
        pool: &SqlitePool,
        client: &super::client::CloudClient,
        worker_url: &str,
        user_id: &str,
        token: &str,
    ) -> AppResult<SyncReport> {
        tracing::info!("SYNC START: user_id = {}", user_id);

        let local_before: (i64,) =
            sqlx::query_as("SELECT count(*) FROM playlists WHERE user_id = ? AND is_smart_mix = 0")
                .bind(user_id)
                .fetch_one(pool)
                .await
                .unwrap_or((0,));
        let local_playlists_before = local_before.0 as usize;

        // Step 1: PULL cloud state FIRST.
        // Safeguard: If the pull fails, abort immediately to preserve local data and prevent empty-state overwrite.
        let remote_data = client.pull_sync(worker_url, token).await?;
        let cloud_playlists_returned = remote_data.playlists.len();
        let cloud_playlist_songs_returned = remote_data.playlist_songs.len();

        tracing::info!("cloud playlists returned: {}", cloud_playlists_returned);
        tracing::info!(
            "cloud playlist_songs returned: {}",
            cloud_playlist_songs_returned
        );
        tracing::info!("local playlists before sync: {}", local_playlists_before);

        // Step 2: Reconcile local SQLite with remote cloud data
        let (playlists_inserted_updated, playlist_memberships_inserted_updated) =
            Self::apply_remote_sync_payload(pool, user_id, &remote_data).await?;

        tracing::info!(
            "playlists inserted/updated locally: {}",
            playlists_inserted_updated
        );
        tracing::info!(
            "playlist memberships inserted/updated: {}",
            playlist_memberships_inserted_updated
        );

        // Step 3: Prepare local changes (including any local playlists and pending tombstones)
        let local_payload = Self::prepare_local_sync_payload(pool, user_id).await?;
        let pending_local_changes_uploaded = local_payload.playlists.len()
            + local_payload.deleted_playlists.len()
            + local_payload.deleted_playlist_songs.len();

        // Step 4: PUSH legitimate local changes to cloud
        let synced_at = client.push_sync(worker_url, token, &local_payload).await?;
        tracing::info!(
            "pending local changes uploaded: {}",
            pending_local_changes_uploaded
        );

        // Step 5: Clear tombstones that have now been pushed
        let _ = Self::clear_tombstones(pool, user_id).await;

        // Step 6: Update session synced_at timestamp
        Self::update_session_synced_at(pool, user_id, synced_at).await?;

        let local_after: (i64,) =
            sqlx::query_as("SELECT count(*) FROM playlists WHERE user_id = ? AND is_smart_mix = 0")
                .bind(user_id)
                .fetch_one(pool)
                .await
                .unwrap_or((0,));
        let local_playlists_after = local_after.0 as usize;

        tracing::info!("local playlists after sync: {}", local_playlists_after);
        tracing::info!("frontend playlist refresh triggered");

        Ok(SyncReport {
            cloud_playlists_returned,
            cloud_playlist_songs_returned,
            local_playlists_before,
            playlists_inserted_updated,
            playlist_memberships_inserted_updated,
            pending_local_changes_uploaded,
            local_playlists_after,
            synced_at,
        })
    }

    pub async fn update_session_synced_at(
        pool: &SqlitePool,
        user_id: &str,
        synced_at: i64,
    ) -> AppResult<()> {
        sqlx::query("UPDATE cloud_sessions SET synced_at = ? WHERE user_id = ?")
            .bind(synced_at)
            .bind(user_id)
            .execute(pool)
            .await
            .map_err(|e| AppError::Database(format!("Failed to update synced_at: {}", e)))?;

        Ok(())
    }
}
