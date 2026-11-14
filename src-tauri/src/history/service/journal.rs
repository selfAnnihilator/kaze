//! The crash-safe session journal: checkpoints of the active session, and
//! recovering sessions a crash or power loss interrupted.
use super::*;

impl HistoryService {
    /// Write or update the active_playback_journal row for crash recovery.
    pub(super) async fn persist_journal_entry_inner(&self, session: &ActiveSession) {
        let pool = match &self.pool {
            Some(p) => p,
            None => return,
        };
        let now = Utc::now().timestamp();
        let device_id = crate::cloud::device::get_or_create_device_id(pool).await
            .unwrap_or_else(|_| "_unknown".to_string());
        let _ = sqlx::query(
            "INSERT INTO active_playback_journal
                (session_id, user_id, track_id, device_id, stat_date, started_at, listened_secs, duration_secs, source, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(session_id) DO UPDATE SET
                 listened_secs = excluded.listened_secs,
                 updated_at    = excluded.updated_at"
        )
        .bind(&session.id)
        .bind(&session.user_id)
        .bind(&session.track_id)
        .bind(&device_id)
        .bind(&session.stat_date)
        .bind(session.started_at)
        .bind(session.listened_secs)
        .bind(session.duration_secs)
        .bind(&session.source)
        .bind(now)
        .execute(pool)
        .await;
    }

    /// Delete the journal entry after a clean finalization.
    pub(super) async fn delete_journal_entry(&self, session_id: &str) {
        let pool = match &self.pool {
            Some(p) => p,
            None => return,
        };
        let _ = sqlx::query("DELETE FROM active_playback_journal WHERE session_id = ?")
            .bind(session_id)
            .execute(pool)
            .await;
    }

    /// Check if enough new listened time has accumulated to warrant a journal checkpoint (~30s).
    pub(super) async fn maybe_checkpoint_journal(&self, session_id: &str) {
        let mut guard = self.current_session.lock().await;
        if let Some(ref mut session) = *guard {
            if session.id != session_id {
                return;
            }
            let current_total = session.total_listened();
            if current_total - session.last_journal_secs >= 30.0 {
                session.last_journal_secs = current_total;
                // We need a snapshot for the async call — avoid holding the lock.
                let snapshot_listened = session.listened_secs;
                let since_extra = session.playing_since
                    .map(|t| t.elapsed().as_secs_f64())
                    .unwrap_or(0.0);
                let effective_listened = snapshot_listened + since_extra;
                // Temporarily update listened_secs for the journal write.
                let orig = session.listened_secs;
                session.listened_secs = effective_listened;
                self.persist_journal_entry_inner(session).await;
                session.listened_secs = orig;
            }
        }
    }

    /// Recover any interrupted sessions recorded in active_playback_journal.
    /// Called at startup before normal operation begins.
    pub async fn recover_interrupted_sessions(&self) {
        let pool = match &self.pool {
            Some(p) => p,
            None => return,
        };

        type JournalRow = (String, String, String, String, String, i64, f64, f64, String);
        let rows: Vec<JournalRow> =
            sqlx::query_as(
                "SELECT session_id, user_id, track_id, device_id, stat_date,
                        started_at, listened_secs, duration_secs, source
                 FROM active_playback_journal"
            )
            .fetch_all(pool)
            .await
            .unwrap_or_default();

        for row in rows {
            let (session_id, user_id, track_id, _device_id, stat_date,
                 started_at, listened_secs, duration_secs, source) = row;

            info!(
                session_id = %session_id,
                track_id   = %track_id,
                listened   = listened_secs,
                "Recovering interrupted playback session from journal"
            );

            let now = Utc::now().timestamp();
            let percentage = if duration_secs > 0.0 {
                (listened_secs / duration_secs).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let is_meaningful = listened_secs >= 30.0 || percentage >= 0.8;

            let entry = PlaybackHistoryRecord {
                id: session_id.clone(),
                user_id: user_id.clone(),
                track_id: track_id.clone(),
                started_at,
                ended_at: now,
                seconds_listened: listened_secs,
                percentage_listened: percentage,
                completed: 0,
                skipped: 0,
                source,
                playlist_id: None,
                recommendation_session_id: None,
            };

            if let Err(e) = self.history_repo.record_session(&entry, &stat_date, is_meaningful).await {
                error!(session_id = %session_id, %e, "Failed to recover interrupted session");
            } else {
                // Clean up the journal row.
                let _ = sqlx::query("DELETE FROM active_playback_journal WHERE session_id = ?")
                    .bind(&session_id)
                    .execute(pool)
                    .await;
            }
        }
    }
}
