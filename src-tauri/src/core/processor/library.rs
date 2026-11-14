//! Processor handlers for library folders, scanning, browsing and metadata.

use super::routing::misrouted;
use super::*;

impl CoreProcessor {
    pub(super) async fn library_command(&self, cmd: Command) -> AppResult<CommandResponse> {
        match cmd {
            Command::CompleteOnboarding {
                music_folders,
                start_scan,
            } => {
                let configured = self
                    .library_service
                    .complete_onboarding(music_folders, start_scan)
                    .await?;
                Ok(CommandResponse::OnboardingCompleted {
                    configured_folders: configured,
                })
            }
            Command::ResetOnboarding => {
                self.library_service.reset_onboarding().await?;
                info!("Reset onboarding status to false");
                Ok(CommandResponse::Ok)
            }
            Command::AddLibraryFolder { path } => {
                if path.trim().is_empty() {
                    return Err(AppError::Validation("Folder path cannot be empty".into()));
                }
                let folder = self.library_service.add_folder(&path).await?;
                info!(folder_id = %folder.id, path = %folder.path, "Added music library folder");
                Ok(CommandResponse::EntityId(folder.id))
            }
            Command::RemoveLibraryFolder { folder_id } => {
                self.library_service.remove_folder(&folder_id).await?;
                info!(%folder_id, "Removed music library folder");
                Ok(CommandResponse::Ok)
            }
            Command::ScanLibrary {
                folder_id,
                incremental,
            } => {
                let summaries = self
                    .library_service
                    .scan_library(folder_id, incremental)
                    .await?;
                let total_added: usize = summaries.iter().map(|s| s.added_tracks).sum();
                let total_updated: usize = summaries.iter().map(|s| s.updated_tracks).sum();
                let total_unchanged: usize = summaries.iter().map(|s| s.unchanged_tracks).sum();
                let total_tracks = total_added + total_updated + total_unchanged;
                info!(
                    total_tracks,
                    added = total_added,
                    updated = total_updated,
                    unchanged = total_unchanged,
                    "Library scan execution finished"
                );
                let user_id = self.current_user_id().await;
                let _ = self
                    .smart_mix_generator
                    .ensure_default_mixes(&user_id)
                    .await;
                Ok(CommandResponse::Ok)
            }
            Command::CancelScan => {
                self.library_service.cancel_scan();
                info!("Library scan cancel requested");
                Ok(CommandResponse::Ok)
            }
            Command::TriggerMetadataRefresh { track_id } => {
                let enriched = self.gap_enrichment.enrich(&track_id).await?;
                info!(track_id = %track_id, enriched = enriched, "Triggered metadata refresh");
                Ok(CommandResponse::Ok)
            }
            Command::FillLibraryGaps { retry } => {
                self.gap_enrichment.request_fill(retry).await?;
                Ok(CommandResponse::Ok)
            }
            Command::RemoveProviderEnrichment => {
                self.gap_enrichment.remove_all().await?;
                Ok(CommandResponse::Ok)
            }
            Command::SetMetadataPreference { key, enabled } => {
                if !matches!(
                    key.as_str(),
                    "enable_jiosaavn" | "auto_fill_metadata" | "enable_lastfm_similarity"
                ) {
                    return Err(AppError::Validation("Unknown metadata preference".into()));
                }
                self.settings_repo
                    .set_setting(&key, if enabled { "true" } else { "false" })
                    .await?;
                if enabled && key == "auto_fill_metadata" {
                    self.gap_enrichment.request_fill(false).await?;
                }
                if enabled && key == "enable_lastfm_similarity" {
                    self.similarity_service.wake();
                }
                Ok(CommandResponse::Ok)
            }
            other => Err(misrouted(&other)),
        }
    }

    pub(super) async fn library_query(&self, query: Query) -> AppResult<QueryResponse> {
        match query {
            Query::GetOnboardingStatus => {
                let completed = self.library_service.is_onboarding_completed().await?;
                let default_music_dir = LibraryService::get_default_music_dir()
                    .to_string_lossy()
                    .to_string();
                let folders = self.library_service.get_folders().await?;
                let mut folders_val = Vec::new();
                for f in folders {
                    let pattern = format!("{}/%", f.path.trim_end_matches('/'));
                    let track_count: i64 =
                        sqlx::query_scalar("SELECT COUNT(*) FROM tracks WHERE file_path LIKE ?")
                            .bind(&pattern)
                            .fetch_one(&self.db_pool)
                            .await
                            .unwrap_or(0);
                    let mut v = serde_json::to_value(&f).unwrap_or_default();
                    if let Some(obj) = v.as_object_mut() {
                        obj.insert("track_count".to_string(), serde_json::json!(track_count));
                    }
                    folders_val.push(v);
                }

                Ok(QueryResponse::OnboardingStatus {
                    completed,
                    default_music_dir,
                    configured_folders: folders_val,
                })
            }
            Query::GetLibraryOverview => {
                let folders = self.library_service.get_folders().await?;
                let val = serde_json::to_value(&folders)
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                Ok(QueryResponse::Tracks(
                    self.online_catalog
                        .present(serde_json::json!([val]))
                        .await?
                        .as_array()
                        .cloned()
                        .unwrap_or_default(),
                ))
            }
            Query::GetLibraryTracksPage { offset, limit, scope } => {
                let user = self.current_user_id().await;
                let scope = LibraryScope::parse(scope.as_deref(), self.download_service.download_dir());
                let (items, total) = self.library_service.track_repo().library_page_scoped(&user, offset, limit.clamp(1, 200), &scope).await?;
                Ok(QueryResponse::LibraryTracksPage(serde_json::json!({"items":items,"total":total})))
            }
            Query::GetLibraryAlbumsPage { offset, limit, scope } => {
                let scope = LibraryScope::parse(scope.as_deref(), self.download_service.download_dir());
                let (items, total) = self.library_service.album_repo().library_page_scoped(offset, limit.clamp(1, 100), &scope).await?;
                Ok(QueryResponse::LibraryAlbumsPage(serde_json::json!({"items":items,"total":total})))
            }
            Query::GetTracks {
                offset,
                limit,
                sort_by,
                ascending,
            } => {
                let user_id = self.current_user_id().await;
                let tracks = self
                    .library_service
                    .track_repo()
                    .list_tracks_scoped(
                        Some(&user_id),
                        offset,
                        limit,
                        sort_by.as_deref(),
                        ascending,
                    )
                    .await?;
                let val: Vec<serde_json::Value> = tracks
                    .into_iter()
                    .filter_map(|t| serde_json::to_value(t).ok())
                    .collect();
                Ok(QueryResponse::Tracks(
                    self.online_catalog
                        .present(serde_json::Value::Array(val))
                        .await?
                        .as_array()
                        .cloned()
                        .unwrap_or_default(),
                ))
            }
            Query::GetTrackById { track_id } => {
                let user_id = self.current_user_id().await;
                let track = self
                    .library_service
                    .track_repo()
                    .find_by_id_scoped(&track_id, Some(&user_id))
                    .await?;
                let val = track.and_then(|t| serde_json::to_value(t).ok());
                let val = match val {
                    Some(value) => {
                        let result = self
                            .online_catalog
                            .present(serde_json::json!([value]))
                            .await?;
                        result.as_array().and_then(|items| items.first()).cloned()
                    }
                    None => None,
                };
                Ok(QueryResponse::Track(val))
            }
            Query::GetArtists { offset, limit } => {
                let artists = self
                    .library_service
                    .artist_repo()
                    .list_artists(offset, limit)
                    .await?;
                let val: Vec<serde_json::Value> = artists
                    .into_iter()
                    .filter_map(|a| serde_json::to_value(a).ok())
                    .collect();
                Ok(QueryResponse::Artists(val))
            }
            Query::GetAlbums { offset, limit } => {
                let albums = self
                    .library_service
                    .album_repo()
                    .list_albums(offset, limit)
                    .await?;
                let val: Vec<serde_json::Value> = albums
                    .into_iter()
                    .filter_map(|al| serde_json::to_value(al).ok())
                    .collect();
                Ok(QueryResponse::Albums(val))
            }
            Query::GetAlbumDetails { album_id } => {
                let repo = self.library_service.album_repo();
                let album = repo.find_by_id(&album_id).await?.ok_or_else(|| {
                    AppError::NotFound("Album is no longer in the library".into())
                })?;
                let user_id = self.current_user_id().await;
                let tracks = repo.tracks_scoped(&album_id, &user_id).await?;
                let rows: Vec<_> = tracks
                    .into_iter()
                    .filter_map(|track| serde_json::to_value(track).ok())
                    .collect();
                let tracks = self
                    .online_catalog
                    .present(serde_json::Value::Array(rows))
                    .await?;
                Ok(QueryResponse::AlbumDetails(
                    serde_json::json!({"album":album,"tracks":tracks}),
                ))
            }
            Query::SearchLibrary { query_text, limit } => {
                let user_id = self.current_user_id().await;
                let results = self
                    .library_service
                    .search_scoped(Some(&user_id), &query_text, limit)
                    .await?;
                let val: Vec<serde_json::Value> = results
                    .into_iter()
                    .filter_map(|t| serde_json::to_value(t).ok())
                    .collect();
                Ok(QueryResponse::SearchResults(val))
            }
            other => Err(misrouted(&other)),
        }
    }
}
