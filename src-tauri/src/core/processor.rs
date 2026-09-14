use crate::config::AppConfig;
use crate::core::command::{Command, CommandResponse};
use crate::core::error::{AppError, AppResult};
use crate::core::event::Event;
use crate::core::event_bus::EventBus;
use crate::core::query::{Query, QueryResponse};
use crate::database::models::PlaylistRecord;
use crate::cloud::{
    credentials, device, AuthSessionState, CloudClient, CloudSessionMetadata, CloudSyncStatus,
    CloudUser, SessionExpiredReason, SyncManager,
};
use crate::database::repositories::{
    PlaylistRepository, SettingsRepository, SqliteDownloadRepository, SqliteHistoryRepository,
    SqlitePlaylistRepository, SqliteSettingsRepository, SqliteStatsRepository,
    SqliteUserRepository, SqliteWishlistRepository, StatsRepository, UserProfile, UserRepository,
};
use crate::discovery::{DiscoveryCoordinator, DiscoveryRecommendation, FuzzyTrackMatcher, WishlistManager};
use crate::downloads::{DownloadProvider, DownloadService, SoulseekProvider};
use std::collections::{HashMap, HashSet};
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
use base64::Engine;
use lofty::file::TaggedFileExt;

/// Central processing coordinator for commands, queries, and event dispatching.
pub struct CoreProcessor {
    event_bus: Arc<EventBus>,
    db_pool: SqlitePool,
    library_service: Arc<LibraryService>,
    playback_service: Arc<PlaybackService>,
    history_service: Arc<HistoryService>,
    ranking_engine: Arc<RankingEngine>,
    playlist_repo: Arc<SqlitePlaylistRepository>,
    stats_repo: Arc<SqliteStatsRepository>,
    user_repo: Arc<SqliteUserRepository>,
    cloud_client: Arc<CloudClient>,
    settings_repo: Arc<SqliteSettingsRepository>,
    pub current_user: Arc<RwLock<Option<UserProfile>>>,
    pub session_state: Arc<RwLock<AuthSessionState>>,
    app_start_date: Arc<RwLock<i64>>,
    taste_engine: Arc<TasteProfileEngine>,
    recommender: Arc<LocalRecommender>,
    smart_mix_generator: Arc<SmartMixGenerator>,
    provider_coordinator: Arc<ProviderCoordinator>,
    wishlist_manager: Arc<WishlistManager>,
    discovery_coordinator: Arc<DiscoveryCoordinator>,
    download_service: Arc<DownloadService>,
    config: Arc<RwLock<AppConfig>>,
    cover_art_cache: Arc<tokio::sync::RwLock<HashMap<String, Option<String>>>>,
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
            let ytdlp = Arc::new(crate::downloads::YtDlpProvider::new());
            let soulseek = Arc::new(SoulseekProvider::new(
                &config.downloads.slskd_host,
                config.downloads.slskd_port,
                config.downloads.slskd_api_key.clone(),
            ));
            Arc::new(crate::downloads::CompositeDownloadProvider::new(ytdlp, soulseek))
        });
        let download_dir = config.downloads.download_dir.clone().unwrap_or_else(|| {
            directories::UserDirs::new()
                .and_then(|u| u.audio_dir().map(|p| p.join("Downloads")))
                .unwrap_or_else(|| config.cache_dir.join("downloads"))
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

        let user_repo = Arc::new(SqliteUserRepository::new(db_pool.clone()));
        let cloud_client = Arc::new(CloudClient::new());
        let settings_repo = Arc::new(SqliteSettingsRepository::new(db_pool.clone()));
        let current_user = Arc::new(RwLock::new(None));
        let session_state = Arc::new(RwLock::new(AuthSessionState::SignedOut));
        let app_start_date = Arc::new(RwLock::new(0));

        // Validate active cloud session on launch, sync with server, and enforce hybrid expiry
        let current_user_init = current_user.clone();
        let session_state_init = session_state.clone();
        let pool_init = db_pool.clone();
        let cloud_client_init = cloud_client.clone();
        let settings_repo_init = settings_repo.clone();
        let event_bus_init = event_bus.clone();
        tokio::spawn(async move {
            let worker_url = settings_repo_init
                .get_setting("cloud_sync_url")
                .await
                .ok()
                .flatten()
                .unwrap_or_else(|| "https://soundflow-cloud-worker.abhi-atlas-2026.workers.dev".to_string());

            if let Ok(Some(session)) = SyncManager::get_active_session(&pool_init).await {
                let now = chrono::Utc::now().timestamp();
                let token = credentials::get_session_token(&session.user_id);

                // If session was created for a different worker instance or token missing, reset session
                if token.is_none() || session.worker_url != worker_url {
                    let _ = SyncManager::delete_session(&pool_init, &session.user_id).await;
                    if let Some(_tok) = token {
                        let _ = credentials::delete_session_token(&session.user_id);
                    }
                    *current_user_init.write().await = None;
                    *session_state_init.write().await = AuthSessionState::SignedOut;
                    let _ = event_bus_init.publish(Event::UserLoggedOut);
                    let _ = event_bus_init.publish(Event::SessionChanged { user: None });
                    return;
                }

                // Check local absolute expiry (hard 90-day maximum lifetime)
                if session.absolute_expires_at > 0 && now >= session.absolute_expires_at {
                    let _ = credentials::delete_session_token(&session.user_id);
                    let _ = SyncManager::delete_session(&pool_init, &session.user_id).await;
                    *current_user_init.write().await = None;
                    *session_state_init.write().await = AuthSessionState::SessionExpired {
                        reason: SessionExpiredReason::AbsoluteTimeout,
                    };
                    let _ = event_bus_init.publish(Event::UserLoggedOut);
                    let _ = event_bus_init.publish(Event::SessionChanged { user: None });
                    return;
                }

                // Check local idle expiry (30-day inactivity timeout)
                if session.idle_expires_at > 0 && now >= session.idle_expires_at {
                    let _ = credentials::delete_session_token(&session.user_id);
                    let _ = SyncManager::delete_session(&pool_init, &session.user_id).await;
                    *current_user_init.write().await = None;
                    *session_state_init.write().await = AuthSessionState::SessionExpired {
                        reason: SessionExpiredReason::IdleTimeout,
                    };
                    let _ = event_bus_init.publish(Event::UserLoggedOut);
                    let _ = event_bus_init.publish(Event::SessionChanged { user: None });
                    return;
                }

                let tok = token.unwrap();
                *session_state_init.write().await = AuthSessionState::Authenticating;

                // Validate session against authoritative server
                match cloud_client_init.get_me(&worker_url, &tok).await {
                    Ok(cloud_user) => {
                        let _ = sqlx::query("DELETE FROM users WHERE username = ? OR id = ?")
                            .bind(&cloud_user.username)
                            .bind(&cloud_user.id)
                            .execute(&pool_init)
                            .await;
                        let _ = sqlx::query(
                            "INSERT INTO users (id, username, password_hash, created_at)
                             VALUES (?, ?, '', ?)"
                        )
                        .bind(&cloud_user.id)
                        .bind(&cloud_user.username)
                        .bind(cloud_user.created_at)
                        .execute(&pool_init)
                        .await;

                        let profile = UserProfile {
                            id: cloud_user.id.clone(),
                            username: cloud_user.username.clone(),
                            created_at: cloud_user.created_at,
                        };
                        *current_user_init.write().await = Some(profile.clone());
                        *session_state_init.write().await = AuthSessionState::OnlineAuthenticated {
                            user: cloud_user.clone(),
                            session_id: session.session_id.clone(),
                        };

                        // Associate any orphan custom playlists with this user
                        let _ = sqlx::query("UPDATE playlists SET user_id = ? WHERE is_smart_mix = 0 AND (user_id = 'default' OR user_id IS NULL)")
                            .bind(&cloud_user.id)
                            .execute(&pool_init)
                            .await;

                        let _ = SyncManager::update_validation_timestamp(&pool_init, &session.user_id, now).await;

                        // Broadcast session change to frontend
                        let user_val = serde_json::to_value(&profile).ok();
                        let _ = event_bus_init.publish(Event::SessionChanged { user: user_val });

                        // Automatically sync with server on launch: Pull first, reconcile, push
                        if let Ok(_report) = SyncManager::sync_with_cloud(
                            &pool_init,
                            &cloud_client_init,
                            &worker_url,
                            &cloud_user.id,
                            &tok,
                        ).await {
                            let _ = event_bus_init.publish(Event::PlaylistsUpdated);
                        }
                    }
                    Err(AppError::Validation(_)) => {
                        // User is revoked on server, expired, or doesn't exist on server.
                        // Revoke session metadata only; local library/playlists remain intact but gated.
                        let _ = credentials::delete_session_token(&session.user_id);
                        let _ = SyncManager::delete_session(&pool_init, &session.user_id).await;
                        *current_user_init.write().await = None;
                        *session_state_init.write().await = AuthSessionState::SessionExpired {
                            reason: SessionExpiredReason::Revoked,
                        };
                        let _ = event_bus_init.publish(Event::UserLoggedOut);
                        let _ = event_bus_init.publish(Event::SessionChanged { user: None });
                    }
                    Err(AppError::Network(_)) => {
                        // Truly offline with existing active session: allow offline continuation
                        if session.authenticated_before == 1 {
                            let cloud_user = CloudUser {
                                id: session.user_id.clone(),
                                username: session.username.clone(),
                                created_at: session.created_at,
                            };
                            let profile = UserProfile {
                                id: session.user_id.clone(),
                                username: session.username.clone(),
                                created_at: session.created_at,
                            };
                            *current_user_init.write().await = Some(profile.clone());
                            *session_state_init.write().await = AuthSessionState::OfflineAuthenticated {
                                user: cloud_user,
                                session_id: session.session_id.clone(),
                            };
                            let user_val = serde_json::to_value(&profile).ok();
                            let _ = event_bus_init.publish(Event::SessionChanged { user: user_val });
                        } else {
                            *current_user_init.write().await = None;
                            *session_state_init.write().await = AuthSessionState::CloudUnavailable;
                        }
                    }
                    Err(_) => {
                        *current_user_init.write().await = None;
                        *session_state_init.write().await = AuthSessionState::SignedOut;
                    }
                }
            }
        });

        Self {
            event_bus,
            db_pool,
            library_service,
            playback_service,
            history_service,
            ranking_engine,
            playlist_repo,
            stats_repo,
            user_repo,
            cloud_client,
            settings_repo,
            current_user,
            session_state,
            app_start_date,
            taste_engine,
            recommender,
            smart_mix_generator,
            provider_coordinator,
            wishlist_manager,
            discovery_coordinator,
            download_service,
            config: Arc::new(RwLock::new(config)),
            cover_art_cache: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    async fn get_or_init_app_start_date(&self) -> i64 {
        let mut guard = self.app_start_date.write().await;
        if *guard > 0 {
            return *guard;
        }
        let row: Option<(String,)> = sqlx::query_as("SELECT value FROM application_settings WHERE key = 'app_first_start_date'")
            .fetch_optional(&self.db_pool)
            .await
            .ok()
            .flatten();

        if let Some((val,)) = row {
            if let Ok(ts) = val.parse::<i64>() {
                *guard = ts;
                return ts;
            }
        }

        let now = chrono::Utc::now().timestamp();
        let _ = sqlx::query(
            "INSERT INTO application_settings (key, value, updated_at) VALUES ('app_first_start_date', ?, ?)
             ON CONFLICT(key) DO NOTHING"
        )
        .bind(now.to_string())
        .bind(now)
        .execute(&self.db_pool)
        .await;

        *guard = now;
        now
    }

    /// Retrieve the configured Cloudflare Worker URL (or fallback to production instance default)
    pub async fn get_cloud_worker_url(&self) -> String {
        self.settings_repo
            .get_setting("cloud_sync_url")
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| "https://soundflow-cloud-worker.abhi-atlas-2026.workers.dev".to_string())
    }

    /// Trigger background push sync for the active user if authenticated.
    pub async fn trigger_background_sync(&self) {
        let user_id = {
            let current_user_guard = self.current_user.read().await;
            current_user_guard.as_ref().map(|u| u.id.clone())
        };
        if let Some(uid) = user_id {
            let pool_clone = self.db_pool.clone();
            let client_clone = self.cloud_client.clone();
            let worker_url = self.get_cloud_worker_url().await;
            tokio::spawn(async move {
                if let Some(token) = credentials::get_session_token(&uid) {
                    if let Ok(payload) = SyncManager::prepare_local_sync_payload(&pool_clone, &uid).await {
                        if let Ok(synced_at) = client_clone.push_sync(&worker_url, &token, &payload).await {
                            let _ = SyncManager::clear_tombstones(&pool_clone, &uid).await;
                            let _ = SyncManager::update_session_synced_at(&pool_clone, &uid, synced_at).await;
                        }
                    }
                }
            });
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
            Command::DequeueTrack { track_id } => {
                self.playback_service.dequeue_track(&track_id).await;
                Ok(CommandResponse::Ok)
            }
            Command::ClearQueue => {
                self.playback_service.clear_queue().await;
                Ok(CommandResponse::Ok)
            }

            // --- User Feedback & Taste ---
            Command::LikeTrack { track_id } => {
                self.history_service.set_track_like(&track_id, 1).await?;
                self.trigger_background_sync().await;
                Ok(CommandResponse::Ok)
            }
            Command::DislikeTrack { track_id } => {
                self.history_service.set_track_like(&track_id, -1).await?;
                self.trigger_background_sync().await;
                Ok(CommandResponse::Ok)
            }
            Command::RemoveTrackFeedback { track_id } => {
                self.history_service.set_track_like(&track_id, 0).await?;
                self.trigger_background_sync().await;
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
                let trimmed_name = name.trim().to_string();
                if trimmed_name.is_empty() {
                    return Err(AppError::Validation("Playlist name cannot be empty".into()));
                }

                let current_user_guard = self.current_user.read().await;
                let user_id = current_user_guard.as_ref().map(|u| u.id.clone()).unwrap_or_else(|| "default".to_string());
                drop(current_user_guard);

                if let Some(_) = self.playlist_repo.find_by_name_and_user(&trimmed_name, &user_id).await? {
                    return Err(AppError::Validation(format!(
                        "A playlist named '{}' already exists",
                        trimmed_name
                    )));
                }

                let now = chrono::Utc::now().timestamp();
                let id = format!("pl_{}", uuid::Uuid::new_v4());
                let record = PlaylistRecord {
                    id: id.clone(),
                    name: trimmed_name,
                    description,
                    is_smart_mix: 0,
                    mix_type: None,
                    generation_reason: None,
                    expires_at: None,
                    created_at: now,
                    updated_at: now,
                };
                self.playlist_repo.create_playlist_with_user(&record, &user_id).await?;

                // Background sync if user is logged in
                self.trigger_background_sync().await;

                Ok(CommandResponse::EntityId(id))
            }
            Command::DeletePlaylist { playlist_id } => {
                let user_id = {
                    let current_user_guard = self.current_user.read().await;
                    current_user_guard.as_ref().map(|u| u.id.clone())
                };
                if let Some(ref uid) = user_id {
                    let _ = SyncManager::record_tombstone(&self.db_pool, uid, "playlist", &playlist_id).await;
                }
                self.playlist_repo.delete_playlist(&playlist_id).await?;
                self.trigger_background_sync().await;
                Ok(CommandResponse::Ok)
            }
            Command::AddTrackToPlaylist {
                playlist_id,
                track_id,
                title,
                artist,
                album,
                duration_secs,
                cover_art_url,
                preview_url,
            } => {
                let track_opt = self.library_service.track_repo().find_by_id(&track_id).await?;
                let needs_online_ensure = match &track_opt {
                    None => true,
                    Some(t) => t.title.starts_with("itunes:") || t.title.starts_with("online:") || t.format == "online",
                };

                if needs_online_ensure {
                    let is_clean_title = match &title {
                        Some(t) => !t.trim().is_empty() && !t.starts_with("itunes:") && !t.starts_with("online:"),
                        None => false,
                    };

                    let (t_title, t_artist, t_album, t_dur, t_cover, t_prev) = if is_clean_title {
                        (
                            title.unwrap_or_default(),
                            artist,
                            album,
                            duration_secs,
                            cover_art_url,
                            preview_url,
                        )
                    } else {
                        let ext_opt: Option<(String, String, Option<String>, Option<f64>, Option<String>, Option<String>)> = sqlx::query_as(
                            "SELECT title, artist, album, duration_secs, cover_art_url, preview_url FROM external_tracks WHERE id = ?"
                        )
                        .bind(&track_id)
                        .fetch_optional(&self.db_pool)
                        .await
                        .ok()
                        .flatten();

                        if let Some((e_title, e_artist, e_album, e_dur, e_cov, e_prev)) = ext_opt {
                            (e_title, Some(e_artist), e_album, e_dur, e_cov, e_prev)
                        } else {
                            (title.unwrap_or_else(|| track_id.clone()), artist, album, duration_secs, cover_art_url, preview_url)
                        }
                    };

                    let _ = crate::recommendations::mixes::ensure_online_track(
                        &self.db_pool,
                        &track_id,
                        if t_title.is_empty() { &track_id } else { &t_title },
                        t_artist.as_deref(),
                        t_album.as_deref(),
                        t_dur,
                        t_cover.as_deref(),
                        t_prev.as_deref(),
                    )
                    .await;
                }
                self.playlist_repo.add_track(&playlist_id, &track_id, None).await?;
                self.trigger_background_sync().await;
                Ok(CommandResponse::Ok)
            }
            Command::RemoveTrackFromPlaylist { playlist_id, track_id } => {
                let user_id = {
                    let current_user_guard = self.current_user.read().await;
                    current_user_guard.as_ref().map(|u| u.id.clone())
                };
                if let Some(ref uid) = user_id {
                    let entity_id = format!("{}:{}", playlist_id, track_id);
                    let _ = SyncManager::record_tombstone(&self.db_pool, uid, "playlist_song", &entity_id).await;
                }
                self.playlist_repo.remove_track(&playlist_id, &track_id).await?;
                self.trigger_background_sync().await;
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
            Command::LaunchSoulseek {
                search_query,
                filter_query,
            } => {
                let msg = self.launch_soulseek_qt(search_query, filter_query).await?;
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

            // --- User Authentication & Profiles ---
            Command::SignUp { username, password } => {
                let trimmed_user = username.trim().to_string();
                if trimmed_user.is_empty() {
                    return Err(AppError::Validation("Username cannot be empty".to_string()));
                }
                if trimmed_user.len() < 3 || trimmed_user.len() > 50 {
                    return Err(AppError::Validation("Username must be between 3 and 50 characters".to_string()));
                }
                if !trimmed_user.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.') {
                    return Err(AppError::Validation("Username can only contain alphanumeric characters, underscores, hyphens, and dots".to_string()));
                }
                if password.len() < 8 || password.len() > 128 {
                    return Err(AppError::Validation("Password must be between 8 and 128 characters".to_string()));
                }

                let worker_url = self.get_cloud_worker_url().await;
                let device_id = device::get_or_create_device_id(&self.db_pool).await?;
                let device_name = device::get_device_name();
                let client_ver = device::get_client_version();

                let auth_res = self.cloud_client.register(
                    &worker_url,
                    &trimmed_user,
                    &password,
                    Some(&device_id),
                    Some(&device_name),
                    Some(&client_ver),
                ).await?;

                if let (Some(cloud_user), Some(token)) = (auth_res.user, auth_res.token) {
                    let profile = UserProfile {
                        id: cloud_user.id.clone(),
                        username: cloud_user.username.clone(),
                        created_at: cloud_user.created_at,
                    };

                    let now = chrono::Utc::now().timestamp();
                    let session_meta = auth_res.session;
                    let session_id = session_meta.as_ref().map(|s| s.id.clone()).unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                    let idle_expires_at = session_meta.as_ref().map(|s| s.idle_expires_at).unwrap_or(now + 30 * 86400);
                    let absolute_expires_at = session_meta.as_ref().map(|s| s.absolute_expires_at).unwrap_or(now + 90 * 86400);

                    // Upsert local user profile cleanly
                    let _ = sqlx::query("DELETE FROM users WHERE username = ? OR id = ?")
                        .bind(&profile.username)
                        .bind(&profile.id)
                        .execute(&self.db_pool)
                        .await;
                    let _ = sqlx::query(
                        "INSERT INTO users (id, username, password_hash, created_at)
                         VALUES (?, ?, '', ?)"
                    )
                    .bind(&profile.id)
                    .bind(&profile.username)
                    .bind(profile.created_at)
                    .execute(&self.db_pool)
                    .await;

                    // Store raw bearer token securely in OS keyring (fallback to private local file)
                    if let Err(e) = credentials::store_session_token(&profile.id, &token) {
                        tracing::warn!("Failed to store session token in secure credentials: {}", e);
                    }

                    // Cache session metadata only in cloud_sessions (never store raw tokens in SQLite)
                    let session = CloudSessionMetadata {
                        user_id: profile.id.clone(),
                        username: profile.username.clone(),
                        session_id: session_id.clone(),
                        device_id: device_id.clone(),
                        device_name: device_name.clone(),
                        idle_expires_at,
                        absolute_expires_at,
                        last_cloud_validation_at: Some(now),
                        worker_url: worker_url.clone(),
                        synced_at: None,
                        authenticated_before: 1,
                        created_at: now,
                    };
                    let _ = SyncManager::save_session(&self.db_pool, &session).await;

                    // Reassign orphan playlists to this user
                    let _ = self.playlist_repo.reassign_orphan_playlists_to_user(&profile.id).await;

                    // Claim guest data locally
                    let _ = self.user_repo.claim_guest_data_for_user(&profile.id).await;
                    *self.current_user.write().await = Some(profile.clone());
                    *self.session_state.write().await = AuthSessionState::OnlineAuthenticated {
                        user: cloud_user,
                        session_id,
                    };

                    // Background deterministic sync: pull remote D1 data first, reconcile, then push local
                    let pool_clone = self.db_pool.clone();
                    let client_clone = self.cloud_client.clone();
                    let user_id_clone = profile.id.clone();
                    let event_bus_clone = self.event_bus.clone();
                    tokio::spawn(async move {
                        if let Ok(_report) = SyncManager::sync_with_cloud(
                            &pool_clone,
                            &client_clone,
                            &worker_url,
                            &user_id_clone,
                            &token,
                        ).await {
                            let _ = event_bus_clone.publish(Event::PlaylistsUpdated);
                        }
                    });

                    let val = serde_json::to_value(&profile).unwrap_or_default();
                    let _ = self.event_bus.publish(Event::SessionChanged { user: Some(val.clone()) });
                    return Ok(CommandResponse::UserProfile(val));
                }
                Err(AppError::Validation("Cloud registration did not return user info".to_string()))
            }
            Command::Login { username, password } => {
                let trimmed_user = username.trim().to_string();
                if trimmed_user.is_empty() {
                    return Err(AppError::Validation("Username cannot be empty".to_string()));
                }
                if password.is_empty() {
                    return Err(AppError::Validation("Password cannot be empty".to_string()));
                }

                let worker_url = self.get_cloud_worker_url().await;
                let device_id = device::get_or_create_device_id(&self.db_pool).await?;
                let device_name = device::get_device_name();
                let client_ver = device::get_client_version();

                let auth_res = self.cloud_client.login(
                    &worker_url,
                    &trimmed_user,
                    &password,
                    Some(&device_id),
                    Some(&device_name),
                    Some(&client_ver),
                ).await?;

                if let (Some(cloud_user), Some(token)) = (auth_res.user, auth_res.token) {
                    let profile = UserProfile {
                        id: cloud_user.id.clone(),
                        username: cloud_user.username.clone(),
                        created_at: cloud_user.created_at,
                    };

                    let now = chrono::Utc::now().timestamp();
                    let session_meta = auth_res.session;
                    let session_id = session_meta.as_ref().map(|s| s.id.clone()).unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                    let idle_expires_at = session_meta.as_ref().map(|s| s.idle_expires_at).unwrap_or(now + 30 * 86400);
                    let absolute_expires_at = session_meta.as_ref().map(|s| s.absolute_expires_at).unwrap_or(now + 90 * 86400);

                    // Upsert local user profile (without password hash)
                    let _ = sqlx::query("DELETE FROM users WHERE username = ? OR id = ?")
                        .bind(&profile.username)
                        .bind(&profile.id)
                        .execute(&self.db_pool)
                        .await;
                    let _ = sqlx::query(
                        "INSERT INTO users (id, username, password_hash, created_at)
                         VALUES (?, ?, '', ?)"
                    )
                    .bind(&profile.id)
                    .bind(&profile.username)
                    .bind(profile.created_at)
                    .execute(&self.db_pool)
                    .await;

                    // Store raw bearer token securely in OS keyring (fallback to private local file)
                    if let Err(e) = credentials::store_session_token(&profile.id, &token) {
                        tracing::warn!("Failed to store session token in secure credentials: {}", e);
                    }

                    // Cache session metadata only in cloud_sessions (never store raw tokens in SQLite)
                    let session = CloudSessionMetadata {
                        user_id: profile.id.clone(),
                        username: profile.username.clone(),
                        session_id: session_id.clone(),
                        device_id: device_id.clone(),
                        device_name: device_name.clone(),
                        idle_expires_at,
                        absolute_expires_at,
                        last_cloud_validation_at: Some(now),
                        worker_url: worker_url.clone(),
                        synced_at: None,
                        authenticated_before: 1,
                        created_at: now,
                    };
                    let _ = SyncManager::save_session(&self.db_pool, &session).await;

                    // Reassign orphan playlists to this user
                    let _ = self.playlist_repo.reassign_orphan_playlists_to_user(&profile.id).await;

                    *self.current_user.write().await = Some(profile.clone());
                    *self.session_state.write().await = AuthSessionState::OnlineAuthenticated {
                        user: cloud_user,
                        session_id,
                    };

                    tracing::info!("LOGIN SUCCESS: authenticated user_id = {}", profile.id);

                    // Deterministic bidirectional sync: pull remote D1 data down first, reconcile, then push local
                    let pool_clone = self.db_pool.clone();
                    let client_clone = self.cloud_client.clone();
                    let user_id_clone = profile.id.clone();
                    let event_bus_clone = self.event_bus.clone();
                    tokio::spawn(async move {
                        if let Ok(_report) = SyncManager::sync_with_cloud(
                            &pool_clone,
                            &client_clone,
                            &worker_url,
                            &user_id_clone,
                            &token,
                        ).await {
                            let _ = event_bus_clone.publish(Event::PlaylistsUpdated);
                        }
                    });

                    let val = serde_json::to_value(&profile).unwrap_or_default();
                    let _ = self.event_bus.publish(Event::SessionChanged { user: Some(val.clone()) });
                    return Ok(CommandResponse::UserProfile(val));
                }
                Err(AppError::Validation("Cloud login did not return user info".to_string()))
            }
            Command::Logout => {
                let worker_url = self.get_cloud_worker_url().await;
                if let Ok(Some(session)) = SyncManager::get_active_session(&self.db_pool).await {
                    let client = self.cloud_client.clone();
                    let user_id = session.user_id.clone();
                    let token = credentials::get_session_token(&user_id);
                    if let Some(tok) = token {
                        tokio::spawn(async move {
                            let _ = client.logout(&worker_url, &tok).await;
                        });
                    }
                    let _ = SyncManager::delete_session(&self.db_pool, &user_id).await;
                    let _ = credentials::delete_session_token(&user_id);
                }
                *self.current_user.write().await = None;
                *self.session_state.write().await = AuthSessionState::SignedOut;

                // Stop any playing audio and clear playback queue to match fresh startup state
                let _ = self.playback_service.stop().await;
                self.playback_service.clear_queue().await;

                // Note: Authentication controls ACCESS to user data.
                // Logout clears session credentials from active memory/keyring,
                // but does NOT destroy user playlists, songs, statistics, or history.
                let _ = self.event_bus.publish(Event::UserLoggedOut);
                let _ = self.event_bus.publish(Event::SessionChanged { user: None });

                Ok(CommandResponse::Ok)
            }
            Command::LogoutAll => {
                let worker_url = self.get_cloud_worker_url().await;
                if let Ok(Some(session)) = SyncManager::get_active_session(&self.db_pool).await {
                    let client = self.cloud_client.clone();
                    let user_id = session.user_id.clone();
                    let token = credentials::get_session_token(&user_id);
                    if let Some(tok) = token {
                        let _ = client.logout_all(&worker_url, &tok).await;
                    }
                    let _ = SyncManager::delete_session(&self.db_pool, &user_id).await;
                    let _ = credentials::delete_session_token(&user_id);
                }
                *self.current_user.write().await = None;
                *self.session_state.write().await = AuthSessionState::SignedOut;

                // Stop any playing audio and clear playback queue to match fresh startup state
                let _ = self.playback_service.stop().await;
                self.playback_service.clear_queue().await;

                // Invalidate all cloud sessions, but preserve local stored data
                let _ = self.event_bus.publish(Event::UserLoggedOut);
                let _ = self.event_bus.publish(Event::SessionChanged { user: None });

                Ok(CommandResponse::Ok)
            }
            Command::RevokeSession { session_id } => {
                let worker_url = self.get_cloud_worker_url().await;
                let active = SyncManager::get_active_session(&self.db_pool).await.ok().flatten();
                if let Some(session) = active {
                    let token = credentials::get_session_token(&session.user_id)
                        .ok_or_else(|| AppError::Validation("No active session token".to_string()))?;
                    self.cloud_client.revoke_session(&worker_url, &token, &session_id).await?;
                    if session.session_id == session_id {
                        let _ = SyncManager::delete_session(&self.db_pool, &session.user_id).await;
                        *self.current_user.write().await = None;
                        *self.session_state.write().await = AuthSessionState::SignedOut;
                    }
                }
                Ok(CommandResponse::Ok)
            }
            Command::SyncCloudData => {
                let worker_url = self.get_cloud_worker_url().await;
                let session = SyncManager::get_active_session(&self.db_pool)
                    .await?
                    .ok_or_else(|| AppError::Validation("Not logged in to a cloud account".to_string()))?;

                let now = chrono::Utc::now().timestamp();
                let is_idle_expired = session.idle_expires_at > 0 && now >= session.idle_expires_at;
                let is_abs_expired = session.absolute_expires_at > 0 && now >= session.absolute_expires_at;
                if is_idle_expired || is_abs_expired {
                    *self.session_state.write().await = AuthSessionState::SessionExpired {
                        reason: if is_abs_expired {
                            SessionExpiredReason::AbsoluteTimeout
                        } else {
                            SessionExpiredReason::IdleTimeout
                        },
                    };
                    return Err(AppError::Validation("Session expired. Please log in again.".to_string()));
                }

                let token = credentials::get_session_token(&session.user_id)
                    .ok_or_else(|| AppError::Validation("Session credentials not found locally. Please log in again.".to_string()))?;

                // Deterministic sync: Pull first, reconcile, push
                let report = SyncManager::sync_with_cloud(
                    &self.db_pool,
                    &self.cloud_client,
                    &worker_url,
                    &session.user_id,
                    &token,
                ).await?;

                SyncManager::update_validation_timestamp(&self.db_pool, &session.user_id, now).await?;
                let _ = self.event_bus.publish(Event::PlaylistsUpdated);

                Ok(CommandResponse::CloudSyncCompleted { synced_at: report.synced_at })
            }
            Command::SetCloudServerUrl { url } => {
                let trimmed = url.trim().to_string();
                if trimmed.is_empty() {
                    return Err(AppError::Validation("Server URL cannot be empty".to_string()));
                }
                self.settings_repo.set_setting("cloud_sync_url", &trimmed).await?;
                Ok(CommandResponse::Ok)
            }
            Command::RecordPlaybackSession {
                track_id,
                title,
                artist,
                album,
                duration_secs,
                seconds_listened,
                completed,
                skipped,
                source,
            } => {
                let now = chrono::Utc::now().timestamp();
                let current_user_guard = self.current_user.read().await;
                let user_id = current_user_guard.as_ref().map(|u| u.id.as_str()).unwrap_or("default");

                // Ensure external track exists in db if needed
                let _ = sqlx::query(
                    "INSERT INTO external_tracks (id, provider, provider_id, title, artist, album, duration_secs, created_at)
                     VALUES (?, 'online', ?, ?, ?, ?, ?, ?)
                     ON CONFLICT(provider, provider_id) DO UPDATE SET title = excluded.title, artist = excluded.artist"
                )
                .bind(&track_id)
                .bind(&track_id)
                .bind(&title)
                .bind(artist.as_deref().unwrap_or("Unknown Artist"))
                .bind(album.as_deref().unwrap_or(""))
                .bind(duration_secs)
                .bind(now)
                .execute(&self.db_pool)
                .await;

                let _ = crate::recommendations::mixes::ensure_online_track(
                    &self.db_pool,
                    &track_id,
                    if title.is_empty() { &track_id } else { &title },
                    artist.as_deref(),
                    album.as_deref(),
                    Some(duration_secs),
                    None,
                    None,
                )
                .await;

                let percentage = if duration_secs > 0.0 {
                    (seconds_listened / duration_secs).clamp(0.0, 1.0)
                } else {
                    0.0
                };

                let entry_id = uuid::Uuid::new_v4().to_string();
                let started_at = now - (seconds_listened.round() as i64);

                sqlx::query(
                    "INSERT INTO playback_history (
                        id, track_id, started_at, ended_at, seconds_listened,
                        percentage_listened, completed, skipped, source, playlist_id,
                        recommendation_session_id, user_id
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
                )
                .bind(&entry_id)
                .bind(&track_id)
                .bind(started_at)
                .bind(now)
                .bind(seconds_listened)
                .bind(percentage)
                .bind(if completed { 1 } else { 0 })
                .bind(if skipped { 1 } else { 0 })
                .bind(&source)
                .bind(None::<String>)
                .bind(None::<String>)
                .bind(user_id)
                .execute(&self.db_pool)
                .await
                .map_err(|e| AppError::Database(format!("Failed to record playback history: {}", e)))?;

                let is_meaningful = seconds_listened >= 30.0 || completed;
                let _ = self.stats_repo.update_track_playback_stats(
                    &track_id,
                    seconds_listened,
                    is_meaningful,
                    completed,
                    skipped,
                ).await;

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
                let mut val = serde_json::to_value(&state)
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                if let Some(track_id) = &state.current_track_id {
                    if let Ok(Some(track)) = self.library_service.track_repo().find_by_id(track_id).await {
                        if let Ok(track_val) = serde_json::to_value(track) {
                            if let Some(obj) = val.as_object_mut() {
                                obj.insert("current_track".to_string(), track_val);
                            }
                        }
                    }
                }
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
                let mut val: Vec<serde_json::Value> = Vec::new();
                for m in mixes {
                    let count = self.playlist_repo.get_track_count(&m.id).await.unwrap_or(0);
                    if let Ok(mut v) = serde_json::to_value(&m) {
                        if let Some(obj) = v.as_object_mut() {
                            obj.insert("track_count".to_string(), serde_json::json!(count));
                        }
                        val.push(v);
                    }
                }
                Ok(QueryResponse::SmartMixes(val))
            }
            Query::GetPlaylists => {
                let _ = self.smart_mix_generator.ensure_default_mixes().await;
                let current_user_guard = self.current_user.read().await;
                let playlists = if let Some(ref u) = *current_user_guard {
                    self.playlist_repo.get_user_playlists(&u.id).await?
                } else {
                    self.playlist_repo.get_smart_mixes().await?
                };
                let mut val: Vec<serde_json::Value> = Vec::new();
                for p in playlists {
                    let count = self.playlist_repo.get_track_count(&p.id).await.unwrap_or(0);
                    if let Ok(mut v) = serde_json::to_value(&p) {
                        if let Some(obj) = v.as_object_mut() {
                            obj.insert("track_count".to_string(), serde_json::json!(count));
                        }
                        val.push(v);
                    }
                }
                Ok(QueryResponse::Playlists(val))
            }
            Query::GetPlaylistTracks { playlist_id } => {
                let _ = crate::recommendations::mixes::repair_legacy_online_tracks(&self.db_pool).await;
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
            Query::GetDiscoveryRecommendations {
                limit,
                force_refresh,
            } => {
                let recs = self
                    .discovery_coordinator
                    .get_discovery_recommendations(limit as usize, force_refresh.unwrap_or(false))
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
            Query::ResolveFullTrackAudio { artist, title } => {
                let res = self
                    .download_service
                    .resolve_full_track_audio(&artist, &title)
                    .await?;
                if let Some((stream_url, duration_secs)) = res {
                    Ok(QueryResponse::FullTrackAudio {
                        stream_url,
                        duration_secs,
                    })
                } else {
                    Err(crate::core::error::AppError::NotFound(
                        "Full track stream not available".to_string(),
                    ))
                }
            }
            Query::SearchOnlineMusic { query, limit } => {
                info!(query = %query, "Processor dispatching SearchOnlineMusic");
                let limit_val = limit.unwrap_or(30) as usize;
                let mut results = self
                    .discovery_coordinator
                    .search_online_music(&query, limit_val)
                    .await
                    .unwrap_or_else(|e| {
                        warn!(error = %e, "search_online_music failed, falling back to empty list");
                        Vec::new()
                    });

                // Dual-source fallback: If online iTunes search returns few or zero items,
                // fall back to download network (yt-dlp / YouTube), which captures regional, indie, and unreleased music
                if results.len() < 5 {
                    info!(query = %query, existing_count = results.len(), "Augmenting online search with audio download network");
                    if let Ok(dl_results) = self.download_service.search(&query).await {
                        let mut seen_keys: HashSet<String> = results
                            .iter()
                            .map(|r| format!("{}:{}", r.artist.to_lowercase().trim(), r.title.to_lowercase().trim()))
                            .collect();

                        let local_tracks = self
                            .library_service
                            .track_repo()
                            .list_tracks(0, 10000, None, true)
                            .await
                            .unwrap_or_default();
                        let candidate_local_tuples: Vec<(&str, &str, &str, f64)> = local_tracks
                            .iter()
                            .filter(|t| t.format != "online" && !t.file_path.starts_with("online://") && !t.id.starts_with("itunes:") && !t.id.starts_with("online:"))
                            .map(|t| (t.id.as_str(), t.title.as_str(), t.artist_name.as_deref().unwrap_or(""), t.duration_secs))
                            .collect();

                        let wishlist_items = self.wishlist_manager.get_wishlist(None).await.unwrap_or_default();
                        let wishlist_keys: HashSet<String> = wishlist_items
                            .iter()
                            .map(|w| format!("{}:{}", w.artist.to_lowercase().trim(), w.title.to_lowercase().trim()))
                            .collect();

                        for dl_res in dl_results {
                            let raw_name = dl_res.filename.trim_end_matches(".mp3").trim_end_matches(".flac");
                            let (artist, title) = if let Some((a, t)) = raw_name.split_once(" - ") {
                                (a.trim().to_string(), t.trim().to_string())
                            } else {
                                (dl_res.username.clone(), raw_name.to_string())
                            };

                            let key = format!("{}:{}", artist.to_lowercase().trim(), title.to_lowercase().trim());
                            if seen_keys.contains(&key) {
                                continue;
                            }
                            seen_keys.insert(key.clone());

                            let match_res = FuzzyTrackMatcher::find_best_match(
                                &title,
                                &artist,
                                None,
                                candidate_local_tuples.iter().copied(),
                            );
                            let in_wishlist = wishlist_keys.contains(&key);

                            results.push(DiscoveryRecommendation {
                                external_track_id: format!("download:{}", dl_res.id),
                                provider: dl_res.provider.clone(),
                                provider_id: dl_res.id.clone(),
                                title,
                                artist,
                                album: None,
                                duration_secs: Some(210.0),
                                cover_art_url: None,
                                preview_url: None,
                                genre: Some("Online Audio Stream".to_string()),
                                match_status: match_res.status,
                                matched_local_track_id: match_res.matched_track_id,
                                recommendation_reason: format!("Stream source for \"{}\"", query.trim()),
                                in_wishlist,
                            });

                            if results.len() >= limit_val {
                                break;
                            }
                        }
                    }
                }

                info!(count = results.len(), "Processor returning SearchOnlineMusic recommendations");
                let json_arr: Vec<serde_json::Value> = results
                    .into_iter()
                    .map(|r| serde_json::to_value(r).unwrap_or_default())
                    .collect();
                Ok(QueryResponse::DiscoveryRecommendations(json_arr))
            }
            Query::GetTrackCoverArt { track_id } => {
                if let Some(cached) = self.cover_art_cache.read().await.get(&track_id) {
                    return Ok(QueryResponse::CoverArt(cached.clone()));
                }

                let mut cover_result: Option<String> = None;
                let track = self
                    .library_service
                    .track_repo()
                    .find_by_id(&track_id)
                    .await?;
                if let Some(t) = track {
                    let path = std::path::Path::new(&t.file_path);
                    if path.exists() {
                        if let Ok(probe) = lofty::probe::Probe::open(path) {
                            if let Ok(probe) = probe.guess_file_type() {
                                if let Ok(tagged_file) = probe.read() {
                                    // Search across all tags for pictures
                                    let picture = tagged_file
                                        .tags()
                                        .iter()
                                        .find_map(|tag| tag.pictures().first())
                                        .or_else(|| tagged_file.primary_tag().and_then(|tag| tag.pictures().first()))
                                        .or_else(|| tagged_file.first_tag().and_then(|tag| tag.pictures().first()));

                                    if let Some(pic) = picture {
                                        let mime = pic.mime_type().map(|m| m.as_str()).unwrap_or("image/jpeg");
                                        let encoded = base64::engine::general_purpose::STANDARD.encode(pic.data());
                                        cover_result = Some(format!("data:{};base64,{}", mime, encoded));
                                    }
                                }
                            }
                        }

                        // If not found in tags, check folder for cover.jpg / folder.jpg / album.jpg / cover.png
                        if cover_result.is_none() {
                            if let Some(parent) = path.parent() {
                                for candidate in &["cover.jpg", "folder.jpg", "album.jpg", "cover.png", "folder.png", "front.jpg", "front.png"] {
                                    let img_path = parent.join(candidate);
                                    if img_path.is_file() {
                                        if let Ok(bytes) = std::fs::read(&img_path) {
                                            let mime = if candidate.ends_with(".png") { "image/png" } else { "image/jpeg" };
                                            let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
                                            cover_result = Some(format!("data:{};base64,{}", mime, encoded));
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                self.cover_art_cache.write().await.insert(track_id, cover_result.clone());
                Ok(QueryResponse::CoverArt(cover_result))
            }
            Query::GetTrackPlaylistMemberships => {
                let current_user_guard = self.current_user.read().await;
                let memberships = if let Some(ref u) = *current_user_guard {
                    self.playlist_repo.get_track_playlist_memberships_for_user(&u.id).await?
                } else {
                    std::collections::HashMap::new()
                };
                let val = serde_json::to_value(&memberships)
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                Ok(QueryResponse::TrackPlaylistMemberships(val))
            }
            Query::GetTrackLyrics {
                track_id,
                artist,
                title,
                duration_secs,
            } => {
                // 1. If local track_id provided, attempt to read embedded lyrics from file tags
                if let Some(ref tid) = track_id {
                    if let Ok(Some(track)) = self.library_service.track_repo().find_by_id(tid).await {
                        let path = std::path::Path::new(&track.file_path);
                        if path.exists() {
                            if let Ok(probe) = lofty::probe::Probe::open(path) {
                                if let Ok(probe) = probe.guess_file_type() {
                                    if let Ok(tagged_file) = probe.read() {
                                        if let Some(tag) = tagged_file.primary_tag().or_else(|| tagged_file.first_tag()) {
                                            if let Some(lyrics_val) = tag.get_string(&lofty::tag::ItemKey::Lyrics) {
                                                let is_synced = lyrics_val.contains('[') && lyrics_val.contains(']');
                                                let payload = serde_json::json!({
                                                    "trackName": title,
                                                    "artistName": artist,
                                                    "plainLyrics": if is_synced { None } else { Some(lyrics_val) },
                                                    "syncedLyrics": if is_synced { Some(lyrics_val) } else { None },
                                                });
                                                return Ok(QueryResponse::Lyrics(Some(payload)));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // 2. Query LRCLIB API for time-synced or plain lyrics
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(6))
                    .build()
                    .unwrap_or_default();

                let mut params = vec![
                    ("artist_name", artist.clone()),
                    ("track_name", title.clone()),
                ];
                let dur_str;
                if let Some(d) = duration_secs {
                    if d > 0.0 {
                        dur_str = format!("{}", d.round() as u64);
                        params.push(("duration", dur_str));
                    }
                }

                match client.get("https://lrclib.net/api/get").query(&params).send().await {
                    Ok(resp) if resp.status().is_success() => {
                        if let Ok(json) = resp.json::<serde_json::Value>().await {
                            let plain = json.get("plainLyrics").and_then(|v| v.as_str()).map(|s| s.to_string());
                            let synced = json.get("syncedLyrics").and_then(|v| v.as_str()).map(|s| s.to_string());
                            let payload = serde_json::json!({
                                "trackName": title,
                                "artistName": artist,
                                "plainLyrics": plain,
                                "syncedLyrics": synced,
                            });
                            return Ok(QueryResponse::Lyrics(Some(payload)));
                        }
                    }
                    _ => {}
                }

                Ok(QueryResponse::Lyrics(None))
            }
            Query::GetStatsOverview { year, month } => {
                let current_user_guard = self.current_user.read().await;
                let user_id = current_user_guard.as_ref().map(|u| u.id.as_str()).unwrap_or("default");
                let app_start_date = self.get_or_init_app_start_date().await;
                let user_joined_date = current_user_guard.as_ref().map(|u| u.created_at).unwrap_or(app_start_date);
                let weights = self.config.read().await.ranking.clone();
                let overview = self.stats_repo.get_stats_overview(
                    user_id,
                    user_joined_date,
                    app_start_date,
                    year,
                    month,
                    &weights,
                ).await?;
                let val = serde_json::to_value(&overview).unwrap_or_default();
                Ok(QueryResponse::StatsOverview(val))
            }
            Query::GetCurrentUser => {
                let current_user_guard = self.current_user.read().await;
                if let Some(ref u) = *current_user_guard {
                    let val = serde_json::to_value(u).ok();
                    return Ok(QueryResponse::CurrentUser(val));
                }
                drop(current_user_guard);

                // If startup background validation is still resolving, return active valid local session
                if let Ok(Some(session)) = SyncManager::get_active_session(&self.db_pool).await {
                    let now = chrono::Utc::now().timestamp();
                    let worker_url = self.get_cloud_worker_url().await;
                    let is_idle_expired = session.idle_expires_at > 0 && now >= session.idle_expires_at;
                    let is_abs_expired = session.absolute_expires_at > 0 && now >= session.absolute_expires_at;
                    if !is_idle_expired && !is_abs_expired && session.worker_url == worker_url {
                        if let Some(_tok) = credentials::get_session_token(&session.user_id) {
                            let profile = UserProfile {
                                id: session.user_id.clone(),
                                username: session.username.clone(),
                                created_at: session.created_at,
                            };
                            *self.current_user.write().await = Some(profile.clone());
                            *self.session_state.write().await = AuthSessionState::OnlineAuthenticated {
                                user: CloudUser {
                                    id: session.user_id.clone(),
                                    username: session.username.clone(),
                                    created_at: session.created_at,
                                },
                                session_id: session.session_id.clone(),
                            };
                            let _ = self.playlist_repo.reassign_orphan_playlists_to_user(&session.user_id).await;
                            return Ok(QueryResponse::CurrentUser(serde_json::to_value(&profile).ok()));
                        }
                    }
                }

                Ok(QueryResponse::CurrentUser(None))
            }
            Query::GetCloudSyncStatus => {
                let worker_url = self.get_cloud_worker_url().await;
                let session = SyncManager::get_active_session(&self.db_pool).await.ok().flatten();
                let current_user_guard = self.current_user.read().await;
                let current_state = self.session_state.read().await.clone();

                let status = CloudSyncStatus {
                    connected: session.is_some() && !matches!(current_state, AuthSessionState::SignedOut | AuthSessionState::SessionExpired { .. }),
                    worker_url,
                    user_id: current_user_guard.as_ref().map(|u| u.id.clone()),
                    username: current_user_guard.as_ref().map(|u| u.username.clone()),
                    session_id: session.as_ref().map(|s| s.session_id.clone()),
                    device_name: session.as_ref().map(|s| s.device_name.clone()),
                    idle_expires_at: session.as_ref().map(|s| s.idle_expires_at),
                    absolute_expires_at: session.as_ref().map(|s| s.absolute_expires_at),
                    last_synced_at: session.and_then(|s| s.synced_at),
                    session_state: Some(current_state),
                };

                let val = serde_json::to_value(&status).unwrap_or_default();
                Ok(QueryResponse::CloudSyncStatus(val))
            }
            Query::GetSessionState => {
                let state = self.session_state.read().await.clone();
                let val = serde_json::to_value(&state).unwrap_or_default();
                Ok(QueryResponse::SessionState(val))
            }
            Query::ListSessions => {
                let worker_url = self.get_cloud_worker_url().await;
                let session = SyncManager::get_active_session(&self.db_pool).await.ok().flatten();
                if let Some(session) = session {
                    if let Some(token) = credentials::get_session_token(&session.user_id) {
                        match self.cloud_client.list_sessions(&worker_url, &token).await {
                            Ok(sessions) => {
                                let val: Vec<serde_json::Value> = sessions
                                    .into_iter()
                                    .filter_map(|s| serde_json::to_value(s).ok())
                                    .collect();
                                return Ok(QueryResponse::Sessions(val));
                            }
                            Err(e) => {
                                tracing::warn!("Failed to fetch remote sessions: {}", e);
                            }
                        }
                    }
                }
                Ok(QueryResponse::Sessions(Vec::new()))
            }
            _ => {
                warn!(?query, "Query handler routed to fallback");
                Ok(QueryResponse::Empty)
            }
        }
    }

    async fn launch_soulseek_qt(
        &self,
        search_query: Option<String>,
        filter_query: Option<String>,
    ) -> AppResult<String> {
        // 1. Copy search query to clipboard
        let clip_text = search_query
            .as_deref()
            .or(filter_query.as_deref());

        if let Some(q) = clip_text {
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

        // 2. Check if SoulseekQt is currently running
        let mut is_running = false;
        if let Ok(output) = std::process::Command::new("pgrep")
            .arg("-f")
            .arg("SoulseekQt")
            .output()
        {
            if output.status.success() && !output.stdout.is_empty() {
                is_running = true;
            }
        }

        let possible_paths = [
            "/home/abhi/Applications/SoulseekQt-2024-6-30.AppImage",
            "/home/abhi/Applications/SoulseekQt.AppImage",
            "/home/abhi/Documents/SoulseekQt-2024-6-30.AppImage",
        ];

        let mut launched_path = String::new();
        if !is_running {
            let mut launched = false;
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
        } else {
            launched_path = "running instance".to_string();
        }

        // 3. If a search query or filter query is provided, automate SoulseekQt GUI search
        if search_query.is_some() || filter_query.is_some() {
            let query = search_query.clone();
            let filter = filter_query.clone();
            let need_wait_for_launch = !is_running;
            tokio::spawn(async move {
                Self::automate_soulseek_search(query, filter, need_wait_for_launch).await;
            });
        }

        let msg = match (&search_query, &filter_query) {
            (Some(q), Some(f)) => format!("Searching for \"{}\" with filter \"{}\" in SoulseekQt...", q, f),
            (Some(q), None) => format!("Searching for \"{}\" in SoulseekQt...", q),
            (None, Some(f)) => format!("Filtering for \"{}\" in SoulseekQt...", f),
            (None, None) => format!("Launched SoulseekQt ({})", launched_path),
        };

        info!("{}", msg);
        Ok(msg)
    }

    async fn automate_soulseek_search(
        query: Option<String>,
        filter: Option<String>,
        need_wait_for_launch: bool,
    ) {
        // Wait for window to appear or initialize
        if need_wait_for_launch {
            for _ in 0..30 {
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                if let Ok(out) = std::process::Command::new("xdotool")
                    .env("DISPLAY", ":0")
                    .args(["search", "--class", "soulseek"])
                    .output()
                {
                    if out.status.success() && !out.stdout.is_empty() {
                        tokio::time::sleep(tokio::time::Duration::from_millis(600)).await;
                        break;
                    }
                }
            }
        } else {
            tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
        }

        // 1. Focus in Niri (Wayland compositor) if running under Niri
        if let Ok(output) = std::process::Command::new("niri")
            .args(["msg", "--json", "windows"])
            .output()
        {
            if output.status.success() {
                if let Ok(windows) = serde_json::from_slice::<Vec<serde_json::Value>>(&output.stdout) {
                    for win in windows {
                        let app_id = win.get("app_id").and_then(|v| v.as_str()).unwrap_or("");
                        let title = win.get("title").and_then(|v| v.as_str()).unwrap_or("");
                        if app_id.to_lowercase().contains("soulseek") || title.to_lowercase().contains("soulseek") {
                            if let Some(id) = win.get("id").and_then(|v| v.as_u64()) {
                                let _ = std::process::Command::new("niri")
                                    .args(["msg", "action", "focus-window", "--id", &id.to_string()])
                                    .status();
                                break;
                            }
                        }
                    }
                }
            }
        }

        // 2. Find X11 Window ID via xdotool
        let wid_output = std::process::Command::new("xdotool")
            .env("DISPLAY", ":0")
            .args(["search", "--class", "soulseek"])
            .output();

        let wid = match wid_output {
            Ok(out) if out.status.success() => {
                String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .next()
                    .map(|s| s.trim().to_string())
            }
            _ => None,
        };

        if let Some(wid) = wid {
            if !wid.is_empty() {
                // Focus window in X11
                let _ = std::process::Command::new("xdotool")
                    .env("DISPLAY", ":0")
                    .args(["windowfocus", &wid])
                    .status();

                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                // 1. If query is provided, execute primary search
                if let Some(ref q) = query {
                    // Put query in clipboard
                    let _ = std::process::Command::new("wl-copy").arg(q).status();

                    // Click Search Tab
                    let _ = std::process::Command::new("xdotool")
                        .env("DISPLAY", ":0")
                        .args(["mousemove", "--window", &wid, "265", "52", "click", "1"])
                        .status();

                    tokio::time::sleep(tokio::time::Duration::from_millis(80)).await;

                    // Click Manual Searches sub-tab
                    let _ = std::process::Command::new("xdotool")
                        .env("DISPLAY", ":0")
                        .args(["mousemove", "--window", &wid, "85", "86", "click", "1"])
                        .status();

                    tokio::time::sleep(tokio::time::Duration::from_millis(80)).await;

                    // Click Search Input Box
                    let _ = std::process::Command::new("xdotool")
                        .env("DISPLAY", ":0")
                        .args(["mousemove", "--window", &wid, "200", "122", "click", "1"])
                        .status();

                    tokio::time::sleep(tokio::time::Duration::from_millis(80)).await;

                    // Clear input box
                    let _ = std::process::Command::new("xdotool")
                        .env("DISPLAY", ":0")
                        .args(["key", "ctrl+a", "BackSpace"])
                        .status();

                    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

                    // Paste query
                    let _ = std::process::Command::new("xdotool")
                        .env("DISPLAY", ":0")
                        .args(["key", "ctrl+v"])
                        .status();

                    tokio::time::sleep(tokio::time::Duration::from_millis(80)).await;

                    // Press Return to execute search
                    let _ = std::process::Command::new("xdotool")
                        .env("DISPLAY", ":0")
                        .args(["key", "Return"])
                        .status();

                    tokio::time::sleep(tokio::time::Duration::from_millis(60)).await;

                    // Click Search button as well
                    let _ = std::process::Command::new("xdotool")
                        .env("DISPLAY", ":0")
                        .args(["mousemove", "--window", &wid, "725", "122", "click", "1"])
                        .status();

                    info!(query = %q, wid = %wid, "Automated SoulseekQt primary search");
                }

                // 2. If filter is provided, put it into the search result filter box
                if let Some(ref f) = filter {
                    if !f.trim().is_empty() {
                        // Wait for search tab to open
                        tokio::time::sleep(tokio::time::Duration::from_millis(350)).await;

                        // Copy filter text to clipboard
                        let _ = std::process::Command::new("wl-copy").arg(f).status();

                        // Click Filter Box at the bottom of the active search tab (x=535, y=946 relative to window)
                        let _ = std::process::Command::new("xdotool")
                            .env("DISPLAY", ":0")
                            .args(["mousemove", "--window", &wid, "535", "946", "click", "1"])
                            .status();

                        tokio::time::sleep(tokio::time::Duration::from_millis(80)).await;

                        // Clear filter box
                        let _ = std::process::Command::new("xdotool")
                            .env("DISPLAY", ":0")
                            .args(["key", "ctrl+a", "BackSpace"])
                            .status();

                        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

                        // Paste filter text
                        let _ = std::process::Command::new("xdotool")
                            .env("DISPLAY", ":0")
                            .args(["key", "ctrl+v"])
                            .status();

                        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

                        // Hit Return
                        let _ = std::process::Command::new("xdotool")
                            .env("DISPLAY", ":0")
                            .args(["key", "Return"])
                            .status();

                        info!(filter = %f, wid = %wid, "Automated SoulseekQt search result filter");
                    }
                }

                return;
            }
        }

        warn!("Could not locate SoulseekQt window to automate search/filter");
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
