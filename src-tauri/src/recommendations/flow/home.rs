//! The Home overview and its cached picks.
use super::*;

impl FlowEngine {
    /// Fetches the composite Kaze Home overview:
    /// - Flow Session
    /// - Recent Rotation
    /// - Rediscover
    /// - For You
    /// - Library Moment
    /// - Your Week
    pub async fn get_home_overview(&self, user_id: &str) -> AppResult<HomeOverview> {
        let now = Utc::now().timestamp();

        // 1. Flow session state from application_settings
        let flow_session = self.get_flow_session(user_id).await?;

        // 2. Recent Rotation (distinct tracks played recently)
        let recent_rotation = sqlx::query_as::<_, RecentTrackItem>(
            "SELECT h.track_id, t.title,
                    COALESCE(a.name, 'Unknown Artist') as artist,
                    al.title as album,
                    t.duration_secs,
                    CASE WHEN t.has_cover_art = 1 THEN '/api/cover/' || t.id ELSE NULL END as cover_art_url,
                    MAX(h.started_at) as started_at
             FROM playback_history h
             JOIN tracks t ON t.id = h.track_id
             LEFT JOIN artists a ON a.id = t.artist_id
             LEFT JOIN albums al ON al.id = t.album_id
             WHERE h.user_id = ? AND t.format != 'online' AND t.file_path NOT LIKE 'online://%'
             GROUP BY h.track_id
             ORDER BY started_at DESC
             LIMIT 8"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        // 3. Rediscover: Tracks previously played or liked, unplayed in > 14 days
        let cutoff_rediscover = now - (14 * 86400);
        type RediscoverRow = (String, String, Option<String>, Option<String>, f64, Option<i64>, Option<i64>, Option<i64>, i64);
        let rediscover_rows: Vec<RediscoverRow> = sqlx::query_as(
            "SELECT t.id, t.title, a.name as artist, al.title as album,
                    t.duration_secs, ts.last_played_at, ts.play_count, ts.manual_like,
                    t.has_cover_art
             FROM track_statistics ts
             JOIN tracks t ON t.id = ts.track_id
             LEFT JOIN artists a ON a.id = t.artist_id
             LEFT JOIN albums al ON al.id = t.album_id
             WHERE ts.user_id = ? AND COALESCE(ts.manual_like, 0) != -1
               AND t.format != 'online' AND t.file_path NOT LIKE 'online://%'
               AND (ts.manual_like = 1 OR ts.play_count >= 2)
               AND (ts.last_played_at IS NULL OR ts.last_played_at < ?)
             ORDER BY (COALESCE(ts.play_count, 0) * 2 + CASE WHEN ts.manual_like = 1 THEN 8 ELSE 0 END) DESC
             LIMIT 8"
        )
        .bind(user_id)
        .bind(cutoff_rediscover)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        let rediscover = rediscover_rows
            .into_iter()
            .map(
                |(id, title, artist, album, dur, last_played, play_count, like, has_art)| {
                    let is_liked = like == Some(1);
                    let days_ago = last_played
                        .map(|lp| (now - lp).max(0) / 86400)
                        .unwrap_or(30);

                    let reason = if is_liked && last_played.is_some() && days_ago >= 60 {
                        format!(
                            "Liked song · Not heard in {} months",
                            (days_ago / 30).max(2)
                        )
                    } else if is_liked {
                        "One of your liked tracks".to_string()
                    } else if last_played.is_none() {
                        "Previously played from your library".to_string()
                    } else if days_ago >= 90 {
                        format!(
                            "Played often · Unplayed for {} months",
                            (days_ago / 30).max(3)
                        )
                    } else {
                        format!("Unheard in {} days", days_ago)
                    };

                    RediscoverTrackItem {
                        cover_art_url: if has_art == 1 {
                            Some(format!("/api/cover/{}", id))
                        } else {
                            None
                        },
                        track_id: id,
                        title,
                        artist: artist.unwrap_or_else(|| "Unknown Artist".into()),
                        album,
                        duration_secs: dur,
                        last_played_at: last_played,
                        play_count: play_count.unwrap_or(0),
                        is_liked,
                        explanation: FlowExplanation {
                            primary: if is_liked { "liked" } else { "rediscovery" }.into(),
                            is_liked,
                            play_count,
                            days_since_played: last_played.map(|lp| (now - lp).max(0) / 86400),
                            selected_at: Some(now),
                            ..Default::default()
                        },
                        reason,
                    }
                },
            )
            .collect();

        // 4. Keep stable picks until listening changes or the short cache expires.
        let history_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM playback_history WHERE user_id = ?")
                .bind(user_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
        let mut picks = self.home_picks.lock().await;
        if picks
            .get(user_id)
            .map(|(time, count, _)| now - time >= 60 || *count != history_count)
            .unwrap_or(true)
        {
            let for_you = self
                .generate_playback_tracks_after(user_id, 1.0, 8, &[], &[])
                .await?
                .into_iter()
                .map(|t| RecommendedTrackDetail {
                    track_id: t.track_id,
                    title: t.title,
                    artist_name: Some(t.artist),
                    album_title: t.album,
                    genre_name: None,
                    duration_secs: t.duration_secs,
                    score: t.score,
                    explanation: Some(t.explanation),
                    is_discovery: t.reason_category == "library_exploration",
                    reasons: vec![crate::recommendations::scoring::RecommendationReason {
                        factor: t.reason_category,
                        description: t.reason,
                        weight: t.score,
                    }],
                })
                .collect();
            picks.insert(user_id.into(), (now, history_count, for_you));
        }
        let for_you = picks.get(user_id).unwrap().2.clone();
        drop(picks);

        // Only real library files can provide a library spotlight. Online rows
        // created for metadata/history are not an owned album collection.
        type LibraryMomentRow = (String, String, Option<String>, Option<String>, i64, Option<i64>, String);
        let library_moment_row: Option<LibraryMomentRow> =
            sqlx::query_as(
                "SELECT al.id,al.title,a.name,al.cover_art_path,COUNT(t.id),al.release_year,
                    (SELECT t2.id FROM tracks t2 WHERE t2.album_id=al.id
                     AND lower(t2.format)!='online' AND t2.file_path!='' AND t2.file_path NOT LIKE 'online://%'
                     AND t2.file_path NOT LIKE 'http://%' AND t2.file_path NOT LIKE 'https://%'
                     ORDER BY t2.has_cover_art DESC,COALESCE(t2.disc_number,1),COALESCE(t2.track_number,2147483647),t2.id LIMIT 1)
                 FROM albums al JOIN tracks t ON t.album_id=al.id
                 LEFT JOIN artists a ON a.id=al.artist_id
                 WHERE lower(t.format)!='online' AND t.file_path!='' AND t.file_path NOT LIKE 'online://%'
                   AND t.file_path NOT LIKE 'http://%' AND t.file_path NOT LIKE 'https://%'
                 GROUP BY al.id ORDER BY RANDOM() LIMIT 1"
            ).fetch_optional(&self.pool).await?;
        let library_moment =
            library_moment_row.map(|(aid, atitle, aname, art_path, count, year, first)| {
                LibraryMoment {
                    album_id: aid,
                    first_track_id: Some(first),
                    album_title: atitle,
                    artist_name: aname.unwrap_or_else(|| "Various Artists".into()),
                    cover_art_path: art_path,
                    total_tracks: count,
                    release_year: year,
                    subtitle: "Revisit an album from your library".into(),
                }
            });

        // 6. Your Week: Last 7 days summary from playback_history
        let week_cutoff = now - (7 * 86400);
        let week_summary: Option<(Option<f64>, Option<i64>, Option<i64>)> = sqlx::query_as(
            "SELECT SUM(h.seconds_listened) as total_secs,
                    COUNT(DISTINCT h.track_id) as tracks_count,
                    COUNT(DISTINCT t.artist_id) as artists_count
             FROM playback_history h
             JOIN tracks t ON t.id = h.track_id
             WHERE h.user_id = ? AND h.started_at >= ?",
        )
        .bind(user_id)
        .bind(week_cutoff)
        .fetch_optional(&self.pool)
        .await?;

        let your_week = week_summary.map(|(s, t, a)| {
            let total_seconds = s.unwrap_or(0.0);
            let hours = (total_seconds / 3600.0).floor() as u64;
            let mins = ((total_seconds % 3600.0) / 60.0).floor() as u64;
            let formatted_time = if hours > 0 {
                format!("{}h {}m", hours, mins)
            } else {
                format!("{}m", mins)
            };

            YourWeekStats {
                total_seconds,
                track_count: t.unwrap_or(0) as u64,
                artist_count: a.unwrap_or(0) as u64,
                formatted_time,
            }
        });

        Ok(HomeOverview {
            flow_session,
            recent_rotation,
            rediscover,
            for_you,
            library_moment,
            your_week,
        })
    }

    pub async fn invalidate_home_picks(&self, user_id: &str) {
        self.home_picks.lock().await.remove(user_id);
    }
}
