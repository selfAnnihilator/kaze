use super::local::LocalRecommender;
use super::taste::TasteProfileEngine;
use crate::core::command::SmartMixType;
use crate::core::error::{AppError, AppResult};
use crate::database::models::PlaylistRecord;
use crate::database::repositories::{PlaylistRepository, SqlitePlaylistRepository};
use chrono::Utc;
use sqlx::SqlitePool;
use std::collections::HashSet;

mod build;
mod families;
mod generators;
mod online_tracks;
pub use online_tracks::*;

/// external_tracks row: id, title, artist, album, duration_secs, cover_art_url, preview_url.
type OnlineCandidateRow = (
    String,
    String,
    Option<String>,
    Option<String>,
    Option<f64>,
    Option<String>,
    Option<String>,
);

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

    /// Checks if smart mixes exist, and if none exist or only very few, automatically
    /// generates a rich set of starter smart mixes based on user's library genres and folders.
    pub async fn ensure_default_mixes(&self, user_id: &str) -> AppResult<Vec<PlaylistRecord>> {
        let now = Utc::now().timestamp();
        let existing = self.playlist_repo.get_smart_mixes_for_user(user_id).await?;

        // 1. Check for expired smart mixes and automatically regenerate them
        for mix in &existing {
            if let Some(expires_at) = mix.expires_at {
                if now >= expires_at {
                    if let Some(ref mt) = mix.mix_type {
                        let smart_mix_type = match mt.as_str() {
                            // The Daily family is regenerated together below.
                            s if s == "daily" || s.starts_with("daily:") => None,
                            "on_repeat" => Some(SmartMixType::OnRepeat),
                            "forgotten_favorites" => Some(SmartMixType::ForgottenFavorites),
                            "late_night" => Some(SmartMixType::LateNight),
                            "discovery" => Some(SmartMixType::Discovery),
                            s if s.starts_with("genre:") => {
                                Some(SmartMixType::Genre(s[6..].to_string()))
                            }
                            s if s.starts_with("artist:") => {
                                Some(SmartMixType::Artist(s[7..].to_string()))
                            }
                            _ => None,
                        };
                        if let Some(smt) = smart_mix_type {
                            let _ = self.generate_mix(user_id, &smt).await;
                        }
                    }
                }
            }
        }

        let existing = self.playlist_repo.get_smart_mixes_for_user(user_id).await?;
        if let Err(error) = self.sync_daily_family(user_id, &existing).await {
            tracing::warn!("Daily mix family refresh failed for {user_id}: {error}");
        }
        let existing = self.playlist_repo.get_smart_mixes_for_user(user_id).await?;
        if existing.len() >= 3 {
            return Ok(existing);
        }

        // Check if there are tracks in the library at all
        let total_tracks: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tracks")
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

        let profile = self.taste_engine.compute_taste_profile(user_id).await?;
        let has_starters = !super::setup::members(&self.pool, user_id, &profile)
            .await?
            .is_empty();
        if total_tracks == 0 && !has_starters {
            return Ok(existing);
        }

        tracing::info!(
            "Auto-generating recommended smart mixes based on library for user {}...",
            user_id
        );

        // 1. Daily Mix
        if !existing.iter().any(|m| {
            m.mix_type
                .as_deref()
                .is_some_and(|t| t == "daily" || t.starts_with("daily:"))
                || m.name.eq_ignore_ascii_case("Daily Mix")
        }) {
            let _ = self.generate_mix(user_id, &SmartMixType::Daily).await;
        }

        // 2. Local Discoveries
        if !existing.iter().any(|m| {
            m.mix_type.as_deref() == Some("discovery")
                || m.name.eq_ignore_ascii_case("Local Discoveries")
        }) {
            let _ = self.generate_mix(user_id, &SmartMixType::Discovery).await;
        }

        // 3. Check for specific prevalent subfolders/genres: Phonk & Lo-Fi
        let phonk_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tracks t LEFT JOIN genres g ON g.id = t.genre_id \
             WHERE LOWER(COALESCE(g.name, '')) LIKE '%phonk%' OR LOWER(t.file_path) LIKE '%/phonk/%'"
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        if phonk_count > 0
            && !existing
                .iter()
                .any(|m| m.name.to_lowercase().contains("phonk"))
        {
            let _ = self
                .generate_mix(user_id, &SmartMixType::Genre("Phonk".to_string()))
                .await;
        }

        let lofi_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM tracks t LEFT JOIN genres g ON g.id = t.genre_id \
             WHERE LOWER(COALESCE(g.name, '')) LIKE '%lofi%' OR LOWER(COALESCE(g.name, '')) LIKE '%lo-fi%' OR LOWER(t.file_path) LIKE '%/lofi/%'"
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        if lofi_count > 0
            && !existing.iter().any(|m| {
                m.name.to_lowercase().contains("lofi") || m.name.to_lowercase().contains("lo-fi")
            })
        {
            let _ = self
                .generate_mix(user_id, &SmartMixType::Genre("Lo-Fi".to_string()))
                .await;
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
             LIMIT 3",
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        for genre in top_genres {
            if !existing
                .iter()
                .any(|m| m.name.to_lowercase().contains(&genre.to_lowercase()))
            {
                let _ = self
                    .generate_mix(user_id, &SmartMixType::Genre(genre))
                    .await;
            }
        }

        self.playlist_repo.get_smart_mixes_for_user(user_id).await
    }
}
