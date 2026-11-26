//! Processor handlers for online catalogue (charts, trending, online search, playlist import).

use super::routing::misrouted;
use super::*;
mod mobile_catalog;

impl CoreProcessor {
    pub(super) async fn discovery_query(&self, query: Query) -> AppResult<QueryResponse> {
        match query {
            Query::GetLanguageCharts { language, force_refresh } =>
                self.mobile_language_charts(&language, force_refresh.unwrap_or(false)).await,
            Query::SearchMobileCatalog { query, limit } =>
                self.search_mobile_catalog(&query, limit.unwrap_or(35).min(50) as usize).await,
            Query::GetTasteArtists { languages, artist } =>
                self.mobile_taste_artists(&languages, artist.as_deref()).await,
            Query::RefreshCharts => Ok(QueryResponse::Charts(
                self.discovery_coordinator.charts(true).await?,
            )),
            Query::GetCharts => Ok(QueryResponse::Charts(
                self.discovery_coordinator.charts(false).await?,
            )),
            Query::GetCachedWorldTrending { limit } => {
                let recs = self
                    .discovery_coordinator
                    .cached_world_trending(limit as usize)
                    .await?;
                let val = recs
                    .into_iter()
                    .filter_map(|r| serde_json::to_value(r).ok())
                    .collect();
                Ok(QueryResponse::DiscoveryRecommendations(
                    self.present_recommendations(
                        serde_json::Value::Array(val),
                        &self.current_user_id().await,
                    )
                    .await?
                    .as_array()
                    .cloned()
                    .unwrap_or_default(),
                ))
            }
            Query::GetWorldTrending {
                limit,
                force_refresh,
            } => {
                let recs = self
                    .discovery_coordinator
                    .get_world_trending(limit as usize, force_refresh.unwrap_or(false))
                    .await?;
                let val = recs
                    .into_iter()
                    .filter_map(|r| serde_json::to_value(r).ok())
                    .collect();
                Ok(QueryResponse::DiscoveryRecommendations(
                    self.present_recommendations(
                        serde_json::Value::Array(val),
                        &self.current_user_id().await,
                    )
                    .await?
                    .as_array()
                    .cloned()
                    .unwrap_or_default(),
                ))
            }
            Query::GetChartSongs { chart_id, limit } => {
                let recs = self
                    .discovery_coordinator
                    .get_chart_songs(&chart_id, limit as usize)
                    .await?;
                let val = recs
                    .into_iter()
                    .filter_map(|r| serde_json::to_value(r).ok())
                    .collect();
                Ok(QueryResponse::DiscoveryRecommendations(
                    self.present_recommendations(
                        serde_json::Value::Array(val),
                        &self.current_user_id().await,
                    )
                    .await?
                    .as_array()
                    .cloned()
                    .unwrap_or_default(),
                ))
            }
            Query::ImportJioSaavnPlaylist { url_or_id } => {
                let val = self.import_jiosaavn_playlist(&url_or_id).await?;
                Ok(QueryResponse::JioSaavnPlaylistImport(val))
            }
            Query::SearchOnlineMusic {
                query,
                limit,
                provider,
            } => match provider.as_deref().unwrap_or("jiosaavn") {
                "jiosaavn" => {
                    if self
                        .settings_repo
                        .get_setting("enable_jiosaavn")
                        .await?
                        .as_deref()
                        == Some("false")
                    {
                        return Err(AppError::Validation(
                            "Enable JioSaavn in Settings to search online music".into(),
                        ));
                    }
                    let results = self
                        .discovery_coordinator
                        .search_online_music(&query, limit.unwrap_or(30).min(50) as usize)
                        .await?
                        .into_iter()
                        .filter_map(|row| serde_json::to_value(row).ok())
                        .collect();
                    return Ok(QueryResponse::DiscoveryRecommendations(
                        self.online_catalog
                            .present(serde_json::Value::Array(results))
                            .await?
                            .as_array()
                            .cloned()
                            .unwrap_or_default(),
                    ));
                }
                _ => Err(AppError::Validation(
                    "JioSaavn is the only online music source in this trial".into(),
                )),
            },
            other => Err(misrouted(&other)),
        }
    }

    pub(super) async fn import_jiosaavn_playlist(&self, reference: &str) -> AppResult<serde_json::Value> {
        if !self.provider_coordinator.enabled().await {
            return Err(AppError::Validation(
                "Enable online music in Settings to import a JioSaavn playlist".into(),
            ));
        }
        if let Some(id) = crate::providers::spotify_embed::playlist_id(reference) {
            return self.import_spotify_playlist(&id).await;
        }
        let trimmed = reference.trim();
        let is_jiosaavn = (trimmed.chars().all(|c| c.is_ascii_digit()) && !trimmed.is_empty())
            || reqwest::Url::parse(trimmed).is_ok_and(|u| {
                matches!(u.host_str(), Some("www.jiosaavn.com" | "jiosaavn.com"))
            });
        if !is_jiosaavn {
            return Err(AppError::Validation(
                "Paste a JioSaavn or Spotify playlist link".into(),
            ));
        }
        let (id, title, image, tracks) = self
            .provider_coordinator
            .jiosaavn
            .playlist_reference(reference)
            .await?;
        let mut songs = Vec::new();
        for track in tracks {
            let row = self.provider_coordinator.persist(track).await?;
            songs.push(import_row(&row));
        }
        Ok(
            serde_json::json!({"playlist_id":id,"title":title,"cover_url":image,"source":"jiosaavn",
            "total_tracks":songs.len(),"matched_tracks":0,"missing_tracks":songs.len(),"tracks":songs,
            "unmatched":[],"source_total":songs.len()}),
        )
    }

    /// Spotify supplies names only; each track is matched to a playable JioSaavn recording.
    pub(super) async fn import_spotify_playlist(&self, id: &str) -> AppResult<serde_json::Value> {
        const LIMIT: usize = 50;
        const CONCURRENCY: usize = 4;
        let playlist = self
            .provider_coordinator
            .spotify
            .playlist(id)
            .await
            .map_err(|message| AppError::ExternalApi { provider: "spotify".into(), message })?;
        let source_total = playlist.tracks.len();
        let wanted: Vec<_> = playlist.tracks.into_iter().take(LIMIT).collect();
        let semaphore = Arc::new(tokio::sync::Semaphore::new(CONCURRENCY));
        let mut tasks = tokio::task::JoinSet::new();
        for (index, track) in wanted.iter().cloned().enumerate() {
            let jiosaavn = self.provider_coordinator.jiosaavn.clone();
            let semaphore = semaphore.clone();
            tasks.spawn(async move {
                let _permit = semaphore.acquire_owned().await.ok()?;
                let title = crate::providers::strict_match::clean_title(&track.title);
                let query = match track.artists.first() {
                    Some(artist) => format!("{title} {artist}"),
                    None => title,
                };
                let candidates = jiosaavn.search(&query, 10).await.ok()?;
                let found = crate::providers::strict_match::import_match(
                    &track.title,
                    &track.artists,
                    track.duration_secs,
                    &candidates,
                )
                .cloned();
                found.map(|found| (index, found))
            });
        }
        let mut matches: Vec<Option<crate::providers::ExternalTrackMetadata>> =
            vec![None; wanted.len()];
        while let Some(joined) = tasks.join_next().await {
            if let Ok(Some((index, found))) = joined {
                matches[index] = Some(found);
            }
        }
        let mut songs = Vec::new();
        let mut unmatched = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for (track, found) in wanted.iter().zip(matches) {
            match found {
                Some(found) if seen.insert(found.provider_track_id.clone()) => {
                    let row = self.provider_coordinator.persist(found).await?;
                    songs.push(import_row(&row));
                }
                Some(_) => {}
                None => unmatched.push(serde_json::json!({
                    "title": track.title, "artist": track.artists.join(", ")
                })),
            }
        }
        Ok(serde_json::json!({"playlist_id":playlist.id,"title":playlist.title,
            "cover_url":playlist.cover_url,"source":"spotify","total_tracks":songs.len(),
            "matched_tracks":0,"missing_tracks":songs.len(),"tracks":songs,"unmatched":unmatched,
            "source_total":source_total}))
    }
}

fn import_row(row: &crate::database::models::ExternalTrackRecord) -> serde_json::Value {
    serde_json::json!({"track_id":row.id,"title":row.title,"artist":row.artist,
        "album":row.album,"duration_secs":row.duration_secs,"cover_art_url":row.cover_art_url,
        "in_library":false,"match_status":"NOT_FOUND","matched_local_track_id":null})
}
