use crate::config::RankingWeightsConfig;
use crate::core::error::AppResult;
use crate::core::query::{RankingEntity, TimeWindow};
use crate::database::repositories::{
    RankedAlbumItem, RankedArtistItem, RankedGenreItem, RankedTrackItem, StatsRepository,
};
use chrono::{Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "entity", content = "items")]
pub enum RankedOutput {
    Tracks(Vec<RankedTrackItem>),
    Artists(Vec<RankedArtistItem>),
    Albums(Vec<RankedAlbumItem>),
    Genres(Vec<RankedGenreItem>),
}

pub struct RankingEngine {
    stats_repo: Arc<dyn StatsRepository>,
    config: Arc<RwLock<RankingWeightsConfig>>,
}

impl RankingEngine {
    pub fn new(
        stats_repo: Arc<dyn StatsRepository>,
        config: RankingWeightsConfig,
    ) -> Self {
        Self {
            stats_repo,
            config: Arc::new(RwLock::new(config)),
        }
    }

    /// Computes the start timestamp in seconds for a given TimeWindow.
    pub fn compute_window_start(window: TimeWindow) -> Option<i64> {
        let now = Utc::now();
        match window {
            TimeWindow::Today => {
                let start_of_day = now
                    .date_naive()
                    .and_hms_opt(0, 0, 0)
                    .unwrap_or(now.naive_utc())
                    .and_utc();
                Some(start_of_day.timestamp())
            }
            TimeWindow::Last7Days => Some((now - ChronoDuration::days(7)).timestamp()),
            TimeWindow::Last30Days => Some((now - ChronoDuration::days(30)).timestamp()),
            TimeWindow::Last6Months => Some((now - ChronoDuration::days(180)).timestamp()),
            TimeWindow::LastYear => Some((now - ChronoDuration::days(365)).timestamp()),
            TimeWindow::AllTime => None,
        }
    }

    /// Retrieves ranked entities according to multi-factor scoring formula for a specific user.
    pub async fn get_rankings_for_user(
        &self,
        user_id: Option<&str>,
        window: TimeWindow,
        entity: RankingEntity,
        limit: u32,
    ) -> AppResult<RankedOutput> {
        let start = Self::compute_window_start(window);
        let weights = self.config.read().await.clone();

        match entity {
            RankingEntity::Tracks => {
                let items = self.stats_repo.get_ranked_tracks_for_user(user_id, start, &weights, limit).await?;
                Ok(RankedOutput::Tracks(items))
            }
            RankingEntity::Artists => {
                let items = self.stats_repo.get_ranked_artists_for_user(user_id, start, &weights, limit).await?;
                Ok(RankedOutput::Artists(items))
            }
            RankingEntity::Albums => {
                let items = self.stats_repo.get_ranked_albums_for_user(user_id, start, &weights, limit).await?;
                Ok(RankedOutput::Albums(items))
            }
            RankingEntity::Genres => {
                let items = self.stats_repo.get_ranked_genres_for_user(user_id, start, &weights, limit).await?;
                Ok(RankedOutput::Genres(items))
            }
        }
    }

    /// Retrieves ranked entities according to multi-factor scoring formula (defaults to default user).
    pub async fn get_rankings(
        &self,
        window: TimeWindow,
        entity: RankingEntity,
        limit: u32,
    ) -> AppResult<RankedOutput> {
        self.get_rankings_for_user(None, window, entity, limit).await
    }
}
