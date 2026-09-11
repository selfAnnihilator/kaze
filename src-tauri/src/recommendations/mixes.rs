use super::local::LocalRecommender;
use super::taste::TasteProfileEngine;
use crate::core::command::SmartMixType;
use crate::core::error::{AppError, AppResult};
use crate::database::models::PlaylistRecord;
use crate::database::repositories::{PlaylistRepository, SqlitePlaylistRepository};
use chrono::Utc;
use sqlx::SqlitePool;
use std::collections::HashSet;

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
        let mix_id = format!("mix_{}", uuid::Uuid::new_v4());

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

        let playlist = PlaylistRecord {
            id: mix_id.clone(),
            name,
            description,
            is_smart_mix: 1,
            mix_type: Some(mix_type_str),
            generation_reason: Some(generation_reason),
            expires_at: Some(now + (7 * 86400)), // Smart mixes default to 7-day freshness
            created_at: now,
            updated_at: now,
        };

        // Persist playlist record
        self.playlist_repo.create_playlist(&playlist).await?;

        // Persist playlist tracks
        self.playlist_repo.set_tracks(&mix_id, &track_ids).await?;

        Ok(playlist)
    }

    /// Daily mix: 60% high-affinity tracks, 20% forgotten favorites, 20% discovery picks.
    async fn generate_daily_mix(&self) -> AppResult<Vec<String>> {
        let recs = self.recommender.recommend(30).await?;
        let mut track_ids = Vec::new();
        let mut seen = HashSet::new();

        for r in recs {
            if seen.insert(r.track_id.clone()) {
                track_ids.push(r.track_id);
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

        if rows.is_empty() {
            // Fallback: random tracks
            let fallback: Vec<String> = sqlx::query_scalar(
                "SELECT id FROM tracks ORDER BY RANDOM() LIMIT 25"
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AppError::Database(e.to_string()))?;
            return Ok(fallback);
        }

        Ok(rows)
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

        let discovery_tracks: Vec<String> = recs
            .into_iter()
            .filter(|r| r.is_discovery)
            .take(25)
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
            .map_err(|e| AppError::Database(e.to_string()))?;

            return Ok(fallback);
        }

        Ok(discovery_tracks)
    }

    /// Checks if smart mixes exist, and if none exist or only very few, automatically
    /// generates a rich set of starter smart mixes based on user's library genres and folders.
    pub async fn ensure_default_mixes(&self) -> AppResult<Vec<PlaylistRecord>> {
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
        let _ = self.generate_mix(&SmartMixType::Daily).await;

        // 2. Local Discoveries
        let _ = self.generate_mix(&SmartMixType::Discovery).await;

        // 3. Check for specific prevalent subfolders/genres: Phonk & Lo-Fi
        let phonk_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tracks t LEFT JOIN genres g ON g.id = t.genre_id \
             WHERE LOWER(COALESCE(g.name, '')) LIKE '%phonk%' OR LOWER(t.file_path) LIKE '%/phonk/%'"
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        if phonk_count > 0 {
            let _ = self.generate_mix(&SmartMixType::Genre("Phonk".to_string())).await;
        }

        let lofi_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tracks t LEFT JOIN genres g ON g.id = t.genre_id \
             WHERE LOWER(COALESCE(g.name, '')) LIKE '%lofi%' OR LOWER(COALESCE(g.name, '')) LIKE '%lo-fi%' OR LOWER(t.file_path) LIKE '%/lofi/%'"
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        if lofi_count > 0 {
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
            let _ = self.generate_mix(&SmartMixType::Genre(genre)).await;
        }

        self.playlist_repo.get_smart_mixes().await
    }
}
