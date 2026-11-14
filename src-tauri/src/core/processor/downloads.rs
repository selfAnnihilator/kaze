//! Processor handlers for downloads and the wishlist.

use super::routing::misrouted;
use super::*;

impl CoreProcessor {
    pub(super) async fn downloads_command(&self, cmd: Command) -> AppResult<CommandResponse> {
        match cmd {
            Command::AddToWishlist {
                title,
                artist,
                album,
                external_id,
            } => {
                let item = self
                    .wishlist_manager
                    .add_to_wishlist(title, artist, album, external_id, None)
                    .await?;
                info!(wishlist_id = %item.id, title = %item.title, "Added track to wishlist");
                Ok(CommandResponse::EntityId(item.id))
            }
            Command::UpdateWishlistStatus {
                wishlist_id,
                status,
            } => {
                self.wishlist_manager
                    .update_status_enum(&wishlist_id, status)
                    .await?;
                info!(wishlist_id = %wishlist_id, "Updated wishlist item status");
                Ok(CommandResponse::Ok)
            }
            Command::AddMissingToWishlist { tracks } => {
                let count = self.add_missing_to_wishlist(tracks).await?;
                Ok(CommandResponse::WishlistAdded { count })
            }
            Command::SearchDownloads {
                artist,
                title,
                album: _,
            } => {
                if !self.gap_enrichment.enabled("enable_jiosaavn", true).await {
                    return Err(AppError::Validation(
                        "JioSaavn is disabled in Settings".into(),
                    ));
                }
                // Persist source identity for every downloadable search row.
                let _ = self
                    .discovery_coordinator
                    .search_online_music(&format!("{} {}", artist, title), 25)
                    .await?;
                let results = self.download_service.search_track(&artist, &title).await?;
                let val: Vec<serde_json::Value> = results
                    .into_iter()
                    .filter_map(|r| serde_json::to_value(r).ok())
                    .collect();
                Ok(CommandResponse::SearchResults(val))
            }
            Command::DownloadTrack { track_id } => {
                if !self.gap_enrichment.enabled("enable_jiosaavn", true).await {
                    return Err(AppError::Validation(
                        "Online music is disabled in Settings".into(),
                    ));
                }
                // A search row may use a saved alias for this exact provider ID.
                let bound_id = if let Some(source) = track_id.strip_prefix("jiosaavn:") {
                    sqlx::query_scalar::<_,String>("SELECT id FROM external_tracks WHERE provider='jiosaavn' AND provider_id=?").bind(source).fetch_optional(&self.db_pool).await?.unwrap_or(track_id.clone())
                } else {
                    track_id.clone()
                };
                let track = self.online_catalog.bind(&bound_id).await?.ok_or_else(|| {
                    AppError::Validation("This song has no saved online recording".into())
                })?;
                let task = self.download_service.download_track(&track).await?;
                Ok(CommandResponse::DownloadStarted { task_id: task.id })
            }
            Command::RemoveDownload { track_id, source_id } => {
                let released = self
                    .download_service
                    .remove_download(&track_id, source_id.as_deref())
                    .await?;
                if let Some((local, online)) = released {
                    // Queued or playing copies continue as the online song.
                    self.playback_service.replace_queued_track(&local, &online).await;
                }
                Ok(CommandResponse::Ok)
            }
            Command::StartDownload {
                search_result_id,
                wishlist_id,
            } => {
                if !self.gap_enrichment.enabled("enable_jiosaavn", true).await {
                    return Err(AppError::Validation(
                        "JioSaavn is disabled in Settings".into(),
                    ));
                }
                let task = self
                    .download_service
                    .start_download(&search_result_id, wishlist_id)
                    .await?;
                Ok(CommandResponse::DownloadStarted { task_id: task.id })
            }
            Command::CancelDownload { task_id } => {
                self.download_service.cancel_download(&task_id).await?;
                Ok(CommandResponse::Ok)
            }
            Command::PollDownloadProgress { task_id } => {
                let _ = self.download_service.poll_task(&task_id).await?;
                Ok(CommandResponse::Ok)
            }
            other => Err(misrouted(&other)),
        }
    }

    pub(super) async fn downloads_query(&self, query: Query) -> AppResult<QueryResponse> {
        match query {
            Query::GetWishlist => {
                let items = self.wishlist_manager.get_wishlist(None).await?;
                let val: Vec<serde_json::Value> = items
                    .into_iter()
                    .filter_map(|w| serde_json::to_value(w).ok())
                    .collect();
                Ok(QueryResponse::Wishlist(val))
            }
            Query::GetDownloads {
                status_filter,
                limit,
            } => {
                let tasks = self
                    .download_service
                    .list_downloads(status_filter.as_deref(), limit.min(10000))
                    .await?;
                let val = crate::downloads::rows::download_rows(&self.db_pool, tasks).await?;
                Ok(QueryResponse::Downloads(val))
            }
            other => Err(misrouted(&other)),
        }
    }

    pub(super) async fn add_missing_to_wishlist(&self, tracks: Vec<serde_json::Value>) -> AppResult<usize> {
        let mut count = 0;
        for t in tracks {
            let title = t.get("title").and_then(|v| v.as_str()).unwrap_or("").trim();
            let artist = t
                .get("artist")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            let album = t
                .get("album")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let external_id = t
                .get("track_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            if !title.is_empty() && !artist.is_empty() {
                let _ = self
                    .wishlist_manager
                    .add_to_wishlist(
                        title.to_string(),
                        artist.to_string(),
                        album,
                        external_id,
                        None,
                    )
                    .await?;
                count += 1;
            }
        }
        info!(added = count, "Added missing tracks to wishlist");
        Ok(count)
    }
}
