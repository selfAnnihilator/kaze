//! Song and artist/genre feedback and how recommendations react to it.

use super::*;

impl CoreProcessor {
    pub(super) async fn set_song_feedback(&self, track_id: &str, feedback: i64) -> AppResult<()> {
        let user = self.current_user_id().await;
        if self
            .library_service
            .track_repo()
            .find_by_id(track_id)
            .await?
            .is_none()
        {
            type ExternalTrackRow = (String,String,Option<String>,Option<f64>,Option<String>);
            let row: Option<ExternalTrackRow> = sqlx::query_as("SELECT title,artist,album,duration_secs,cover_art_url FROM external_tracks WHERE id=?").bind(track_id).fetch_optional(&self.db_pool).await?;
            let Some((title, artist, album, duration, cover)) = row else {
                return Err(AppError::NotFound("Song not found".into()));
            };
            crate::recommendations::mixes::ensure_online_track(
                &self.db_pool,
                &OnlineTrackInput {
                    id: track_id,
                    title: &title,
                    artist: Some(&artist),
                    album: album.as_deref(),
                    duration_secs: duration,
                    cover_art_url: cover.as_deref(),
                    preview_url: None,
                },
            )
            .await?;
        }
        let index =
            crate::recommendations::signals::SongSignals::load(&self.db_pool, &user).await?;
        let ids = index.related_ids(track_id);
        let title = index
            .identities
            .get(track_id)
            .map(|song| song.title.clone())
            .unwrap_or_else(|| track_id.into());
        let liked = self
            .playlist_repo
            .ensure_liked_songs_playlist(&user)
            .await?;
        let mut transaction = self.db_pool.begin().await?;
        let identity = index
            .identities
            .get(track_id)
            .ok_or_else(|| AppError::NotFound("Song identity not found".into()))?;
        let previous: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(updated_at),0) FROM recommendation_feedback WHERE user_id=?",
        )
        .bind(&user)
        .fetch_one(&mut *transaction)
        .await?;
        let timestamp = chrono::Utc::now()
            .timestamp_millis()
            .max(previous.saturating_add(1));
        for key in index.related_feedback_keys(track_id) {
            sqlx::query("UPDATE recommendation_feedback SET manual_like=?,updated_at=? WHERE user_id=? AND song_key=?").bind(feedback).bind(timestamp).bind(&user).bind(key).execute(&mut *transaction).await?;
        }
        let key = crate::recommendations::signals::song_key(&identity.title, &identity.artist)
            .unwrap_or_else(|| format!("id:{track_id}"));
        sqlx::query("INSERT INTO recommendation_feedback(user_id,song_key,track_id,source_id,title,artist,manual_like,updated_at) VALUES(?,?,?,?,?,?,?,?) ON CONFLICT(user_id,song_key) DO UPDATE SET manual_like=excluded.manual_like,updated_at=excluded.updated_at,track_id=excluded.track_id,source_id=excluded.source_id,title=excluded.title,artist=excluded.artist")
            .bind(&user).bind(key).bind(track_id).bind(&identity.source_id).bind(&identity.title).bind(&identity.artist).bind(feedback).bind(timestamp).execute(&mut *transaction).await?;

        // One explicit opinion per recording, even when several saved IDs exist.
        for id in &ids {
            sqlx::query("UPDATE track_statistics SET manual_like=0 WHERE user_id=? AND track_id=?")
                .bind(&user)
                .bind(id)
                .execute(&mut *transaction)
                .await?;
            sqlx::query("DELETE FROM playlist_tracks WHERE playlist_id=? AND track_id=?")
                .bind(&liked)
                .bind(id)
                .execute(&mut *transaction)
                .await?;
            if feedback == -1 {
                sqlx::query("DELETE FROM playlist_tracks WHERE track_id=? AND playlist_id IN (SELECT id FROM playlists WHERE user_id=? AND is_smart_mix=1)").bind(id).bind(&user).execute(&mut *transaction).await?;
            }
        }
        sqlx::query("INSERT INTO track_statistics(user_id,track_id,manual_like) VALUES(?,?,?) ON CONFLICT(user_id,track_id) DO UPDATE SET manual_like=excluded.manual_like")
            .bind(&user).bind(track_id).bind(feedback).execute(&mut *transaction).await?;
        sqlx::query("UPDATE playlists SET expires_at=0 WHERE user_id=? AND is_smart_mix=1")
            .bind(&user)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        if feedback == 1 {
            self.playlist_repo.add_track(&liked, track_id, None).await?;
        }
        if feedback == -1 {
            self.playback_service
                .remove_recommended_songs(&user, &ids)
                .await?;
        }
        self.flow_engine.invalidate_home_picks(&user).await;
        let _ = self
            .event_bus
            .publish(Event::RecommendationFeedbackChanged {
                user_id: user.clone(),
                track_id: track_id.into(),
                title,
                track_ids: ids,
                disliked: feedback == -1,
                manual_like: feedback,
            });
        let _ = self
            .event_bus
            .publish(Event::TasteProfileChanged { user_id: user });
        self.trigger_background_sync().await;
        Ok(())
    }

    /// Writes artist-level ("block") or artist+genre ("less") feedback for a
    /// track and invalidates every cached recommendation surface.
    pub(super) async fn set_entity_feedback(
        &self,
        track_id: &str,
        level: &str,
    ) -> AppResult<Vec<crate::recommendations::entity_feedback::EntityFeedbackRow>> {
        use crate::recommendations::entity_feedback::{self as ef, EntityFeedbackRow};
        let user = self.current_user_id().await;
        let local: Option<(String, Option<String>)> = sqlx::query_as(
            "SELECT COALESCE(a.name, ''), g.name FROM tracks t
             LEFT JOIN artists a ON a.id = t.artist_id
             LEFT JOIN genres g ON g.id = t.genre_id WHERE t.id = ?",
        )
        .bind(track_id)
        .fetch_optional(&self.db_pool)
        .await?;
        let (artist, genre) = match local {
            Some(row) => row,
            None => sqlx::query_as::<_, (String, Option<String>)>(
                "SELECT artist, genre FROM external_tracks WHERE id = ?",
            )
            .bind(track_id)
            .fetch_optional(&self.db_pool)
            .await?
            .ok_or_else(|| AppError::NotFound("Song not found".into()))?,
        };
        let now = chrono::Utc::now().timestamp();
        let mut rows = Vec::new();
        for (key, display) in crate::recommendations::cooccurrence::display_credits(&artist) {
            ef::upsert(&self.db_pool, &user, "artist", &key, &display, level, now).await?;
            rows.push(EntityFeedbackRow {
                kind: "artist".into(),
                key,
                display,
                level: level.into(),
                created_at: now,
            });
        }
        if level == "less" {
            if let Some(genre) = genre.as_deref().map(str::trim).filter(|g| !g.is_empty()) {
                let key = ef::genre_norm(genre);
                ef::upsert(&self.db_pool, &user, "genre", &key, genre, "less", now).await?;
                rows.push(EntityFeedbackRow {
                    kind: "genre".into(),
                    key,
                    display: genre.into(),
                    level: "less".into(),
                    created_at: now,
                });
            }
        }
        if rows.is_empty() {
            return Err(AppError::NotFound("Song has no artist or genre".into()));
        }
        if level == "block" {
            let index =
                crate::recommendations::signals::SongSignals::load(&self.db_pool, &user).await?;
            let ids: Vec<String> = index
                .identities
                .values()
                .filter(|song| index.entity.is_blocked_artist(&song.artist))
                .map(|song| song.id.clone())
                .collect();
            sqlx::query("DELETE FROM playlist_tracks WHERE track_id IN (SELECT value FROM json_each(?)) AND playlist_id IN (SELECT id FROM playlists WHERE user_id=? AND is_smart_mix=1)")
                .bind(serde_json::to_string(&ids).unwrap_or_else(|_| "[]".into()))
                .bind(&user)
                .execute(&self.db_pool)
                .await?;
            self.playback_service
                .remove_recommended_songs(&user, &ids)
                .await?;
        }
        self.after_entity_feedback(&user, rows.clone(), false).await?;
        Ok(rows)
    }

    pub(super) async fn clear_entity_feedback(&self, kind: &str, key: &str) -> AppResult<()> {
        let user = self.current_user_id().await;
        let rows: Vec<_> = crate::recommendations::entity_feedback::load_rows(&self.db_pool, &user)
            .await?
            .into_iter()
            .filter(|row| row.kind == kind && row.key == key)
            .collect();
        if crate::recommendations::entity_feedback::clear(&self.db_pool, &user, kind, key).await? {
            self.after_entity_feedback(&user, rows, true).await?;
        }
        Ok(())
    }

    pub(super) async fn after_entity_feedback(
        &self,
        user: &str,
        rows: Vec<crate::recommendations::entity_feedback::EntityFeedbackRow>,
        cleared: bool,
    ) -> AppResult<()> {
        sqlx::query("UPDATE playlists SET expires_at=0 WHERE user_id=? AND is_smart_mix=1")
            .bind(user)
            .execute(&self.db_pool)
            .await?;
        self.flow_engine.invalidate_home_picks(user).await;
        let _ = self.event_bus.publish(Event::EntityFeedbackChanged {
            user_id: user.into(),
            rows,
            cleared,
        });
        let _ = self.event_bus.publish(Event::TasteProfileChanged {
            user_id: user.into(),
        });
        self.trigger_background_sync().await;
        Ok(())
    }

    pub(super) async fn present_recommendations(
        &self,
        value: serde_json::Value,
        user: &str,
    ) -> AppResult<serde_json::Value> {
        let value = self.online_catalog.present(value).await?;
        Ok(
            crate::recommendations::signals::SongSignals::load(&self.db_pool, user)
                .await?
                .filter(value)
                .unwrap_or(serde_json::Value::Null),
        )
    }
}
