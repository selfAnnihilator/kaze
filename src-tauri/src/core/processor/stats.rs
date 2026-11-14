//! Processor handlers for listening statistics and rankings.

use super::routing::misrouted;
use super::*;

impl CoreProcessor {
    pub(super) async fn stats_query(&self, query: Query) -> AppResult<QueryResponse> {
        match query {
            Query::GetTopRankings {
                window,
                entity,
                limit,
            } => {
                let user_id = self.current_user_id().await;
                let ranked = self
                    .ranking_engine
                    .get_rankings_for_user(Some(&user_id), window, entity, limit)
                    .await?;
                let val = match ranked {
                    crate::ranking::RankedOutput::Tracks(items) => items
                        .into_iter()
                        .filter_map(|i| serde_json::to_value(i).ok())
                        .collect(),
                    crate::ranking::RankedOutput::Artists(items) => items
                        .into_iter()
                        .filter_map(|i| serde_json::to_value(i).ok())
                        .collect(),
                    crate::ranking::RankedOutput::Albums(items) => items
                        .into_iter()
                        .filter_map(|i| serde_json::to_value(i).ok())
                        .collect(),
                    crate::ranking::RankedOutput::Genres(items) => items
                        .into_iter()
                        .filter_map(|i| serde_json::to_value(i).ok())
                        .collect(),
                };
                Ok(QueryResponse::Rankings(
                    self.online_catalog
                        .present(serde_json::Value::Array(val))
                        .await?
                        .as_array()
                        .cloned()
                        .unwrap_or_default(),
                ))
            }
            Query::GetStatsOverview { year, month } => {
                let current_user_guard = self.current_user.read().await;
                let user_id = current_user_guard
                    .as_ref()
                    .map(|u| u.id.as_str())
                    .unwrap_or("default");
                let app_start_date = self.get_or_init_app_start_date().await;
                let user_joined_date = current_user_guard
                    .as_ref()
                    .map(|u| u.created_at)
                    .unwrap_or(app_start_date);
                let weights = self.config.read().await.ranking.clone();
                let overview = self
                    .stats_repo
                    .get_stats_overview(
                        user_id,
                        user_joined_date,
                        app_start_date,
                        year,
                        month,
                        &weights,
                    )
                    .await?;
                let val = serde_json::to_value(&overview).unwrap_or_default();
                Ok(QueryResponse::StatsOverview(val))
            }
            other => Err(misrouted(&other)),
        }
    }
}
