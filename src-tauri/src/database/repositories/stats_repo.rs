use crate::config::RankingWeightsConfig;
use crate::core::error::{AppError, AppResult};
use crate::database::models::TrackStatisticsRecord;
use async_trait::async_trait;
use chrono::{Datelike, Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

mod types;
pub use types::*;

mod writes;
mod ranked;
mod overview;
mod all_time;

#[async_trait]
pub trait StatsRepository: Send + Sync {
    async fn update_track_playback_stats(
        &self,
        track_id: &str,
        listened_secs: f64,
        is_meaningful: bool,
        completed: bool,
        skipped: bool,
    ) -> AppResult<()>;

    async fn update_track_playback_stats_scoped(
        &self,
        user_id: &str,
        track_id: &str,
        listened_secs: f64,
        is_meaningful: bool,
        completed: bool,
        skipped: bool,
    ) -> AppResult<()>;

    async fn set_track_like(&self, track_id: &str, like_status: i64) -> AppResult<()>;
    async fn set_track_like_scoped(&self, user_id: &str, track_id: &str, like_status: i64) -> AppResult<()>;
    async fn get_track_stats(&self, track_id: &str) -> AppResult<Option<TrackStatisticsRecord>>;
    async fn get_track_stats_scoped(&self, user_id: &str, track_id: &str) -> AppResult<Option<TrackStatisticsRecord>>;

    async fn get_ranked_tracks(
        &self,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedTrackItem>>;

    async fn get_ranked_tracks_for_user(
        &self,
        user_id: Option<&str>,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedTrackItem>>;

    async fn get_ranked_artists(
        &self,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedArtistItem>>;

    async fn get_ranked_artists_for_user(
        &self,
        user_id: Option<&str>,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedArtistItem>>;

    async fn get_ranked_albums(
        &self,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedAlbumItem>>;

    async fn get_ranked_albums_for_user(
        &self,
        user_id: Option<&str>,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedAlbumItem>>;

    async fn get_ranked_genres(
        &self,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedGenreItem>>;

    async fn get_ranked_genres_for_user(
        &self,
        user_id: Option<&str>,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedGenreItem>>;

    async fn get_stats_overview(
        &self,
        user_id: &str,
        user_joined_at: i64,
        app_start_date: i64,
        selected_year: Option<i32>,
        selected_month: Option<u32>,
        weights: &RankingWeightsConfig,
    ) -> AppResult<StatsOverview>;

    async fn get_top_listening_days(
        &self,
        user_id: &str,
        limit: u32,
    ) -> AppResult<Vec<TopListeningDay>>;

    async fn get_ranked_tracks_scoped(
        &self,
        user_id: Option<&str>,
        window_start: Option<i64>,
        window_end: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedTrackItem>>;

    async fn get_ranked_artists_scoped(
        &self,
        user_id: Option<&str>,
        window_start: Option<i64>,
        window_end: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedArtistItem>>;

    async fn get_all_time_ranked_tracks(
        &self,
        user_id: &str,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedTrackItem>>;

    async fn get_all_time_ranked_genres(
        &self,
        user_id: &str,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedGenreItem>>;

    async fn get_all_time_ranked_artists(
        &self,
        user_id: &str,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedArtistItem>>;

    async fn record_daily_playback(
        &self,
        user_id: &str,
        device_id: Option<&str>,
        stat_date: &str,
        playback: DailyPlayback,
    ) -> AppResult<()>;

    async fn backfill_daily_stats_from_history(&self, default_device_id: Option<&str>) -> AppResult<usize>;
}

#[derive(Clone)]
pub struct SqliteStatsRepository {
    pool: SqlitePool,
}

impl SqliteStatsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn resolve_device_id(&self, explicit_id: Option<&str>) -> String {
        if let Some(id) = explicit_id {
            if !id.trim().is_empty() {
                return id.to_string();
            }
        }
        crate::cloud::device::get_or_create_device_id(&self.pool)
            .await
            .unwrap_or_else(|_| "legacy_device".to_string())
    }
}

#[async_trait]
impl StatsRepository for SqliteStatsRepository {
    async fn update_track_playback_stats(
        &self,
        track_id: &str,
        listened_secs: f64,
        is_meaningful: bool,
        completed: bool,
        skipped: bool,
    ) -> AppResult<()> {
        self.update_track_playback_stats_scoped("default", track_id, listened_secs, is_meaningful, completed, skipped).await
    }

    async fn update_track_playback_stats_scoped(
        &self,
        user_id: &str,
        track_id: &str,
        listened_secs: f64,
        is_meaningful: bool,
        completed: bool,
        skipped: bool,
    ) -> AppResult<()> {
        self.update_track_playback_stats_scoped_query(user_id, track_id, listened_secs, is_meaningful, completed, skipped).await
    }

    async fn record_daily_playback(
        &self,
        user_id: &str,
        device_id: Option<&str>,
        stat_date: &str,
        playback: DailyPlayback,
    ) -> AppResult<()> {
        self.record_daily_playback_query(user_id, device_id, stat_date, playback).await
    }

    async fn backfill_daily_stats_from_history(&self, default_device_id: Option<&str>) -> AppResult<usize> {
        self.backfill_daily_stats_from_history_query(default_device_id).await
    }

    async fn set_track_like(&self, track_id: &str, like_status: i64) -> AppResult<()> {
        self.set_track_like_scoped("default", track_id, like_status).await
    }

    async fn set_track_like_scoped(&self, user_id: &str, track_id: &str, like_status: i64) -> AppResult<()> {
        self.set_track_like_scoped_query(user_id, track_id, like_status).await
    }

    async fn get_track_stats(&self, track_id: &str) -> AppResult<Option<TrackStatisticsRecord>> {
        self.get_track_stats_scoped("default", track_id).await
    }

    async fn get_track_stats_scoped(&self, user_id: &str, track_id: &str) -> AppResult<Option<TrackStatisticsRecord>> {
        self.get_track_stats_scoped_query(user_id, track_id).await
    }

    async fn get_ranked_tracks(
        &self,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedTrackItem>> {
        self.get_ranked_tracks_for_user(None, window_start, weights, limit).await
    }

    async fn get_ranked_tracks_for_user(
        &self,
        user_id: Option<&str>,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedTrackItem>> {
        self.get_ranked_tracks_for_user_query(user_id, window_start, weights, limit).await
    }

    async fn get_ranked_artists(
        &self,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedArtistItem>> {
        self.get_ranked_artists_for_user(None, window_start, weights, limit).await
    }

    async fn get_ranked_artists_for_user(
        &self,
        user_id: Option<&str>,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedArtistItem>> {
        self.get_ranked_artists_for_user_query(user_id, window_start, weights, limit).await
    }

    async fn get_ranked_albums(
        &self,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedAlbumItem>> {
        self.get_ranked_albums_for_user(None, window_start, weights, limit).await
    }

    async fn get_ranked_albums_for_user(
        &self,
        user_id: Option<&str>,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedAlbumItem>> {
        self.get_ranked_albums_for_user_query(user_id, window_start, weights, limit).await
    }

    async fn get_ranked_genres(
        &self,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedGenreItem>> {
        self.get_ranked_genres_for_user(None, window_start, weights, limit).await
    }

    async fn get_ranked_genres_for_user(
        &self,
        user_id: Option<&str>,
        window_start: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedGenreItem>> {
        self.get_ranked_genres_for_user_query(user_id, window_start, weights, limit).await
    }

    async fn get_stats_overview(
        &self,
        user_id: &str,
        user_joined_at: i64,
        app_start_date: i64,
        selected_year: Option<i32>,
        selected_month: Option<u32>,
        weights: &RankingWeightsConfig,
    ) -> AppResult<StatsOverview> {
        self.get_stats_overview_query(user_id, user_joined_at, app_start_date, selected_year, selected_month, weights).await
    }

    async fn get_top_listening_days(&self, user_id: &str, limit: u32) -> AppResult<Vec<TopListeningDay>> {
        self.get_top_listening_days_query(user_id, limit).await
    }

    async fn get_ranked_tracks_scoped(
        &self,
        user_id: Option<&str>,
        window_start: Option<i64>,
        window_end: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedTrackItem>> {
        self.get_ranked_tracks_scoped_query(user_id, window_start, window_end, weights, limit).await
    }

    async fn get_ranked_artists_scoped(
        &self,
        user_id: Option<&str>,
        window_start: Option<i64>,
        window_end: Option<i64>,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedArtistItem>> {
        self.get_ranked_artists_scoped_query(user_id, window_start, window_end, weights, limit).await
    }

    async fn get_all_time_ranked_tracks(
        &self,
        user_id: &str,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedTrackItem>> {
        self.get_all_time_ranked_tracks_query(user_id, weights, limit).await
    }

    async fn get_all_time_ranked_artists(
        &self,
        user_id: &str,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedArtistItem>> {
        self.get_all_time_ranked_artists_query(user_id, weights, limit).await
    }

    async fn get_all_time_ranked_genres(
        &self,
        user_id: &str,
        weights: &RankingWeightsConfig,
        limit: u32,
    ) -> AppResult<Vec<RankedGenreItem>> {
        self.get_all_time_ranked_genres_query(user_id, weights, limit).await
    }
}
