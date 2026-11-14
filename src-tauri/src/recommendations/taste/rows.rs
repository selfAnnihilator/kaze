//! The rows a taste profile is computed from, one query each.
use super::*;

#[derive(Debug, FromRow)]
#[allow(dead_code)]
pub(super) struct HistorySessionRow {
    pub(super) track_id: String,
    pub(super) artist_id: Option<String>,
    pub(super) artist_name: Option<String>,
    pub(super) genre_name: Option<String>,
    pub(super) year: Option<i64>,
    pub(super) started_at: i64,
    pub(super) duration_secs: f64,
    pub(super) seconds_listened: f64,
    pub(super) skipped: i64,
}

#[derive(Debug, FromRow)]
#[allow(dead_code)]
pub(super) struct FeedbackRow {
    pub(super) track_id: String,
    pub(super) artist_id: Option<String>,
    pub(super) artist_name: Option<String>,
    pub(super) genre_name: Option<String>,
    pub(super) manual_like: i64,
}

#[derive(Debug, FromRow)]
pub(super) struct PriorRow {
    pub(super) track_id: String,
    pub(super) artist_id: Option<String>,
    pub(super) artist_name: Option<String>,
    pub(super) genre_name: Option<String>,
    pub(super) from_playlist: i64,
}

/// Listening history with track, artist and genre, oldest first.
pub(super) async fn history(pool: &SqlitePool, user_id: &str) -> AppResult<Vec<HistorySessionRow>> {
    sqlx::query_as::<_, HistorySessionRow>(
        "SELECT h.track_id, t.artist_id, a.name as artist_name, COALESCE(g.name, e.genre) as genre_name,
                t.year, h.started_at, COALESCE(t.duration_secs,0) as duration_secs, h.seconds_listened, h.skipped
         FROM playback_history h
         JOIN tracks t ON t.id = h.track_id
         LEFT JOIN artists a ON a.id = t.artist_id
         LEFT JOIN genres g ON g.id = t.genre_id
         LEFT JOIN external_tracks e ON e.id = t.id
         WHERE h.user_id = ?
         ORDER BY h.started_at ASC"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))
}

/// Explicit likes/dislikes, plus Liked Songs entries without a rating.
pub(super) async fn feedback(pool: &SqlitePool, user_id: &str) -> AppResult<Vec<FeedbackRow>> {
    sqlx::query_as::<_, FeedbackRow>(
        "SELECT ts.track_id, t.artist_id, a.name as artist_name, COALESCE(g.name, e.genre) as genre_name, ts.manual_like
         FROM track_statistics ts
         JOIN tracks t ON t.id = ts.track_id
         LEFT JOIN artists a ON a.id = t.artist_id
         LEFT JOIN genres g ON g.id = t.genre_id
         LEFT JOIN external_tracks e ON e.id = t.id
         WHERE ts.user_id = ?
         UNION
         SELECT t.id,t.artist_id,a.name,COALESCE(g.name,e.genre),1
         FROM playlist_tracks pt JOIN playlists p ON p.id=pt.playlist_id JOIN tracks t ON t.id=pt.track_id
         LEFT JOIN artists a ON a.id=t.artist_id LEFT JOIN genres g ON g.id=t.genre_id LEFT JOIN external_tracks e ON e.id=t.id
         WHERE p.user_id=? AND p.name='Liked Songs' AND p.is_smart_mix=0
           AND NOT EXISTS(SELECT 1 FROM track_statistics ts WHERE ts.track_id=t.id AND ts.user_id=p.user_id AND ts.manual_like!=0)"
    )
    .bind(user_id).bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))
}

/// Passive priors: the shared local library and the user's own playlists.
pub(super) async fn priors(pool: &SqlitePool, user_id: &str) -> AppResult<Vec<PriorRow>> {
    sqlx::query_as::<_, PriorRow>(
        "SELECT t.id as track_id, t.artist_id, a.name as artist_name, g.name as genre_name, 0 as from_playlist
         FROM tracks t
         LEFT JOIN artists a ON a.id = t.artist_id
         LEFT JOIN genres g ON g.id = t.genre_id
         WHERE t.format != 'online' AND t.file_path NOT LIKE 'online://%'
         UNION ALL
         SELECT DISTINCT t.id, t.artist_id, a.name, COALESCE(g.name, e.genre), 1
         FROM playlist_tracks pt
         JOIN playlists p ON p.id = pt.playlist_id
         JOIN tracks t ON t.id = pt.track_id
         LEFT JOIN artists a ON a.id = t.artist_id
         LEFT JOIN genres g ON g.id = t.genre_id
         LEFT JOIN external_tracks e ON e.id = t.id
         WHERE p.user_id = ? AND p.is_smart_mix = 0 AND p.name != 'Liked Songs'",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::Database(e.to_string()))
}
