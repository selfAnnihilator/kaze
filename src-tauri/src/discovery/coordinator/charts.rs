//! Top charts and world trending songs.
use super::*;

impl DiscoveryCoordinator {
    /// Fetches trending hits and popular new music across overall charts and targeted user genres.
    pub async fn charts(&self, refresh: bool) -> AppResult<Vec<serde_json::Value>> {
        let Some(providers) = &self.provider_coordinator else {
            return Ok(vec![]);
        };
        if !providers.enabled().await {
            return Ok(vec![]);
        }
        Ok(providers.jiosaavn.charts(refresh).await?)
    }

    pub async fn get_chart_songs(
        &self,
        chart_id: &str,
        limit: usize,
    ) -> AppResult<Vec<DiscoveryRecommendation>> {
        self.chart_songs(chart_id, limit, false).await
    }

    pub(super) async fn chart_songs(
        &self,
        chart_id: &str,
        limit: usize,
        refresh: bool,
    ) -> AppResult<Vec<DiscoveryRecommendation>> {
        let Some(providers) = &self.provider_coordinator else {
            return Ok(vec![]);
        };
        if !providers.enabled().await {
            return Ok(vec![]);
        }
        Self::fetch_chart(providers.clone(), chart_id, limit, refresh).await
    }

    pub(super) async fn fetch_chart(
        providers: Arc<ProviderCoordinator>,
        chart_id: &str,
        limit: usize,
        refresh: bool,
    ) -> AppResult<Vec<DiscoveryRecommendation>> {
        let id = chart_id
            .strip_prefix("jiosaavn:playlist:")
            .ok_or_else(|| crate::core::AppError::Validation("Unknown JioSaavn chart".into()))?;
        let mut rows = Vec::new();
        for track in providers
            .jiosaavn
            .playlist(id, limit.min(50), refresh)
            .await?
        {
            rows.push(recommendation(
                providers.persist(track).await?,
                "Chart selection",
            ));
        }
        Ok(rows)
    }

    /// Raw editorial feed only. Account feedback is applied by Core on every
    /// read, including cached reads, so this shared cache cannot leak taste.
    pub async fn cached_world_trending(
        &self,
        limit: usize,
    ) -> AppResult<Vec<DiscoveryRecommendation>> {
        if self.world_chart_cache.read().await.is_none() {
            let stored: Option<(String, i64)> = sqlx::query_as(
                "SELECT payload, fetched_at FROM discovery_feed_cache WHERE feed_key='world_trending'",
            ).fetch_optional(self.taste_engine.pool()).await?;
            if let Some((payload, fetched_at)) = stored {
                if let Ok(songs) = serde_json::from_str::<Vec<DiscoveryRecommendation>>(&payload) {
                    let age = chrono::Utc::now()
                        .timestamp()
                        .saturating_sub(fetched_at)
                        .max(0) as u64;
                    let now = std::time::Instant::now();
                    let at = now
                        .checked_sub(std::time::Duration::from_secs(age))
                        .unwrap_or(now);
                    let mut cache = self.world_chart_cache.write().await;
                    if cache.is_none() {
                        *cache = Some((at, songs));
                    }
                }
            }
        }
        Ok(self
            .world_chart_cache
            .read()
            .await
            .as_ref()
            .map(|(_, songs)| songs.iter().take(limit).cloned().collect())
            .unwrap_or_default())
    }

    /// Fetch independent playlists concurrently, retaining provider chart order.
    pub async fn get_world_trending(
        &self,
        limit: usize,
        refresh: bool,
    ) -> AppResult<Vec<DiscoveryRecommendation>> {
        let cached = self.cached_world_trending(limit).await?;
        let _fetch = self.world_fetch_lock.lock().await;
        if !refresh {
            if let Some((at, songs)) = &*self.world_chart_cache.read().await {
                if at.elapsed().as_secs() < 900 {
                    return Ok(songs.iter().take(limit).cloned().collect());
                }
            }
        }
        let charts = match self.charts(refresh).await {
            Ok(charts) => charts,
            Err(_) if !cached.is_empty() => return Ok(cached),
            Err(error) => return Err(error),
        };
        let mut requests = tokio::task::JoinSet::new();
        if let Some(providers) = &self.provider_coordinator {
            for (index, chart) in charts.iter().take(3).enumerate() {
                let Some(id) = chart["id"].as_str() else {
                    continue;
                };
                let id = format!("jiosaavn:playlist:{id}");
                let title = chart["title"]
                    .as_str()
                    .unwrap_or("Chart selection")
                    .to_owned();
                let providers = providers.clone();
                requests.spawn(async move {
                    (
                        index,
                        title,
                        Self::fetch_chart(providers, &id, 20, refresh).await,
                    )
                });
            }
        }
        let mut feeds = Vec::new();
        let mut first_error = None;
        while let Some(result) = requests.join_next().await {
            match result {
                Ok((index, title, Ok(rows))) => feeds.push((index, title, rows)),
                Ok((_, _, Err(error))) => {
                    if first_error.is_none() {
                        first_error = Some(error);
                    }
                }
                Err(error) => {
                    if first_error.is_none() {
                        first_error = Some(crate::core::AppError::Internal(error.to_string()));
                    }
                }
            }
        }
        feeds.sort_by_key(|(index, _, _)| *index);
        let mut songs = Vec::new();
        let mut seen = HashSet::new();
        for (_, title, rows) in feeds {
            for mut row in rows {
                if seen.insert(row.provider_id.clone()) {
                    row.recommendation_reason = title.clone();
                    songs.push(row);
                }
            }
        }
        if songs.is_empty() {
            if !cached.is_empty() {
                return Ok(cached);
            }
            if let Some(error) = first_error {
                return Err(error);
            }
        } else {
            let payload = serde_json::to_string(&songs)
                .map_err(|error| crate::core::AppError::Internal(error.to_string()))?;
            sqlx::query("INSERT INTO discovery_feed_cache(feed_key,payload,fetched_at) VALUES('world_trending',?,?) ON CONFLICT(feed_key) DO UPDATE SET payload=excluded.payload,fetched_at=excluded.fetched_at")
                .bind(payload).bind(chrono::Utc::now().timestamp()).execute(self.taste_engine.pool()).await?;
            *self.world_chart_cache.write().await =
                Some((std::time::Instant::now(), songs.clone()));
        }
        Ok(songs.into_iter().take(limit).collect())
    }
}
