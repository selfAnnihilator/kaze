use super::local::LocalRecommender;
use super::taste::TasteProfileEngine;
use crate::core::command::SmartMixType;
use crate::core::error::{AppError, AppResult};
use crate::database::models::PlaylistRecord;
use crate::database::repositories::{PlaylistRepository, SqlitePlaylistRepository};
use chrono::Utc;
use sqlx::SqlitePool;
use std::collections::HashSet;

pub async fn ensure_online_track(
    pool: &SqlitePool,
    id: &str,
    title: &str,
    artist: Option<&str>,
    album: Option<&str>,
    duration_secs: Option<f64>,
    cover_art_url: Option<&str>,
    preview_url: Option<&str>,
) -> AppResult<()> {
    let now = chrono::Utc::now().timestamp();

    // 1. Ensure artist exists if given
    let artist_id = if let Some(a_name) = artist {
        let clean = a_name.trim();
        if !clean.is_empty() {
            let normalized = clean.to_lowercase();
            if let Some((existing_id,)) = sqlx::query_as::<_, (String,)>(
                "SELECT id FROM artists WHERE normalized_name = ?"
            )
            .bind(&normalized)
            .fetch_optional(pool)
            .await
            .unwrap_or(None) {
                Some(existing_id)
            } else {
                let a_id = uuid::Uuid::new_v4().to_string();
                let _ = sqlx::query(
                    "INSERT INTO artists (id, name, normalized_name, created_at)
                     VALUES (?, ?, ?, ?)"
                )
                .bind(&a_id)
                .bind(clean)
                .bind(&normalized)
                .bind(now)
                .execute(pool)
                .await;
                Some(a_id)
            }
        } else {
            None
        }
    } else {
        None
    };

    // 2. Ensure album exists if given
    let album_id = if let Some(al_title) = album {
        let clean = al_title.trim();
        if !clean.is_empty() {
            let normalized = clean.to_lowercase();
            if let Some((existing_id,)) = sqlx::query_as::<_, (String,)>(
                "SELECT id FROM albums WHERE normalized_title = ? AND (artist_id IS ? OR artist_id = ?)"
            )
            .bind(&normalized)
            .bind(&artist_id)
            .bind(&artist_id)
            .fetch_optional(pool)
            .await
            .unwrap_or(None) {
                Some(existing_id)
            } else {
                let al_id = uuid::Uuid::new_v4().to_string();
                let _ = sqlx::query(
                    "INSERT INTO albums (id, title, normalized_title, artist_id, created_at)
                     VALUES (?, ?, ?, ?, ?)"
                )
                .bind(&al_id)
                .bind(clean)
                .bind(&normalized)
                .bind(&artist_id)
                .bind(now)
                .execute(pool)
                .await;
                Some(al_id)
            }
        } else {
            None
        }
    } else {
        None
    };

    // 3. Ensure track in external_tracks table
    let _ = sqlx::query(
        "INSERT INTO external_tracks (id, provider, provider_id, title, artist, album, duration_secs, cover_art_url, preview_url, match_status, created_at)
         VALUES (?, 'online', ?, ?, ?, ?, ?, ?, ?, 'NOT_FOUND', ?)
         ON CONFLICT(id) DO UPDATE SET
             title = excluded.title,
             artist = excluded.artist,
             album = excluded.album,
             duration_secs = excluded.duration_secs,
             cover_art_url = excluded.cover_art_url,
             preview_url = excluded.preview_url"
    )
    .bind(id)
    .bind(id)
    .bind(title)
    .bind(artist.unwrap_or("Unknown Artist"))
    .bind(album)
    .bind(duration_secs)
    .bind(cover_art_url)
    .bind(preview_url)
    .bind(now)
    .execute(pool)
    .await;

    // 4. Ensure or update track in tracks table
    let fake_path = format!("online://{}", id);
    let dur = match duration_secs {
        Some(d) if d > 0.0 => d,
        _ => 210.0,
    };
    let has_cov = if cover_art_url.is_some() { 1 } else { 0 };

    let _ = sqlx::query(
        "INSERT INTO tracks (id, file_path, file_size, modified_timestamp, title, normalized_title, artist_id, album_id, duration_secs, format, has_cover_art, created_at, updated_at)
         VALUES (?, ?, 0, ?, ?, ?, ?, ?, ?, 'online', ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
             title = excluded.title,
             normalized_title = excluded.normalized_title,
             artist_id = COALESCE(excluded.artist_id, tracks.artist_id),
             album_id = COALESCE(excluded.album_id, tracks.album_id),
             duration_secs = CASE WHEN excluded.duration_secs > 0 THEN excluded.duration_secs ELSE tracks.duration_secs END,
             format = 'online',
             has_cover_art = excluded.has_cover_art,
             updated_at = excluded.updated_at"
    )
    .bind(id)
    .bind(&fake_path)
    .bind(now)
    .bind(title)
    .bind(title.to_lowercase())
    .bind(&artist_id)
    .bind(&album_id)
    .bind(dur)
    .bind(has_cov)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await;

    Ok(())
}

pub async fn repair_legacy_online_tracks(pool: &SqlitePool) -> AppResult<()> {
    let rows: Vec<(String, String, String, Option<String>, Option<f64>, Option<String>, Option<String>)> =
        sqlx::query_as(
            "SELECT ext.id, ext.title, ext.artist, ext.album, ext.duration_secs, ext.cover_art_url, ext.preview_url
             FROM external_tracks ext
             JOIN tracks t ON t.id = ext.id
             WHERE t.title LIKE 'itunes:%' OR t.title LIKE 'online:%' OR t.artist_id IS NULL OR t.duration_secs = 0.0"
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default();

    for (id, title, artist, album, duration_secs, cover_art_url, preview_url) in rows {
        let _ = ensure_online_track(
            pool,
            &id,
            &title,
            Some(&artist),
            album.as_deref(),
            duration_secs,
            cover_art_url.as_deref(),
            preview_url.as_deref(),
        ).await;
    }

    // Ensure external tracks incorrectly matched against online track placeholders are reset
    let _ = sqlx::query(
        "UPDATE external_tracks SET match_status = 'NOT_FOUND', matched_local_track_id = NULL
         WHERE matched_local_track_id LIKE 'itunes:%' OR matched_local_track_id LIKE 'online:%'"
    )
    .execute(pool)
    .await;

    Ok(())
}

pub struct SmartMixGenerator {
    pool: SqlitePool,
    playlist_repo: SqlitePlaylistRepository,
    taste_engine: TasteProfileEngine,
    recommender: LocalRecommender,
}

impl SmartMixGenerator {
    pub fn new(pool: SqlitePool) -> Self {
        let playlist_repo = SqlitePlaylistRepository::new(pool.clone());
        let taste_engine = TasteProfileEngine::new(pool.clone());
        let recommender = LocalRecommender::new(pool.clone());
        Self {
            pool,
            playlist_repo,
            taste_engine,
            recommender,
        }
    }

    /// Generates a smart mix according to the specified type and persists it as a smart playlist.
    pub async fn generate_mix(&self, mix_type: &SmartMixType) -> AppResult<PlaylistRecord> {
        let now = Utc::now().timestamp();
        let (name, description, mix_type_str, generation_reason, track_ids) = match mix_type {
            SmartMixType::Daily => {
                let track_ids = self.generate_daily_mix().await?;
                (
                    "Daily Mix".to_string(),
                    Some("A personalized daily blend of your favorites, forgotten gems, and fresh picks".to_string()),
                    "daily".to_string(),
                    "Generated from your recent taste profile, featuring top affinities, forgotten favorites, and local exploration".to_string(),
                    track_ids,
                )
            }
            SmartMixType::OnRepeat => {
                let track_ids = self.generate_on_repeat_mix().await?;
                (
                    "On Repeat".to_string(),
                    Some("Your most-played and highest-completion tracks from the last 14 days".to_string()),
                    "on_repeat".to_string(),
                    "Built from tracks with frequent qualified plays and high completion rates over the past 2 weeks".to_string(),
                    track_ids,
                )
            }
            SmartMixType::ForgottenFavorites => {
                let track_ids = self.generate_forgotten_favorites_mix().await?;
                (
                    "Forgotten Favorites".to_string(),
                    Some("Beloved tracks and past favorites you haven't played recently".to_string()),
                    "forgotten_favorites".to_string(),
                    "Discovered from your history and likes that have not been played in over 30 days".to_string(),
                    track_ids,
                )
            }
            SmartMixType::Genre(genre_name) => {
                let track_ids = self.generate_genre_mix(genre_name).await?;
                (
                    format!("{} Mix", genre_name),
                    Some(format!("Curated tracks and highlights from the {} genre", genre_name)),
                    format!("genre:{}", genre_name.to_lowercase()),
                    format!("Personalized selection of top-ranked and discovery tracks in {}", genre_name),
                    track_ids,
                )
            }
            SmartMixType::Artist(artist_name) => {
                let track_ids = self.generate_artist_mix(artist_name).await?;
                (
                    format!("{} Radio", artist_name),
                    Some(format!("Tracks by {} and artists sharing similar musical styles", artist_name)),
                    format!("artist:{}", artist_name.to_lowercase()),
                    format!("Curated playlist centered around {} and related artist affinities", artist_name),
                    track_ids,
                )
            }
            SmartMixType::LateNight => {
                let track_ids = self.generate_late_night_mix().await?;
                (
                    "Late Night Chill".to_string(),
                    Some("Relaxing, deep-cut tracks tailored for evening and late-night listening".to_string()),
                    "late_night".to_string(),
                    "Selection of mellow, unhurried tracks and deep cuts".to_string(),
                    track_ids,
                )
            }
            SmartMixType::Discovery => {
                let track_ids = self.generate_discovery_mix().await?;
                (
                    "Local Discoveries".to_string(),
                    Some("Unheard gems hidden in your local music library matching your taste".to_string()),
                    "discovery".to_string(),
                    "Local tracks with 0 plays that align with your favorite artists and genres".to_string(),
                    track_ids,
                )
            }
        };

        // Reuse existing mix ID and record for this smart mix so we update its contents in-place,
        // without creating a duplicate mix collection beside it
        let existing = self.playlist_repo.find_smart_mix(&mix_type_str, &name).await?;
        let mix_id = match existing {
            Some(ref e) => e.id.clone(),
            None => format!("mix_{}", uuid::Uuid::new_v4()),
        };
        let created_at = existing.as_ref().map(|e| e.created_at).unwrap_or(now);
        let final_name = existing.as_ref().map(|e| e.name.clone()).unwrap_or(name);

        // Daily mixes expire every 24 hours (86,400s); other mixes change weekly (7 days / 604,800s)
        let expires_at = match mix_type {
            SmartMixType::Daily => Some(now + 86400),
            _ => Some(now + (7 * 86400)),
        };

        let playlist = PlaylistRecord {
            id: mix_id.clone(),
            name: final_name,
            description,
            is_smart_mix: 1,
            mix_type: Some(mix_type_str.clone()),
            generation_reason: Some(generation_reason),
            expires_at,
            created_at,
            updated_at: now,
        };

        // Persist playlist record (updates existing in-place)
        self.playlist_repo.create_playlist(&playlist).await?;

        // Update the contents inside the mix (replaces tracks for this mix_id)
        self.playlist_repo.set_tracks(&mix_id, &track_ids).await?;

        // Clean up any historical duplicate entries with the same mix_type or name
        let _ = self.playlist_repo.delete_duplicate_smart_mixes(&mix_type_str, &playlist.name, &mix_id).await;

        Ok(playlist)
    }

    /// Daily mix: 60% high-affinity tracks, 20% forgotten favorites, 20% discovery & online picks.
    async fn generate_daily_mix(&self) -> AppResult<Vec<String>> {
        let recs = self.recommender.recommend(20).await?;
        let mut track_ids = Vec::new();
        let mut seen = HashSet::new();

        for r in recs {
            if seen.insert(r.track_id.clone()) {
                track_ids.push(r.track_id);
            }
        }

        // Blend in online discovery songs from external_tracks
        let online_candidates: Vec<(String, String, Option<String>, Option<String>, Option<f64>, Option<String>, Option<String>)> =
            sqlx::query_as(
                "SELECT id, title, artist, album, duration_secs, cover_art_url, preview_url
                 FROM external_tracks
                 WHERE match_status != 'IGNORE'
                 ORDER BY RANDOM()
                 LIMIT 12"
            )
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

        for (oid, otitle, oartist, oalbum, odur, ocover, opreview) in online_candidates {
            if seen.insert(oid.clone()) {
                let _ = ensure_online_track(
                    &self.pool,
                    &oid,
                    &otitle,
                    oartist.as_deref(),
                    oalbum.as_deref(),
                    odur,
                    ocover.as_deref(),
                    opreview.as_deref(),
                ).await;
                track_ids.push(oid);
            }
        }

        // If library has fewer recommendations, pad with random unplayed tracks
        if track_ids.len() < 10 {
            let padding: Vec<String> = sqlx::query_scalar(
                "SELECT id FROM tracks ORDER BY RANDOM() LIMIT 20"
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

            for id in padding {
                if seen.insert(id.clone()) {
                    track_ids.push(id);
                }
            }
        }

        Ok(track_ids)
    }

    /// On repeat: tracks played often and completed in the last 14 days.
    async fn generate_on_repeat_mix(&self) -> AppResult<Vec<String>> {
        let cutoff = Utc::now().timestamp() - (14 * 86400);

        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT h.track_id
             FROM playback_history h
             WHERE h.started_at >= ?
             GROUP BY h.track_id
             HAVING SUM(h.completed) >= 1 OR COUNT(h.id) >= 2
             ORDER BY COUNT(h.id) DESC, SUM(h.percentage_listened) DESC
             LIMIT 25"
        )
        .bind(cutoff)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        if rows.is_empty() {
            // Fallback 1: top tracks from track_statistics
            let fallback: Vec<String> = sqlx::query_scalar(
                "SELECT track_id FROM track_statistics WHERE manual_like != -1 ORDER BY play_count DESC LIMIT 25"
            )
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

            if !fallback.is_empty() {
                return Ok(fallback);
            }

            // Fallback 2: random tracks from tracks
            let random_fallback: Vec<String> = sqlx::query_scalar(
                "SELECT id FROM tracks ORDER BY RANDOM() LIMIT 25"
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

            return Ok(random_fallback);
        }

        Ok(rows)
    }

    /// Forgotten favorites: played or liked, but unplayed in over 30 days.
    async fn generate_forgotten_favorites_mix(&self) -> AppResult<Vec<String>> {
        let cutoff = Utc::now().timestamp() - (30 * 86400);

        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT ts.track_id
             FROM track_statistics ts
             WHERE (ts.play_count >= 2 OR ts.manual_like = 1)
               AND (ts.last_played_at IS NULL OR ts.last_played_at < ?)
               AND ts.manual_like != -1
             ORDER BY ts.play_count DESC, ts.total_time_listened DESC
             LIMIT 25"
        )
        .bind(cutoff)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        if rows.is_empty() {
            // Fallback: tracks from /fav/ folder or with manual_like = 1, or random sample
            let fallback: Vec<String> = sqlx::query_scalar(
                "SELECT t.id
                 FROM tracks t
                 LEFT JOIN track_statistics ts ON ts.track_id = t.id
                 WHERE (LOWER(t.file_path) LIKE '%/fav/%' OR ts.manual_like = 1)
                   AND COALESCE(ts.manual_like, 0) != -1
                 ORDER BY RANDOM()
                 LIMIT 25"
            )
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

            if !fallback.is_empty() {
                return Ok(fallback);
            }

            let general: Vec<String> = sqlx::query_scalar(
                "SELECT id FROM tracks ORDER BY RANDOM() LIMIT 25"
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;

            return Ok(general);
        }

        Ok(rows)
    }

    /// Genre mix: tracks matching genre, ordered by performance and affinity.
    async fn generate_genre_mix(&self, genre_name: &str) -> AppResult<Vec<String>> {
        let clean_genre = genre_name.trim();
        let pattern = format!("%{}%", clean_genre.to_lowercase());
        let folder_pattern = format!("%/{}%", clean_genre.to_lowercase().replace('-', ""));
        let folder_pattern_raw = format!("%/{}%", clean_genre.to_lowercase());
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT t.id
             FROM tracks t
             LEFT JOIN genres g ON g.id = t.genre_id
             LEFT JOIN track_statistics ts ON ts.track_id = t.id
             WHERE (
                 LOWER(COALESCE(g.name, '')) LIKE ?
                 OR LOWER(t.file_path) LIKE ?
                 OR LOWER(t.file_path) LIKE ?
             )
             AND COALESCE(ts.manual_like, 0) != -1
             ORDER BY COALESCE(ts.play_count, 0) DESC, RANDOM()
             LIMIT 30"
        )
        .bind(&pattern)
        .bind(&folder_pattern)
        .bind(&folder_pattern_raw)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        let mut track_ids = if rows.is_empty() {
            // Fallback: random tracks
            sqlx::query_scalar(
                "SELECT id FROM tracks ORDER BY RANDOM() LIMIT 25"
            )
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default()
        } else {
            rows
        };

        let mut seen: HashSet<String> = track_ids.iter().cloned().collect();

        // Blend in online songs for this genre from external_tracks
        let online_candidates: Vec<(String, String, Option<String>, Option<String>, Option<f64>, Option<String>, Option<String>)> =
            sqlx::query_as(
                "SELECT id, title, artist, album, duration_secs, cover_art_url, preview_url
                 FROM external_tracks
                 WHERE (LOWER(COALESCE(genre, '')) LIKE ? OR LOWER(title) LIKE ? OR LOWER(artist) LIKE ?)
                   AND match_status != 'IGNORE'
                 ORDER BY RANDOM()
                 LIMIT 15"
            )
            .bind(&pattern)
            .bind(&pattern)
            .bind(&pattern)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

        for (oid, otitle, oartist, oalbum, odur, ocover, opreview) in online_candidates {
            if seen.insert(oid.clone()) {
                let _ = ensure_online_track(
                    &self.pool,
                    &oid,
                    &otitle,
                    oartist.as_deref(),
                    oalbum.as_deref(),
                    odur,
                    ocover.as_deref(),
                    opreview.as_deref(),
                ).await;
                track_ids.push(oid);
            }
        }

        Ok(track_ids)
    }

    /// Artist mix: tracks by artist, plus related artists sharing genres.
    async fn generate_artist_mix(&self, artist_name: &str) -> AppResult<Vec<String>> {
        let mut track_ids: Vec<String> = Vec::new();

        // 1. Direct tracks by artist
        let direct: Vec<String> = sqlx::query_scalar(
            "SELECT t.id
             FROM tracks t
             JOIN artists a ON a.id = t.artist_id
             LEFT JOIN track_statistics ts ON ts.track_id = t.id
             WHERE LOWER(a.name) = LOWER(?)
               AND COALESCE(ts.manual_like, 0) != -1
             ORDER BY RANDOM()
             LIMIT 15"
        )
        .bind(artist_name)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        track_ids.extend(direct);

        // 2. Related tracks from shared genres
        let related: Vec<String> = sqlx::query_scalar(
            "SELECT t.id
             FROM tracks t
             JOIN artists a ON a.id = t.artist_id
             JOIN genres g ON g.id = t.genre_id
             LEFT JOIN track_statistics ts ON ts.track_id = t.id
             WHERE LOWER(a.name) != LOWER(?)
               AND g.id IN (
                   SELECT t2.genre_id
                   FROM tracks t2
                   JOIN artists a2 ON a2.id = t2.artist_id
                   WHERE LOWER(a2.name) = LOWER(?) AND t2.genre_id IS NOT NULL
               )
               AND COALESCE(ts.manual_like, 0) != -1
             ORDER BY RANDOM()
             LIMIT 15"
        )
        .bind(artist_name)
        .bind(artist_name)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        track_ids.extend(related);
        Ok(track_ids)
    }

    /// Late night mix: mellow / calm selections.
    async fn generate_late_night_mix(&self) -> AppResult<Vec<String>> {
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT t.id
             FROM tracks t
             LEFT JOIN genres g ON g.id = t.genre_id
             LEFT JOIN track_statistics ts ON ts.track_id = t.id
             WHERE COALESCE(ts.manual_like, 0) != -1
               AND (
                   LOWER(COALESCE(g.name, '')) LIKE '%ambient%'
                   OR LOWER(COALESCE(g.name, '')) LIKE '%chill%'
                   OR LOWER(COALESCE(g.name, '')) LIKE '%acoustic%'
                   OR LOWER(COALESCE(g.name, '')) LIKE '%jazz%'
                   OR LOWER(COALESCE(g.name, '')) LIKE '%downtempo%'
                   OR t.duration_secs >= 240.0
               )
             ORDER BY RANDOM()
             LIMIT 25"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| AppError::Database(e.to_string()))?;

        Ok(rows)
    }

    /// Discovery mix: local tracks with 0 plays that align with user taste.
    async fn generate_discovery_mix(&self) -> AppResult<Vec<String>> {
        let profile = self.taste_engine.compute_taste_profile().await?;
        let recs = self.recommender.recommend_with_profile(&profile, 50).await?;

        let mut discovery_tracks: Vec<String> = recs
            .into_iter()
            .filter(|r| r.is_discovery)
            .take(20)
            .map(|r| r.track_id)
            .collect();

        if discovery_tracks.is_empty() {
            // Fallback: any unplayed tracks
            let fallback: Vec<String> = sqlx::query_scalar(
                "SELECT t.id
                 FROM tracks t
                 LEFT JOIN track_statistics ts ON ts.track_id = t.id
                 WHERE (ts.play_count IS NULL OR ts.play_count = 0)
                   AND COALESCE(ts.manual_like, 0) != -1
                 ORDER BY RANDOM()
                 LIMIT 25"
            )
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

            discovery_tracks = fallback;
        }

        let mut seen: HashSet<String> = discovery_tracks.iter().cloned().collect();

        // Blend in online discovery tracks from external_tracks
        let online_candidates: Vec<(String, String, Option<String>, Option<String>, Option<f64>, Option<String>, Option<String>)> =
            sqlx::query_as(
                "SELECT id, title, artist, album, duration_secs, cover_art_url, preview_url
                 FROM external_tracks
                 WHERE match_status != 'IGNORE'
                 ORDER BY RANDOM()
                 LIMIT 15"
            )
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

        for (oid, otitle, oartist, oalbum, odur, ocover, opreview) in online_candidates {
            if seen.insert(oid.clone()) {
                let _ = ensure_online_track(
                    &self.pool,
                    &oid,
                    &otitle,
                    oartist.as_deref(),
                    oalbum.as_deref(),
                    odur,
                    ocover.as_deref(),
                    opreview.as_deref(),
                ).await;
                discovery_tracks.push(oid);
            }
        }

        Ok(discovery_tracks)
    }

    /// Checks if smart mixes exist, and if none exist or only very few, automatically
    /// generates a rich set of starter smart mixes based on user's library genres and folders.
    pub async fn ensure_default_mixes(&self) -> AppResult<Vec<PlaylistRecord>> {
        let now = Utc::now().timestamp();
        let existing = self.playlist_repo.get_smart_mixes().await?;

        // 1. Check for expired smart mixes and automatically regenerate them
        for mix in &existing {
            if let Some(expires_at) = mix.expires_at {
                if now >= expires_at {
                    if let Some(ref mt) = mix.mix_type {
                        let smart_mix_type = match mt.as_str() {
                            "daily" => Some(SmartMixType::Daily),
                            "on_repeat" => Some(SmartMixType::OnRepeat),
                            "forgotten_favorites" => Some(SmartMixType::ForgottenFavorites),
                            "late_night" => Some(SmartMixType::LateNight),
                            "discovery" => Some(SmartMixType::Discovery),
                            s if s.starts_with("genre:") => Some(SmartMixType::Genre(s[6..].to_string())),
                            s if s.starts_with("artist:") => Some(SmartMixType::Artist(s[7..].to_string())),
                            _ => None,
                        };
                        if let Some(smt) = smart_mix_type {
                            let _ = self.generate_mix(&smt).await;
                        }
                    }
                }
            }
        }

        let existing = self.playlist_repo.get_smart_mixes().await?;
        if existing.len() >= 3 {
            return Ok(existing);
        }

        // Check if there are tracks in the library at all
        let total_tracks: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tracks")
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

        if total_tracks == 0 {
            return Ok(existing);
        }

        tracing::info!("Auto-generating recommended smart mixes based on library...");

        // 1. Daily Mix
        if !existing.iter().any(|m| m.mix_type.as_deref() == Some("daily") || m.name.eq_ignore_ascii_case("Daily Mix")) {
            let _ = self.generate_mix(&SmartMixType::Daily).await;
        }

        // 2. Local Discoveries
        if !existing.iter().any(|m| m.mix_type.as_deref() == Some("discovery") || m.name.eq_ignore_ascii_case("Local Discoveries")) {
            let _ = self.generate_mix(&SmartMixType::Discovery).await;
        }

        // 3. Check for specific prevalent subfolders/genres: Phonk & Lo-Fi
        let phonk_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tracks t LEFT JOIN genres g ON g.id = t.genre_id \
             WHERE LOWER(COALESCE(g.name, '')) LIKE '%phonk%' OR LOWER(t.file_path) LIKE '%/phonk/%'"
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        if phonk_count > 0 && !existing.iter().any(|m| m.name.to_lowercase().contains("phonk")) {
            let _ = self.generate_mix(&SmartMixType::Genre("Phonk".to_string())).await;
        }

        let lofi_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tracks t LEFT JOIN genres g ON g.id = t.genre_id \
             WHERE LOWER(COALESCE(g.name, '')) LIKE '%lofi%' OR LOWER(COALESCE(g.name, '')) LIKE '%lo-fi%' OR LOWER(t.file_path) LIKE '%/lofi/%'"
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        if lofi_count > 0 && !existing.iter().any(|m| m.name.to_lowercase().contains("lofi") || m.name.to_lowercase().contains("lo-fi")) {
            let _ = self.generate_mix(&SmartMixType::Genre("Lo-Fi".to_string())).await;
        }

        // 4. Query top library genres
        let top_genres: Vec<String> = sqlx::query_scalar(
            "SELECT g.name \
             FROM tracks t \
             JOIN genres g ON g.id = t.genre_id \
             WHERE LOWER(g.name) NOT LIKE '%phonk%' AND LOWER(g.name) NOT LIKE '%lofi%' \
             GROUP BY g.name \
             HAVING COUNT(t.id) >= 3 \
             ORDER BY COUNT(t.id) DESC \
             LIMIT 3"
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        for genre in top_genres {
            if !existing.iter().any(|m| m.name.to_lowercase().contains(&genre.to_lowercase())) {
                let _ = self.generate_mix(&SmartMixType::Genre(genre)).await;
            }
        }

        self.playlist_repo.get_smart_mixes().await
    }
}
