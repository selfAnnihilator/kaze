use crate::config::AppConfig;
use crate::core::command::{Command, CommandResponse};
use crate::core::error::{AppError, AppResult};
use crate::core::event::Event;
use crate::core::event_bus::EventBus;
use crate::core::query::{Query, QueryResponse};
use crate::database::models::PlaylistRecord;
use crate::database::repositories::{
    PlaylistRepository, SqliteDownloadRepository, SqliteHistoryRepository,
    SqlitePlaylistRepository, SqliteStatsRepository, SqliteWishlistRepository,
};
use crate::discovery::{DiscoveryCoordinator, WishlistManager};
use crate::downloads::{DownloadProvider, DownloadService, SoulseekProvider};
use crate::history::HistoryService;
use crate::library::LibraryService;
use crate::playback::backend::{AudioBackend, RodioAudioBackend};
use crate::playback::PlaybackService;
use crate::providers::ProviderCoordinator;
use crate::ranking::RankingEngine;
use crate::recommendations::{LocalRecommender, SmartMixGenerator, TasteProfileEngine};
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Central processing coordinator for commands, queries, and event dispatching.
pub struct CoreProcessor {
    event_bus: Arc<EventBus>,
    db_pool: SqlitePool,
    library_service: Arc<LibraryService>,
    playback_service: Arc<PlaybackService>,
    history_service: Arc<HistoryService>,
    ranking_engine: Arc<RankingEngine>,
    playlist_repo: Arc<SqlitePlaylistRepository>,
    taste_engine: Arc<TasteProfileEngine>,
    recommender: Arc<LocalRecommender>,
    smart_mix_generator: Arc<SmartMixGenerator>,
    provider_coordinator: Arc<ProviderCoordinator>,
    wishlist_manager: Arc<WishlistManager>,
    discovery_coordinator: Arc<DiscoveryCoordinator>,
    download_service: Arc<DownloadService>,
    config: Arc<RwLock<AppConfig>>,
}

impl CoreProcessor {
    /// Constructs a new CoreProcessor instance with standard Rodio audio backend.
    pub fn new(db_pool: SqlitePool, config: AppConfig) -> Self {
        let backend = Box::new(RodioAudioBackend::try_new().unwrap_or_else(|_| {
            panic!("Fatal error creating fallback audio backend");
        }));
        Self::new_with_backend(db_pool, config, backend)
    }

    /// Constructs a CoreProcessor instance with an injected AudioBackend (e.g. for testing).
    pub fn new_with_backend(
        db_pool: SqlitePool,
        config: AppConfig,
        backend: Box<dyn AudioBackend>,
    ) -> Self {
        Self::new_with_services(db_pool, config, backend, None)
    }

    /// Constructs a CoreProcessor instance with optional custom download provider (e.g. Mock for testing).
    pub fn new_with_services(
        db_pool: SqlitePool,
        config: AppConfig,
        backend: Box<dyn AudioBackend>,
        custom_download_provider: Option<Arc<dyn DownloadProvider>>,
    ) -> Self {
        let event_bus = Arc::new(EventBus::default());
        let library_service = Arc::new(LibraryService::new(db_pool.clone(), event_bus.clone()));
        let playback_service = PlaybackService::new(
            backend,
            library_service.track_repo(),
            event_bus.clone(),
        );

        let history_repo = Arc::new(SqliteHistoryRepository::new(db_pool.clone()));
        let stats_repo = Arc::new(SqliteStatsRepository::new(db_pool.clone()));

        let history_service = HistoryService::new(
            history_repo.clone(),
            stats_repo.clone(),
            config.history.clone(),
            event_bus.clone(),
        );

        let ranking_engine = Arc::new(RankingEngine::new(
            stats_repo.clone(),
            config.ranking.clone(),
        ));

        let playlist_repo = Arc::new(SqlitePlaylistRepository::new(db_pool.clone()));
        let taste_engine = Arc::new(TasteProfileEngine::new(db_pool.clone()));
        let recommender = Arc::new(LocalRecommender::new(db_pool.clone()));
        let smart_mix_generator = Arc::new(SmartMixGenerator::new(db_pool.clone()));
        let provider_coordinator = Arc::new(ProviderCoordinator::new(
            db_pool.clone(),
            event_bus.clone(),
            config.cache_dir.clone(),
            None,
            None,
        ));
        let wishlist_repo = Arc::new(SqliteWishlistRepository::new(db_pool.clone()));
        let wishlist_manager = Arc::new(WishlistManager::new(wishlist_repo.clone()));
        let discovery_coordinator = Arc::new(DiscoveryCoordinator::new(
            wishlist_repo.clone(),
            library_service.track_repo(),
            taste_engine.clone(),
            Some(provider_coordinator.clone()),
        ));

        let download_repo = Arc::new(SqliteDownloadRepository::new(db_pool.clone()));
        let download_provider = custom_download_provider.unwrap_or_else(|| {
            Arc::new(SoulseekProvider::new(
                &config.downloads.slskd_host,
                config.downloads.slskd_port,
                config.downloads.slskd_api_key.clone(),
            ))
        });
        let download_dir = config.downloads.download_dir.clone().unwrap_or_else(|| {
            config.cache_dir.join("downloads")
        });
        let download_service = Arc::new(DownloadService::new(
            download_repo,
            download_provider,
            event_bus.clone(),
            library_service.clone(),
            wishlist_manager.clone(),
            download_dir,
            config.downloads.auto_import,
        ));

        Self {
            event_bus,
            db_pool,
            library_service,
            playback_service,
            history_service,
            ranking_engine,
            playlist_repo,
            taste_engine,
            recommender,
            smart_mix_generator,
            provider_coordinator,
            wishlist_manager,
            discovery_coordinator,
            download_service,
            config: Arc::new(RwLock::new(config)),
        }
    }

    /// Access the internal EventBus handle.
    pub fn event_bus(&self) -> Arc<EventBus> {
        self.event_bus.clone()
    }

    /// Access the database pool.
    pub fn db_pool(&self) -> &SqlitePool {
        &self.db_pool
    }

    /// Access the LibraryService handle.
    pub fn library_service(&self) -> Arc<LibraryService> {
        self.library_service.clone()
    }

    /// Access the PlaybackService handle.
    pub fn playback_service(&self) -> Arc<PlaybackService> {
        self.playback_service.clone()
    }

    /// Access the HistoryService handle.
    pub fn history_service(&self) -> Arc<HistoryService> {
        self.history_service.clone()
    }

    /// Access the RankingEngine handle.
    pub fn ranking_engine(&self) -> Arc<RankingEngine> {
        self.ranking_engine.clone()
    }

    /// Access the PlaylistRepository handle.
    pub fn playlist_repo(&self) -> Arc<SqlitePlaylistRepository> {
        self.playlist_repo.clone()
    }

    /// Access the TasteProfileEngine handle.
    pub fn taste_engine(&self) -> Arc<TasteProfileEngine> {
        self.taste_engine.clone()
    }

    /// Access the LocalRecommender handle.
    pub fn recommender(&self) -> Arc<LocalRecommender> {
        self.recommender.clone()
    }

    /// Access the SmartMixGenerator handle.
    pub fn smart_mix_generator(&self) -> Arc<SmartMixGenerator> {
        self.smart_mix_generator.clone()
    }

    /// Access the ProviderCoordinator handle.
    pub fn provider_coordinator(&self) -> Arc<ProviderCoordinator> {
        self.provider_coordinator.clone()
    }

    /// Access the WishlistManager handle.
    pub fn wishlist_manager(&self) -> Arc<WishlistManager> {
        self.wishlist_manager.clone()
    }

    /// Access the DiscoveryCoordinator handle.
    pub fn discovery_coordinator(&self) -> Arc<DiscoveryCoordinator> {
        self.discovery_coordinator.clone()
    }

    /// Access the DownloadService handle.
    pub fn download_service(&self) -> Arc<DownloadService> {
        self.download_service.clone()
    }

    /// Dispatches and executes an incoming Command, emitting events and returning the result.
    pub async fn dispatch_command(&self, cmd: Command) -> AppResult<CommandResponse> {
        info!(?cmd, "Dispatching command");

        match cmd {
            // --- Library & Onboarding Management ---
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
                let _ = self.smart_mix_generator.ensure_default_mixes().await;
                Ok(CommandResponse::Ok)
            }
            Command::CancelScan => {
                self.library_service.cancel_scan();
                info!("Library scan cancel requested");
                Ok(CommandResponse::Ok)
            }

            // --- Playback Controls ---
            Command::PlayTrack { track_id, source } => {
                self.playback_service.play_track(&track_id, source).await?;
                Ok(CommandResponse::Ok)
            }
            Command::PlayQueueIndex { index } => {
                self.playback_service.play_queue_index(index).await?;
                Ok(CommandResponse::Ok)
            }
            Command::Pause => {
                self.playback_service.pause().await?;
                Ok(CommandResponse::Ok)
            }
            Command::Resume => {
                self.playback_service.resume().await?;
                Ok(CommandResponse::Ok)
            }
            Command::Stop => {
                self.playback_service.stop().await?;
                Ok(CommandResponse::Ok)
            }
            Command::NextTrack => {
                self.playback_service.next().await?;
                Ok(CommandResponse::Ok)
            }
            Command::PreviousTrack => {
                self.playback_service.previous().await?;
                Ok(CommandResponse::Ok)
            }
            Command::Seek { position_secs } => {
                self.playback_service.seek(position_secs).await?;
                Ok(CommandResponse::Ok)
            }
            Command::SetVolume { volume } => {
                if !(0.0..=1.0).contains(&volume) {
                    return Err(AppError::Validation("Volume must be between 0.0 and 1.0".into()));
                }
                self.playback_service.set_volume(volume).await?;
                Ok(CommandResponse::Ok)
            }
            Command::ToggleMute => {
                let state = self.playback_service.get_playback_state().await;
                let new_vol = if state.volume > 0.0 { 0.0 } else { 0.8 };
                self.playback_service.set_volume(new_vol).await?;
                Ok(CommandResponse::Ok)
            }
            Command::SetRepeatMode { mode } => {
                self.playback_service.set_repeat_mode(mode).await;
                Ok(CommandResponse::Ok)
            }
            Command::SetShuffle { enabled } => {
                self.playback_service.set_shuffle(enabled).await;
                Ok(CommandResponse::Ok)
            }
            Command::EnqueueTrack { track_id, play_next } => {
                self.playback_service.enqueue_track(&track_id, play_next).await?;
                Ok(CommandResponse::Ok)
            }
            Command::ClearQueue => {
                self.playback_service.clear_queue().await;
                Ok(CommandResponse::Ok)
            }

            // --- User Feedback & Taste ---
            Command::LikeTrack { track_id } => {
                self.history_service.set_track_like(&track_id, 1).await?;
                Ok(CommandResponse::Ok)
            }
            Command::DislikeTrack { track_id } => {
                self.history_service.set_track_like(&track_id, -1).await?;
                Ok(CommandResponse::Ok)
            }
            Command::RemoveTrackFeedback { track_id } => {
                self.history_service.set_track_like(&track_id, 0).await?;
                Ok(CommandResponse::Ok)
            }

            // --- Playlists & Recommendations ---
            Command::GenerateSmartMix { mix_type } => {
                let playlist = self.smart_mix_generator.generate_mix(&mix_type).await?;
                let tracks = self.playlist_repo.get_playlist_tracks(&playlist.id).await?;
                let track_count = tracks.len();
                let mix_type_name = playlist.mix_type.clone().unwrap_or_else(|| "custom".to_string());

                let _ = self.event_bus.publish(Event::SmartMixGenerated {
                    playlist_id: playlist.id.clone(),
                    mix_type: mix_type_name,
                    track_count,
                });

                Ok(CommandResponse::MixGenerated {
                    playlist_id: playlist.id,
                    track_count,
                })
            }
            Command::CreatePlaylist { name, description } => {
                let now = chrono::Utc::now().timestamp();
                let id = format!("pl_{}", uuid::Uuid::new_v4());
                let record = PlaylistRecord {
                    id: id.clone(),
                    name,
                    description,
                    is_smart_mix: 0,
                    mix_type: None,
                    generation_reason: None,
                    expires_at: None,
                    created_at: now,
                    updated_at: now,
                };
                self.playlist_repo.create_playlist(&record).await?;
                Ok(CommandResponse::EntityId(id))
            }
            Command::DeletePlaylist { playlist_id } => {
                self.playlist_repo.delete_playlist(&playlist_id).await?;
                Ok(CommandResponse::Ok)
            }
            Command::AddTrackToPlaylist { playlist_id, track_id } => {
                self.playlist_repo.add_track(&playlist_id, &track_id, None).await?;
                Ok(CommandResponse::Ok)
            }
            Command::RemoveTrackFromPlaylist { playlist_id, track_id } => {
                self.playlist_repo.remove_track(&playlist_id, &track_id).await?;
                Ok(CommandResponse::Ok)
            }

            // --- Metadata Providers ---
            Command::TriggerMetadataRefresh { track_id } => {
                let enriched = self.provider_coordinator.enrich_track(&track_id).await?;
                info!(track_id = %track_id, enriched = enriched, "Triggered metadata refresh");
                Ok(CommandResponse::Ok)
            }

            // --- Discovery & Wishlist ---
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
            Command::UpdateWishlistStatus { wishlist_id, status } => {
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

            // --- Soulseek & Downloads ---
            Command::SearchSoulseek { artist, title, album: _ } => {
                let query = format!("{} {}", artist, title);
                let results = match self.download_service.search(&query).await {
                    Ok(res) => res,
                    Err(e) => {
                        tracing::warn!(error = %e, "Slskd daemon not responding, returning empty search results");
                        Vec::new()
                    }
                };
                let val: Vec<serde_json::Value> = results
                    .into_iter()
                    .filter_map(|r| serde_json::to_value(r).ok())
                    .collect();
                Ok(CommandResponse::SearchResults(val))
            }
            Command::LaunchSoulseek { search_query } => {
                let msg = self.launch_soulseek_qt(search_query).await?;
                Ok(CommandResponse::SoulseekLaunched { message: msg })
            }
            Command::ImportSoulseekDownloads => {
                let count = self.import_soulseek_downloads().await?;
                Ok(CommandResponse::SoulseekImported { imported_count: count })
            }
            Command::StartDownload {
                search_result_id,
                wishlist_id,
            } => {
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
        }
    }

    /// Executes a read-only Query and returns the typed QueryResponse.
    pub async fn execute_query(&self, query: Query) -> AppResult<QueryResponse> {
        info!(?query, "Executing query");

        match query {
            Query::GetPlaybackState => {
                let state = self.playback_service.get_playback_state().await;
                let val = serde_json::to_value(&state)
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                Ok(QueryResponse::PlaybackState(val))
            }
            Query::GetTopRankings {
                window,
                entity,
                limit,
            } => {
                let ranked = self.ranking_engine.get_rankings(window, entity, limit).await?;
                let val = match ranked {
                    crate::ranking::RankedOutput::Tracks(items) => {
                        items.into_iter().filter_map(|i| serde_json::to_value(i).ok()).collect()
                    }
                    crate::ranking::RankedOutput::Artists(items) => {
                        items.into_iter().filter_map(|i| serde_json::to_value(i).ok()).collect()
                    }
                    crate::ranking::RankedOutput::Albums(items) => {
                        items.into_iter().filter_map(|i| serde_json::to_value(i).ok()).collect()
                    }
                    crate::ranking::RankedOutput::Genres(items) => {
                        items.into_iter().filter_map(|i| serde_json::to_value(i).ok()).collect()
                    }
                };
                Ok(QueryResponse::Rankings(val))
            }
            Query::GetOnboardingStatus => {
                let completed = self.library_service.is_onboarding_completed().await?;
                let default_music_dir = LibraryService::get_default_music_dir()
                    .to_string_lossy()
                    .to_string();
                let folders = self.library_service.get_folders().await?;
                let mut folders_val = Vec::new();
                for f in folders {
                    let track_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tracks WHERE folder_id = ?")
                        .bind(&f.id)
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
            Query::GetSettings => {
                let cfg = self.config.read().await;
                let val = serde_json::to_value(&*cfg)
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                Ok(QueryResponse::Settings(val))
            }
            Query::GetLibraryOverview => {
                let folders = self.library_service.get_folders().await?;
                let val = serde_json::to_value(&folders)
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                Ok(QueryResponse::Tracks(vec![val]))
            }
            Query::GetTracks {
                offset,
                limit,
                sort_by,
                ascending,
            } => {
                let tracks = self
                    .library_service
                    .track_repo()
                    .list_tracks(offset, limit, sort_by.as_deref(), ascending)
                    .await?;
                let val: Vec<serde_json::Value> = tracks
                    .into_iter()
                    .filter_map(|t| serde_json::to_value(t).ok())
                    .collect();
                Ok(QueryResponse::Tracks(val))
            }
            Query::GetTrackById { track_id } => {
                let track = self
                    .library_service
                    .track_repo()
                    .find_by_id(&track_id)
                    .await?;
                let val = track.and_then(|t| serde_json::to_value(t).ok());
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
            Query::SearchLibrary { query_text, limit } => {
                let results = self.library_service.search(&query_text, limit).await?;
                let val: Vec<serde_json::Value> = results
                    .into_iter()
                    .filter_map(|t| serde_json::to_value(t).ok())
                    .collect();
                Ok(QueryResponse::SearchResults(val))
            }
            Query::GetTasteProfile => {
                let profile = self.taste_engine.compute_taste_profile().await?;
                let val = serde_json::to_value(&profile)
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                Ok(QueryResponse::TasteProfile(val))
            }
            Query::GetLocalRecommendations { limit } => {
                let recs = self.recommender.recommend(limit as usize).await?;
                let val: Vec<serde_json::Value> = recs
                    .into_iter()
                    .filter_map(|r| serde_json::to_value(r).ok())
                    .collect();
                Ok(QueryResponse::Recommendations(val))
            }
            Query::GetSmartMixes => {
                let mixes = self.smart_mix_generator.ensure_default_mixes().await?;
                let val: Vec<serde_json::Value> = mixes
                    .into_iter()
                    .filter_map(|m| serde_json::to_value(m).ok())
                    .collect();
                Ok(QueryResponse::SmartMixes(val))
            }
            Query::GetPlaylists => {
                let _ = self.smart_mix_generator.ensure_default_mixes().await;
                let playlists = self.playlist_repo.get_all_playlists().await?;
                let val: Vec<serde_json::Value> = playlists
                    .into_iter()
                    .filter_map(|p| serde_json::to_value(p).ok())
                    .collect();
                Ok(QueryResponse::Playlists(val))
            }
            Query::GetPlaylistTracks { playlist_id } => {
                let tracks = self.playlist_repo.get_playlist_tracks(&playlist_id).await?;
                let val: Vec<serde_json::Value> = tracks
                    .into_iter()
                    .filter_map(|t| serde_json::to_value(t).ok())
                    .collect();
                Ok(QueryResponse::PlaylistTracks(val))
            }
            Query::GetWishlist => {
                let items = self.wishlist_manager.get_wishlist(None).await?;
                let val: Vec<serde_json::Value> = items
                    .into_iter()
                    .filter_map(|w| serde_json::to_value(w).ok())
                    .collect();
                Ok(QueryResponse::Wishlist(val))
            }
            Query::GetDiscoveryRecommendations { limit } => {
                let recs = self
                    .discovery_coordinator
                    .get_discovery_recommendations(limit as usize)
                    .await?;
                let val: Vec<serde_json::Value> = recs
                    .into_iter()
                    .filter_map(|r| serde_json::to_value(r).ok())
                    .collect();
                Ok(QueryResponse::DiscoveryRecommendations(val))
            }
            Query::GetDownloads { status_filter, limit } => {
                let tasks = self
                    .download_service
                    .list_downloads(status_filter.as_deref(), limit)
                    .await?;
                let val: Vec<serde_json::Value> = tasks
                    .into_iter()
                    .filter_map(|t| serde_json::to_value(t).ok())
                    .collect();
                Ok(QueryResponse::Downloads(val))
            }
            Query::ImportSpotifyPlaylist { url_or_id } => {
                let val = self.import_spotify_playlist(&url_or_id).await?;
                Ok(QueryResponse::SpotifyPlaylistImport(val))
            }
            _ => {
                warn!(?query, "Query handler routed to stub during Phase 8");
                Ok(QueryResponse::Empty)
            }
        }
    }

    async fn launch_soulseek_qt(&self, search_query: Option<String>) -> AppResult<String> {
        // 1. If query is provided, copy to clipboard if wl-copy or xclip is available
        if let Some(ref q) = search_query {
            let _ = std::process::Command::new("wl-copy")
                .arg(q)
                .spawn();

            if let Ok(mut child) = std::process::Command::new("xclip")
                .args(["-selection", "clipboard"])
                .stdin(std::process::Stdio::piped())
                .spawn()
            {
                if let Some(mut stdin) = child.stdin.take() {
                    use std::io::Write;
                    let _ = stdin.write_all(q.as_bytes());
                }
            }
        }

        // 2. Look for SoulseekQt AppImage
        let possible_paths = [
            "/home/abhi/Applications/SoulseekQt-2024-6-30.AppImage",
            "/home/abhi/Applications/SoulseekQt.AppImage",
        ];

        let mut launched = false;
        let mut launched_path = String::new();

        for p in possible_paths {
            if std::path::Path::new(p).exists() {
                if let Ok(_) = std::process::Command::new(p)
                    .env("QT_QPA_PLATFORM", "xcb")
                    .spawn()
                {
                    launched = true;
                    launched_path = p.to_string();
                    break;
                }
            }
        }

        if !launched {
            let _ = std::process::Command::new("gtk-launch")
                .arg("soulseekqt")
                .spawn();
            launched_path = "gtk-launch soulseekqt".to_string();
        }

        let msg = if let Some(q) = search_query {
            format!("Launched SoulseekQt! Query \"{}\" copied to clipboard.", q)
        } else {
            format!("Launched SoulseekQt ({})", launched_path)
        };

        info!("{}", msg);
        Ok(msg)
    }

    async fn import_soulseek_downloads(&self) -> AppResult<usize> {
        let soulseek_complete = "/home/abhi/Soulseek Downloads/complete";
        let path = std::path::PathBuf::from(soulseek_complete);
        if !path.exists() {
            return Ok(0);
        }

        // 1. Add folder if not already present
        let _ = self.library_service.add_folder(soulseek_complete).await;

        // 2. Scan folder
        let summaries = self.library_service.scan_library(None, true).await?;
        let total_added: usize = summaries.iter().map(|s| s.added_tracks).sum();

        // 3. Check wishlist items and update status
        let wishlist_items = self.wishlist_manager.get_wishlist(None).await?;
        for item in wishlist_items {
            if item.status == "want" {
                let match_res = self
                    .discovery_coordinator
                    .match_against_library(&item.title, &item.artist, None)
                    .await?;
                if match_res.status == crate::discovery::MatchStatus::ExactMatch
                    || match_res.status == crate::discovery::MatchStatus::LikelyMatch
                {
                    let _ = self
                        .wishlist_manager
                        .update_status(&item.id, "downloaded")
                        .await;
                }
            }
        }

        info!(total_added, "Imported tracks from Soulseek Downloads");
        Ok(total_added)
    }

    async fn add_missing_to_wishlist(&self, tracks: Vec<serde_json::Value>) -> AppResult<usize> {
        let mut count = 0;
        for t in tracks {
            let title = t.get("title").and_then(|v| v.as_str()).unwrap_or("").trim();
            let artist = t.get("artist").and_then(|v| v.as_str()).unwrap_or("").trim();
            let album = t.get("album").and_then(|v| v.as_str()).map(|s| s.to_string());
            let external_id = t.get("spotify_id").and_then(|v| v.as_str()).map(|s| s.to_string());

            if !title.is_empty() && !artist.is_empty() {
                let _ = self
                    .wishlist_manager
                    .add_to_wishlist(title.to_string(), artist.to_string(), album, external_id, None)
                    .await?;
                count += 1;
            }
        }
        info!(added = count, "Added missing tracks to wishlist");
        Ok(count)
    }

    async fn import_spotify_playlist(&self, url_or_id: &str) -> AppResult<serde_json::Value> {
        // Clean URL or bare ID
        let clean_id = if let Some(idx) = url_or_id.find("/playlist/") {
            let rest = &url_or_id[idx + 10..];
            rest.split('?').next().unwrap_or(rest).trim()
        } else {
            url_or_id.split('?').next().unwrap_or(url_or_id).trim()
        };

        let embed_url = format!("https://open.spotify.com/embed/playlist/{}", clean_id);
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(12))
            .build()
            .unwrap_or_default();

        let resp = client
            .get(&embed_url)
            .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64)")
            .send()
            .await
            .map_err(|e| AppError::Network(format!("Failed to reach Spotify embed: {}", e)))?;

        let html = resp
            .text()
            .await
            .map_err(|e| AppError::Network(format!("Failed to read Spotify response: {}", e)))?;

        // Extract JSON inside <script id="__NEXT_DATA__" type="application/json">...</script>
        let script_tag = "<script id=\"__NEXT_DATA__\" type=\"application/json\">";
        let script_start = html.find(script_tag)
            .ok_or_else(|| AppError::ExternalApi {
                provider: "spotify".into(),
                message: "Unable to parse Spotify playlist payload".into(),
            })?;
        let json_start = script_start + script_tag.len();
        let json_end = html[json_start..].find("</script>")
            .map(|idx| json_start + idx)
            .ok_or_else(|| AppError::ExternalApi {
                provider: "spotify".into(),
                message: "Invalid Spotify response format".into(),
            })?;

        let raw_json = &html[json_start..json_end];
        let root: serde_json::Value = serde_json::from_str(raw_json)
            .map_err(|e| AppError::ExternalApi {
                provider: "spotify".into(),
                message: format!("Failed to parse JSON: {}", e),
            })?;

        let entity = &root["props"]["pageProps"]["state"]["data"]["entity"];
        let playlist_title = entity["title"]
            .as_str()
            .or_else(|| entity["name"].as_str())
            .unwrap_or("Imported Spotify Playlist")
            .to_string();

        let cover_url = entity["coverArt"]["sources"][0]["url"]
            .as_str()
            .map(|s| s.to_string());

        let raw_tracks = entity["trackList"].as_array().cloned().unwrap_or_default();

        let mut parsed_tracks = Vec::new();
        let mut matched_count = 0;
        let mut missing_count = 0;

        for item in &raw_tracks {
            let title = item["title"].as_str().unwrap_or("").trim().to_string();
            let artist = item["subtitle"].as_str().unwrap_or("Unknown Artist").trim().to_string();
            let duration_ms = item["duration"].as_f64().unwrap_or(0.0);
            let duration_secs = if duration_ms > 0.0 { Some(duration_ms / 1000.0) } else { None };
            let spotify_track_id = item["id"].as_str().unwrap_or("").to_string();

            if title.is_empty() {
                continue;
            }

            let match_res = self
                .discovery_coordinator
                .match_against_library(&title, &artist, duration_secs)
                .await?;

            let in_library = match_res.status == crate::discovery::MatchStatus::ExactMatch
                || match_res.status == crate::discovery::MatchStatus::LikelyMatch;

            if in_library {
                matched_count += 1;
            } else {
                missing_count += 1;
            }

            parsed_tracks.push(serde_json::json!({
                "spotify_id": spotify_track_id,
                "title": title,
                "artist": artist,
                "duration_secs": duration_secs,
                "in_library": in_library,
                "match_status": match_res.status.as_str(),
                "matched_local_track_id": match_res.matched_track_id,
            }));
        }

        Ok(serde_json::json!({
            "playlist_id": clean_id,
            "title": playlist_title,
            "cover_url": cover_url,
            "total_tracks": parsed_tracks.len(),
            "matched_tracks": matched_count,
            "missing_tracks": missing_count,
            "tracks": parsed_tracks,
        }))
    }
}
