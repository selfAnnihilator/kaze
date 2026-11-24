//! Retries a sync that lost a race for SQLite's write lock.
//!
//! At launch several writers start together (library scan, mixes, the likes
//! cleanup). A sync that collides with them fails with "database is locked"
//! and, without a retry, the device shows stale cloud data until the next
//! sync. Every sync step is an upsert, so running it again is safe.

use super::{SyncManager, SyncReport};
use crate::cloud::client::CloudClient;
use crate::core::error::{AppError, AppResult};
use sqlx::SqlitePool;
use std::time::Duration;

/// Waits before each retry, in seconds.
const WAITS: [u64; 2] = [3, 10];

/// Whether the error is SQLite's busy or busy-snapshot lock error.
pub(crate) fn lost_lock_race(error: &AppError) -> bool {
    error.to_string().contains("database is locked")
}

impl SyncManager {
    /// `sync_with_cloud`, retried when it loses a race for the write lock.
    pub async fn sync_with_cloud_retrying(
        pool: &SqlitePool,
        client: &CloudClient,
        worker_url: &str,
        user_id: &str,
        token: &str,
    ) -> AppResult<SyncReport> {
        let mut waits = WAITS.iter();
        loop {
            match Self::sync_with_cloud(pool, client, worker_url, user_id, token).await {
                Err(error) if lost_lock_race(&error) => match waits.next() {
                    Some(secs) => {
                        tracing::info!(secs, "cloud sync lost a database lock race; retrying");
                        tokio::time::sleep(Duration::from_secs(*secs)).await;
                    }
                    None => return Err(error),
                },
                other => return other,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_lock_errors_are_retried() {
        assert!(lost_lock_race(&AppError::Database("error returned from database: (code: 517) database is locked".into())));
        assert!(lost_lock_race(&AppError::Database("error returned from database: (code: 5) database is locked".into())));
        assert!(!lost_lock_race(&AppError::Network("timed out".into())));
        assert!(!lost_lock_race(&AppError::Validation("Session expired".into())));
    }
}
