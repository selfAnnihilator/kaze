use crate::cloud::{
    credentials, device, AuthSessionState, CloudClient, CloudSessionMetadata, CloudSyncStatus,
    CloudUser, SessionExpiredReason, SyncManager,
};
use crate::config::AppConfig;
use crate::core::command::{Command, CommandResponse};
use crate::core::error::{AppError, AppResult};
use crate::core::event::Event;
use crate::core::event_bus::EventBus;
use crate::core::query::{Query, QueryResponse};
use crate::database::models::PlaylistRecord;
use crate::database::repositories::{
    PlaylistRepository, SettingsRepository, SqliteDownloadRepository, SqliteHistoryRepository,
    SqlitePlaylistRepository, SqliteSettingsRepository, SqliteStatsRepository,
    SqliteUserRepository, SqliteWishlistRepository, StatsRepository, UserProfile, UserRepository,
};
use crate::discovery::{DiscoveryCoordinator, WishlistManager};
use crate::downloads::{DownloadProvider, DownloadService, JioSaavnDownloadProvider};
use crate::history::HistoryService;
use crate::library::LibraryService;
use crate::playback::backend::{AudioBackend, RodioAudioBackend};
use crate::playback::PlaybackService;
use crate::profile::ProfileService;
use crate::providers::ProviderCoordinator;
use crate::ranking::RankingEngine;
use crate::recommendations::{LocalRecommender, SmartMixGenerator, TasteProfileEngine};
use base64::Engine;
use lofty::file::TaggedFileExt;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::{Mutex, OnceCell, RwLock};
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
    stats_repo: Arc<SqliteStatsRepository>,
    user_repo: Arc<SqliteUserRepository>,
    cloud_client: Arc<CloudClient>,
    settings_repo: Arc<SqliteSettingsRepository>,
    pub profile_service: Arc<ProfileService>,
    pub current_user: Arc<RwLock<Option<UserProfile>>>,
    pub session_state: Arc<RwLock<AuthSessionState>>,
    app_start_date: Arc<RwLock<i64>>,
    taste_engine: Arc<TasteProfileEngine>,
    recommender: Arc<LocalRecommender>,
    smart_mix_generator: Arc<SmartMixGenerator>,
    provider_coordinator: Arc<ProviderCoordinator>,
    gap_enrichment: Arc<crate::providers::enrichment::GapEnrichmentService>,
    wishlist_manager: Arc<WishlistManager>,
    discovery_coordinator: Arc<DiscoveryCoordinator>,
    download_service: Arc<DownloadService>,
    flow_engine: Arc<crate::recommendations::FlowEngine>,
    online_catalog: Arc<crate::providers::online_catalog::OnlineCatalog>,
    config: Arc<RwLock<AppConfig>>,
    cover_art_cache: Arc<tokio::sync::RwLock<HashMap<String, Option<String>>>>,
    is_quitting: AtomicBool,
    close_button_quits: AtomicBool,
    close_behavior_ready: OnceCell<()>,
    close_behavior_updates: Mutex<()>,
    command_gate: RwLock<()>,
    shutdown_done: Mutex<bool>,
    background_tasks: StdMutex<Vec<tokio::task::JoinHandle<()>>>,
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
        let mut background_tasks = Vec::new();
        let event_bus = Arc::new(EventBus::default());
        let library_service = Arc::new(LibraryService::new(db_pool.clone(), event_bus.clone()));
        let playback_service = PlaybackService::new(
            backend,
            library_service.track_repo(),
            event_bus.clone(),
            Some(db_pool.clone()),
        );

        let current_user = Arc::new(RwLock::new(None));
        playback_service.set_recommendation_user(current_user.clone());
        let session_state = Arc::new(RwLock::new(AuthSessionState::SignedOut));

        let history_repo = Arc::new(SqliteHistoryRepository::new(db_pool.clone()));
        let stats_repo = Arc::new(SqliteStatsRepository::new(db_pool.clone()));

        let history_service = HistoryService::new_with_pool(
            history_repo.clone(),
            stats_repo.clone(),
            config.history.clone(),
            event_bus.clone(),
            Some(current_user.clone()),
            Some(db_pool.clone()),
        );

        let hs_recover = history_service.clone();
        background_tasks.push(tokio::spawn(async move {
            hs_recover.recover_interrupted_sessions().await;
        }));

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
        ));
        let gap_enrichment = Arc::new(crate::providers::enrichment::GapEnrichmentService::new(
            db_pool.clone(),
            provider_coordinator.clone(),
            config.cache_dir.clone(),
            event_bus.clone(),
        ));
        background_tasks.push(tokio::spawn(gap_enrichment.clone().run()));
        let cover_art_cache = Arc::new(tokio::sync::RwLock::new(HashMap::new()));
        let cover_cache_updates = cover_art_cache.clone();
        let metadata_playback = playback_service.clone();
        let mut cover_events = event_bus.subscribe();
        background_tasks.push(tokio::spawn(async move {
            loop {
                match cover_events.recv().await {
                    Ok(Event::LibraryMetadataUpdated | Event::LibraryScanCompleted { .. }) => {
                        cover_cache_updates.write().await.clear();
                        metadata_playback.refresh_queue_metadata().await;
                    }
                    Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => (),
                    Err(_) => break,
                }
            }
        }));
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
            Arc::new(JioSaavnDownloadProvider::new(
                provider_coordinator.jiosaavn.clone(),
            ))
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

        let stream_manager = Arc::new(crate::playback::stream::StreamPlaybackManager::new(
            config.cache_dir.clone(),
            Some(db_pool.clone()),
            Some(download_service.clone()),
        ));
        let online_catalog = stream_manager.catalog.as_ref().unwrap().clone();
        online_catalog.set_event_bus(event_bus.clone());
        playback_service.set_stream_manager(stream_manager.clone());
        let sm_maint = stream_manager.clone();
        background_tasks.push(tokio::spawn(async move {
            sm_maint.startup_maintenance().await;
        }));

        let flow_engine = Arc::new(crate::recommendations::FlowEngine::new(db_pool.clone()));
        playback_service.set_flow_engine(flow_engine.clone());
        playback_service.set_starter_preparation(discovery_coordinator.clone());

        let user_repo = Arc::new(SqliteUserRepository::new(db_pool.clone()));
        let cloud_client = Arc::new(CloudClient::new());
        let settings_repo = Arc::new(SqliteSettingsRepository::new(db_pool.clone()));
        let profile_service = Arc::new(ProfileService::new(config.cache_dir.clone()));
        let app_start_date = Arc::new(RwLock::new(0));

        // Validate active cloud session on launch, sync with server, and enforce hybrid expiry
        let current_user_init = current_user.clone();
        let session_state_init = session_state.clone();
        let pool_init = db_pool.clone();
        let cloud_client_init = cloud_client.clone();
        let settings_repo_init = settings_repo.clone();
        let profile_service_init = profile_service.clone();
        let event_bus_init = event_bus.clone();
        background_tasks.push(tokio::spawn(async move {
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
                        let has_avatar = cloud_user.has_avatar.unwrap_or(false);
                        let mut avatar_data_url = profile_service_init.get_cached_avatar_data_url(&cloud_user.id);
                        if !has_avatar && profile_service_init.has_cached_avatar(&cloud_user.id) {
                            profile_service_init.remove_cached_avatar(&cloud_user.id);
                            avatar_data_url = None;
                        }

                        let local_user_meta: Option<(Option<i64>, Option<i64>)> = sqlx::query_as(
                            "SELECT avatar_version, avatar_updated_at FROM users WHERE id = ?"
                        )
                        .bind(&cloud_user.id)
                        .fetch_optional(&pool_init)
                        .await
                        .ok()
                        .flatten();

                        let needs_avatar_download = if !has_avatar {
                            false
                        } else if avatar_data_url.is_none() {
                            true
                        } else if let Some((local_ver, local_updated)) = local_user_meta {
                            (cloud_user.avatar_version.is_some() && cloud_user.avatar_version != local_ver)
                                || (cloud_user.avatar_updated_at.is_some() && cloud_user.avatar_updated_at > local_updated)
                        } else {
                            false
                        };

                        if needs_avatar_download {
                            if let Ok(Some(bytes)) = cloud_client_init.download_avatar(&worker_url, Some(&tok), &cloud_user.id).await {
                                let _ = profile_service_init.save_cached_avatar(&cloud_user.id, &bytes);
                                avatar_data_url = profile_service_init.get_cached_avatar_data_url(&cloud_user.id);
                            }
                        }

                        let avatar_key = if has_avatar || avatar_data_url.is_some() {
                            Some(format!("avatars/{}.webp", cloud_user.id))
                        } else {
                            None
                        };

                        let _ = sqlx::query("DELETE FROM users WHERE username = ? OR id = ?")
                            .bind(&cloud_user.username)
                            .bind(&cloud_user.id)
                            .execute(&pool_init)
                            .await;
                        let _ = sqlx::query(
                            "INSERT INTO users (id, username, password_hash, created_at, display_name, avatar_key, avatar_public_id, avatar_url, avatar_version, avatar_updated_at)
                             VALUES (?, ?, '', ?, ?, ?, ?, ?, ?, ?)
                             ON CONFLICT(id) DO UPDATE SET
                                 username = excluded.username,
                                 display_name = excluded.display_name,
                                 avatar_key = excluded.avatar_key,
                                 avatar_public_id = excluded.avatar_public_id,
                                 avatar_url = excluded.avatar_url,
                                 avatar_version = excluded.avatar_version,
                                 avatar_updated_at = excluded.avatar_updated_at"
                        )
                        .bind(&cloud_user.id)
                        .bind(&cloud_user.username)
                        .bind(cloud_user.created_at)
                        .bind(&cloud_user.display_name)
                        .bind(avatar_key.as_deref())
                        .bind(cloud_user.avatar_public_id.as_deref())
                        .bind(cloud_user.avatar_url.as_deref())
                        .bind(cloud_user.avatar_version)
                        .bind(cloud_user.avatar_updated_at)
                        .execute(&pool_init)
                        .await;

                        let profile = UserProfile::new(
                            cloud_user.id.clone(),
                            cloud_user.username.clone(),
                            cloud_user.created_at,
                        ).with_avatar(
                            cloud_user.display_name.clone(),
                            avatar_key,
                            cloud_user.avatar_public_id.clone(),
                            cloud_user.avatar_url.clone(),
                            cloud_user.avatar_version,
                            cloud_user.avatar_updated_at,
                            avatar_data_url,
                        );

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
                            &tok,
                            &cloud_user.id,
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
                            let cached_avatar = profile_service_init.get_cached_avatar_data_url(&session.user_id);
                            let cloud_user = CloudUser {
                                id: session.user_id.clone(),
                                username: session.username.clone(),
                                created_at: session.created_at,
                                display_name: None,
                                has_avatar: Some(cached_avatar.is_some()),
                                avatar_public_id: None,
                                avatar_url: None,
                                avatar_version: None,
                                avatar_key: if cached_avatar.is_some() { Some(format!("avatars/{}.webp", session.user_id)) } else { None },
                                avatar_updated_at: None,
                            };
                            let profile = UserProfile::new(
                                session.user_id.clone(),
                                session.username.clone(),
                                session.created_at,
                            ).with_avatar(
                                None,
                                if cached_avatar.is_some() { Some(format!("avatars/{}.webp", session.user_id)) } else { None },
                                None,
                                None,
                                None,
                                None,
                                cached_avatar,
                            );
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
        }));

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
            profile_service,
            current_user,
            session_state,
            app_start_date,
            taste_engine,
            recommender,
            smart_mix_generator,
            provider_coordinator,
            gap_enrichment,
            wishlist_manager,
            discovery_coordinator,
            download_service,
            flow_engine,
            online_catalog,
            config: Arc::new(RwLock::new(config)),
            cover_art_cache,
            is_quitting: AtomicBool::new(false),
            close_button_quits: AtomicBool::new(false),
            close_behavior_ready: OnceCell::new(),
            close_behavior_updates: Mutex::new(()),
            command_gate: RwLock::new(()),
            shutdown_done: Mutex::new(false),
            background_tasks: StdMutex::new(background_tasks),
        }
    }

    async fn get_or_init_app_start_date(&self) -> i64 {
        let mut guard = self.app_start_date.write().await;
        if *guard > 0 {
            return *guard;
        }
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT value FROM application_settings WHERE key = 'app_first_start_date'",
        )
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
            .unwrap_or_else(|| {
                "https://soundflow-cloud-worker.abhi-atlas-2026.workers.dev".to_string()
            })
    }

    pub fn is_quitting(&self) -> bool {
        self.is_quitting.load(Ordering::Acquire)
    }

    /// Returns true only for the first request; quitting cannot be undone.
    pub fn begin_quit(&self) -> bool {
        let first = !self.is_quitting.swap(true, Ordering::AcqRel);
        self.playback_service.request_shutdown();
        first
    }

    pub fn close_button_behavior(&self) -> &'static str {
        if self.close_button_quits.load(Ordering::Acquire) {
            "quit_app"
        } else {
            "minimize_to_tray"
        }
    }

    /// Awaited at startup and by settings IPC. A late startup read cannot
    /// overwrite a newer setting. Missing or invalid DB values mean hide-to-tray.
    pub async fn initialize_close_behavior(&self) -> AppResult<()> {
        self.close_behavior_ready
            .get_or_try_init(|| async {
                let saved = self
                    .settings_repo
                    .get_setting("close_button_behavior")
                    .await?;
                self.close_button_quits
                    .store(saved.as_deref() == Some("quit_app"), Ordering::Release);
                Ok::<(), AppError>(())
            })
            .await?;
        Ok(())
    }

    pub async fn set_close_button_behavior(&self, behavior: String) -> AppResult<()> {
        if behavior != "minimize_to_tray" && behavior != "quit_app" {
            return Err(AppError::Validation(format!(
                "Invalid close_button_behavior: '{}'. Allowed: 'minimize_to_tray', 'quit_app'",
                behavior
            )));
        }
        self.initialize_close_behavior().await?;
        let _update = self.close_behavior_updates.lock().await;
        self.settings_repo
            .set_setting("close_button_behavior", &behavior)
            .await?;
        self.close_button_quits
            .store(behavior == "quit_app", Ordering::Release);
        self.config.write().await.close_button_behavior = behavior;
        Ok(())
    }

    /// Quiesce commands, stop audio once, await history writes, then close SQLite.
    pub async fn shutdown(&self) -> AppResult<()> {
        let mut done = self.shutdown_done.lock().await;
        if *done {
            return Ok(());
        }
        self.begin_quit();
        let _commands = self.command_gate.write().await;
        let tasks = std::mem::take(
            &mut *self
                .background_tasks
                .lock()
                .unwrap_or_else(|e| e.into_inner()),
        );
        for task in &tasks {
            task.abort();
        }
        for task in tasks {
            let _ = task.await;
        }
        self.history_service.begin_shutdown();
        let stopped = self.playback_service.shutdown().await;
        self.history_service.shutdown().await;
        self.db_pool.close().await;
        *done = true;
        stopped
    }

    pub async fn current_user_id(&self) -> String {
        let current_user_guard = self.current_user.read().await;
        current_user_guard
            .as_ref()
            .map(|u| u.id.clone())
            .unwrap_or_else(|| "default".to_string())
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
                    if let Ok(payload) =
                        SyncManager::prepare_local_sync_payload(&pool_clone, &uid).await
                    {
                        if let Ok(synced_at) =
                            client_clone.push_sync(&worker_url, &token, &payload).await
                        {
                            let _ = SyncManager::clear_tombstones(&pool_clone, &uid).await;
                            let _ =
                                SyncManager::update_session_synced_at(&pool_clone, &uid, synced_at)
                                    .await;
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

    /// Access the FlowEngine handle.
    pub fn flow_engine(&self) -> Arc<crate::recommendations::FlowEngine> {
        self.flow_engine.clone()
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
        let _commands = self.command_gate.read().await;
        if self.is_quitting() {
            return Err(AppError::Validation("Kaze is shutting down".into()));
        }
        self.playback_service.initialize_saved_playback().await;
        info!(?cmd, "Dispatching command");

        match cmd {
            // --- Library & Onboarding Management ---
            Command::SaveRecommendationSetup {
                expected_user_id,
                artists,
                genres,
                skipped,
            } => {
                let user = self.current_user_id().await;
                if expected_user_id
                    .as_deref()
                    .is_some_and(|expected| expected != user)
                {
                    return Err(AppError::Validation(
                        "Account changed. Reopen music taste and try again.".into(),
                    ));
                }
                crate::recommendations::setup::save(&self.db_pool, &user, artists, genres, skipped)
                    .await?;
                self.flow_engine.invalidate_home_picks(&user).await;
                sqlx::query("UPDATE playlists SET expires_at=0 WHERE user_id=? AND is_smart_mix=1")
                    .bind(&user)
                    .execute(&self.db_pool)
                    .await?;
                let _ = self
                    .event_bus
                    .publish(Event::TasteProfileChanged { user_id: user });
                Ok(CommandResponse::Ok)
            }
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

            // --- Playback Controls ---
            Command::PlayTrack { track_id, source } => {
                self.playback_service.play_track(&track_id, source).await?;
                Ok(CommandResponse::Ok)
            }
            Command::PlayOnlineTrack {
                track_id,
                title,
                artist,
                album,
                duration_secs,
                cover_art_url,
                preview_url,
                source,
            } => {
                self.playback_service
                    .play_online_track(
                        &track_id,
                        &title,
                        &artist,
                        album.as_deref(),
                        duration_secs,
                        cover_art_url.as_deref(),
                        preview_url.as_deref(),
                        source,
                    )
                    .await?;
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
            Command::TogglePlayPause => {
                let state = self.playback_service.get_playback_state().await;
                if state.is_playing {
                    self.playback_service.pause().await?;
                } else if state.is_paused {
                    self.playback_service.resume().await?;
                } else if state.queue_length > 0 {
                    let idx = state.current_queue_index.unwrap_or(0);
                    self.playback_service.play_queue_index(idx).await?;
                } else {
                    return Err(AppError::Playback("Select a song to play".into()));
                }
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
                    return Err(AppError::Validation(
                        "Volume must be between 0.0 and 1.0".into(),
                    ));
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
            Command::EnqueueTrack {
                track_id,
                play_next,
            } => {
                if self
                    .playback_service
                    .enqueue_track(&track_id, play_next)
                    .await?
                {
                    Ok(CommandResponse::Ok)
                } else {
                    Err(AppError::Validation("Track is already in the queue".into()))
                }
            }
            Command::EnqueueOnlineTrack {
                track_id,
                title,
                artist,
                album,
                duration_secs,
                cover_art_url,
                preview_url,
                play_next,
            } => {
                crate::recommendations::mixes::ensure_online_track(
                    &self.db_pool,
                    &track_id,
                    &title,
                    Some(&artist),
                    album.as_deref(),
                    duration_secs,
                    cover_art_url.as_deref(),
                    preview_url.as_deref(),
                )
                .await?;
                if self
                    .playback_service
                    .enqueue_track(&track_id, play_next)
                    .await?
                {
                    Ok(CommandResponse::Ok)
                } else {
                    Err(AppError::Validation("Track is already in the queue".into()))
                }
            }
            Command::DequeueTrack { track_id } => {
                self.playback_service.dequeue_track(&track_id).await;
                Ok(CommandResponse::Ok)
            }
            Command::ClearQueue => {
                self.playback_service.clear_queue().await;
                Ok(CommandResponse::Ok)
            }
            Command::ClearRemoteAudioCache => {
                let (bytes_freed, files_removed) =
                    self.playback_service.clear_remote_audio_cache().await?;
                Ok(CommandResponse::RemoteAudioCacheCleared {
                    bytes_freed,
                    files_removed,
                })
            }

            Command::StartFlow { familiarity_ratio } => {
                let user_id = self.current_user_id().await;
                self.playback_service
                    .start_flow(&self.flow_engine, &user_id, familiarity_ratio)
                    .await?;
                Ok(CommandResponse::Ok)
            }
            Command::ContinueFlow => {
                let user_id = self.current_user_id().await;
                self.playback_service.continue_flow(&user_id).await?;
                Ok(CommandResponse::Ok)
            }
            Command::StopFlow => {
                self.playback_service.stop_flow().await;
                Ok(CommandResponse::Ok)
            }
            Command::SetFlowSteering { familiarity_ratio } => {
                let user_id = self.current_user_id().await;
                self.playback_service
                    .set_flow_steering(&self.flow_engine, &user_id, familiarity_ratio)
                    .await?;
                Ok(CommandResponse::Ok)
            }

            // --- User Feedback & Taste ---
            Command::LikeTrack { track_id } => {
                self.set_song_feedback(&track_id, 1).await?;
                Ok(CommandResponse::Ok)
            }
            Command::DislikeTrack { track_id } => {
                self.set_song_feedback(&track_id, -1).await?;
                Ok(CommandResponse::Ok)
            }
            Command::RemoveTrackFeedback { track_id } => {
                self.set_song_feedback(&track_id, 0).await?;
                Ok(CommandResponse::Ok)
            }
            Command::EnsureLikedSongsPlaylist => {
                let user_id = self.current_user_id().await;
                let id = self
                    .playlist_repo
                    .ensure_liked_songs_playlist(&user_id)
                    .await?;
                Ok(CommandResponse::EntityId(id))
            }

            // --- Playlists & Recommendations ---
            Command::GenerateSmartMix { mix_type } => {
                let user_id = self.current_user_id().await;
                let playlist = self
                    .smart_mix_generator
                    .generate_mix(&user_id, &mix_type)
                    .await?;
                let tracks = self.playlist_repo.get_playlist_tracks(&playlist.id).await?;
                let track_count = tracks.len();
                let mix_type_name = playlist
                    .mix_type
                    .clone()
                    .unwrap_or_else(|| "custom".to_string());

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
                let user_id = current_user_guard
                    .as_ref()
                    .map(|u| u.id.clone())
                    .unwrap_or_else(|| "default".to_string());
                drop(current_user_guard);

                if let Some(_) = self
                    .playlist_repo
                    .find_by_name_and_user(&trimmed_name, &user_id)
                    .await?
                {
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
                self.playlist_repo
                    .create_playlist_with_user(&record, &user_id)
                    .await?;

                // Background sync if user is logged in
                self.trigger_background_sync().await;

                Ok(CommandResponse::EntityId(id))
            }
            Command::DeletePlaylist { playlist_id } => {
                let user_id = {
                    let current_user_guard = self.current_user.read().await;
                    current_user_guard.as_ref().map(|u| u.id.clone())
                };
                let Some(ref uid) = user_id else {
                    return Err(AppError::Validation("Sign in to delete playlists.".into()));
                };
                let owned: Option<(String,)> = sqlx::query_as(
                    "SELECT name FROM playlists WHERE id = ? AND user_id = ? AND is_smart_mix = 0",
                )
                .bind(&playlist_id)
                .bind(uid)
                .fetch_optional(&self.db_pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
                let Some((playlist_name,)) = owned else {
                    return Err(AppError::Validation(
                        "Playlist not found or not owned by this account.".into(),
                    ));
                };
                if playlist_name == "Liked Songs" {
                    return Err(AppError::Validation(
                        "The Liked Songs playlist cannot be deleted.".into(),
                    ));
                }
                if let Some(ref uid) = user_id {
                    let _ =
                        SyncManager::record_tombstone(&self.db_pool, uid, "playlist", &playlist_id)
                            .await;
                }
                self.playlist_repo.delete_playlist(&playlist_id).await?;
                self.trigger_background_sync().await;
                Ok(CommandResponse::Ok)
            }
            Command::RenamePlaylist { playlist_id, name } => {
                let trimmed = name.trim().to_string();
                if trimmed.is_empty() {
                    return Err(AppError::Validation("Playlist name cannot be empty".into()));
                }
                let user_id = {
                    let current_user_guard = self.current_user.read().await;
                    current_user_guard.as_ref().map(|u| u.id.clone())
                };
                let Some(uid) = user_id else {
                    return Err(AppError::Validation("Sign in to rename playlists.".into()));
                };
                let owned: Option<(String,)> = sqlx::query_as(
                    "SELECT name FROM playlists WHERE id = ? AND user_id = ? AND is_smart_mix = 0",
                )
                .bind(&playlist_id)
                .bind(&uid)
                .fetch_optional(&self.db_pool)
                .await
                .map_err(|e| AppError::Database(e.to_string()))?;
                let Some((playlist_name,)) = owned else {
                    return Err(AppError::Validation(
                        "Playlist not found or not owned by this account.".into(),
                    ));
                };
                if playlist_name == "Liked Songs" {
                    return Err(AppError::Validation(
                        "The Liked Songs playlist cannot be renamed.".into(),
                    ));
                }
                if let Some(existing) = self
                    .playlist_repo
                    .find_by_name_and_user(&trimmed, &uid)
                    .await?
                {
                    if existing.id != playlist_id {
                        return Err(AppError::Validation(format!(
                            "A playlist named '{}' already exists",
                            trimmed
                        )));
                    }
                }
                self.playlist_repo
                    .rename_playlist(&playlist_id, &trimmed)
                    .await?;
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
                let track_opt = self
                    .library_service
                    .track_repo()
                    .find_by_id(&track_id)
                    .await?;
                let needs_online_ensure = match &track_opt {
                    None => true,
                    Some(t) => {
                        t.title.starts_with("itunes:")
                            || t.title.starts_with("online:")
                            || t.format == "online"
                    }
                };

                if needs_online_ensure {
                    let is_clean_title = match &title {
                        Some(t) => {
                            !t.trim().is_empty()
                                && !t.starts_with("itunes:")
                                && !t.starts_with("online:")
                        }
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
                            (
                                title.unwrap_or_else(|| track_id.clone()),
                                artist,
                                album,
                                duration_secs,
                                cover_art_url,
                                preview_url,
                            )
                        }
                    };

                    let _ = crate::recommendations::mixes::ensure_online_track(
                        &self.db_pool,
                        &track_id,
                        if t_title.is_empty() {
                            &track_id
                        } else {
                            &t_title
                        },
                        t_artist.as_deref(),
                        t_album.as_deref(),
                        t_dur,
                        t_cover.as_deref(),
                        t_prev.as_deref(),
                    )
                    .await;
                }
                self.playlist_repo
                    .add_track(&playlist_id, &track_id, None)
                    .await?;
                self.trigger_background_sync().await;
                Ok(CommandResponse::Ok)
            }
            Command::RemoveTrackFromPlaylist {
                playlist_id,
                track_id,
            } => {
                let user_id = {
                    let current_user_guard = self.current_user.read().await;
                    current_user_guard.as_ref().map(|u| u.id.clone())
                };
                if let Some(ref uid) = user_id {
                    let entity_id = format!("{}:{}", playlist_id, track_id);
                    let _ = SyncManager::record_tombstone(
                        &self.db_pool,
                        uid,
                        "playlist_song",
                        &entity_id,
                    )
                    .await;
                }
                self.playlist_repo
                    .remove_track(&playlist_id, &track_id)
                    .await?;
                self.trigger_background_sync().await;
                Ok(CommandResponse::Ok)
            }

            // --- Metadata Providers ---
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
                if !matches!(key.as_str(), "enable_jiosaavn" | "auto_fill_metadata") {
                    return Err(AppError::Validation("Unknown metadata preference".into()));
                }
                self.settings_repo
                    .set_setting(&key, if enabled { "true" } else { "false" })
                    .await?;
                if enabled && key == "auto_fill_metadata" {
                    self.gap_enrichment.request_fill(false).await?;
                }
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

            // --- Downloads ---
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

            // --- User Authentication & Profiles ---
            Command::SignUp { username, password } => {
                let trimmed_user = username.trim().to_string();
                if trimmed_user.is_empty() {
                    return Err(AppError::Validation("Username cannot be empty".to_string()));
                }
                if trimmed_user.len() < 3 || trimmed_user.len() > 50 {
                    return Err(AppError::Validation(
                        "Username must be between 3 and 50 characters".to_string(),
                    ));
                }
                if !trimmed_user
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
                {
                    return Err(AppError::Validation("Username can only contain alphanumeric characters, underscores, hyphens, and dots".to_string()));
                }
                if password.len() < 8 || password.len() > 128 {
                    return Err(AppError::Validation(
                        "Password must be between 8 and 128 characters".to_string(),
                    ));
                }

                let worker_url = self.get_cloud_worker_url().await;
                let device_id = device::get_or_create_device_id(&self.db_pool).await?;
                let device_name = device::get_device_name();
                let client_ver = device::get_client_version();

                let auth_res = self
                    .cloud_client
                    .register(
                        &worker_url,
                        &trimmed_user,
                        &password,
                        Some(&device_id),
                        Some(&device_name),
                        Some(&client_ver),
                    )
                    .await?;

                if let (Some(cloud_user), Some(token)) = (auth_res.user, auth_res.token) {
                    let has_avatar = cloud_user.has_avatar.unwrap_or(false);
                    let avatar_data_url = self
                        .profile_service
                        .get_cached_avatar_data_url(&cloud_user.id);
                    let avatar_key = if has_avatar {
                        Some(format!("avatars/{}.webp", cloud_user.id))
                    } else {
                        None
                    };

                    let profile = UserProfile::new(
                        cloud_user.id.clone(),
                        cloud_user.username.clone(),
                        cloud_user.created_at,
                    )
                    .with_avatar(
                        cloud_user.display_name.clone(),
                        avatar_key,
                        cloud_user.avatar_public_id.clone(),
                        cloud_user.avatar_url.clone(),
                        cloud_user.avatar_version,
                        cloud_user.avatar_updated_at,
                        avatar_data_url,
                    );

                    let now = chrono::Utc::now().timestamp();
                    let session_meta = auth_res.session;
                    let session_id = session_meta
                        .as_ref()
                        .map(|s| s.id.clone())
                        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                    let idle_expires_at = session_meta
                        .as_ref()
                        .map(|s| s.idle_expires_at)
                        .unwrap_or(now + 30 * 86400);
                    let absolute_expires_at = session_meta
                        .as_ref()
                        .map(|s| s.absolute_expires_at)
                        .unwrap_or(now + 90 * 86400);

                    // Upsert local user profile cleanly
                    let _ = sqlx::query("DELETE FROM users WHERE username = ? OR id = ?")
                        .bind(&profile.username)
                        .bind(&profile.id)
                        .execute(&self.db_pool)
                        .await;
                    let _ = sqlx::query(
                        "INSERT INTO users (id, username, password_hash, created_at, display_name, avatar_key, avatar_public_id, avatar_url, avatar_version, avatar_updated_at)
                         VALUES (?, ?, '', ?, ?, ?, ?, ?, ?, ?)
                         ON CONFLICT(id) DO UPDATE SET
                             username = excluded.username,
                             display_name = excluded.display_name,
                             avatar_key = excluded.avatar_key,
                             avatar_public_id = excluded.avatar_public_id,
                             avatar_url = excluded.avatar_url,
                             avatar_version = excluded.avatar_version,
                             avatar_updated_at = excluded.avatar_updated_at"
                    )
                    .bind(&profile.id)
                    .bind(&profile.username)
                    .bind(profile.created_at)
                    .bind(&profile.display_name)
                    .bind(profile.avatar_key.as_deref())
                    .bind(profile.avatar_public_id.as_deref())
                    .bind(profile.avatar_url.as_deref())
                    .bind(profile.avatar_version)
                    .bind(profile.avatar_updated_at)
                    .execute(&self.db_pool)
                    .await;

                    // Store raw bearer token securely in OS keyring (fallback to private local file)
                    if let Err(e) = credentials::store_session_token(&profile.id, &token) {
                        tracing::warn!(
                            "Failed to store session token in secure credentials: {}",
                            e
                        );
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
                    let _ = self
                        .playlist_repo
                        .reassign_orphan_playlists_to_user(&profile.id)
                        .await;

                    // Guest listening retains its original owner on registration.
                    self.playback_service.stop_flow().await;
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
                        )
                        .await
                        {
                            let _ = event_bus_clone.publish(Event::PlaylistsUpdated);
                        }
                    });

                    let val = serde_json::to_value(&profile).unwrap_or_default();
                    let _ = self.event_bus.publish(Event::SessionChanged {
                        user: Some(val.clone()),
                    });
                    return Ok(CommandResponse::UserProfile(val));
                }
                Err(AppError::Validation(
                    "Cloud registration did not return user info".to_string(),
                ))
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

                let auth_res = self
                    .cloud_client
                    .login(
                        &worker_url,
                        &trimmed_user,
                        &password,
                        Some(&device_id),
                        Some(&device_name),
                        Some(&client_ver),
                    )
                    .await?;

                if let (Some(cloud_user), Some(token)) = (auth_res.user, auth_res.token) {
                    let has_avatar = cloud_user.has_avatar.unwrap_or(false);
                    let mut avatar_data_url = self
                        .profile_service
                        .get_cached_avatar_data_url(&cloud_user.id);
                    if !has_avatar && self.profile_service.has_cached_avatar(&cloud_user.id) {
                        self.profile_service.remove_cached_avatar(&cloud_user.id);
                        avatar_data_url = None;
                    }

                    let local_user_meta: Option<(Option<i64>, Option<i64>)> = sqlx::query_as(
                        "SELECT avatar_version, avatar_updated_at FROM users WHERE id = ?",
                    )
                    .bind(&cloud_user.id)
                    .fetch_optional(&self.db_pool)
                    .await
                    .ok()
                    .flatten();

                    let needs_avatar_download = if !has_avatar {
                        false
                    } else if avatar_data_url.is_none() {
                        true
                    } else if let Some((local_ver, local_updated)) = local_user_meta {
                        (cloud_user.avatar_version.is_some()
                            && cloud_user.avatar_version != local_ver)
                            || (cloud_user.avatar_updated_at.is_some()
                                && cloud_user.avatar_updated_at > local_updated)
                    } else {
                        false
                    };

                    if needs_avatar_download {
                        if let Ok(Some(bytes)) = self
                            .cloud_client
                            .download_avatar(&worker_url, Some(&token), &cloud_user.id)
                            .await
                        {
                            let _ = self
                                .profile_service
                                .save_cached_avatar(&cloud_user.id, &bytes);
                            avatar_data_url = self
                                .profile_service
                                .get_cached_avatar_data_url(&cloud_user.id);
                        }
                    }

                    let avatar_key = if has_avatar || avatar_data_url.is_some() {
                        Some(format!("avatars/{}.webp", cloud_user.id))
                    } else {
                        None
                    };

                    let profile = UserProfile::new(
                        cloud_user.id.clone(),
                        cloud_user.username.clone(),
                        cloud_user.created_at,
                    )
                    .with_avatar(
                        cloud_user.display_name.clone(),
                        avatar_key,
                        cloud_user.avatar_public_id.clone(),
                        cloud_user.avatar_url.clone(),
                        cloud_user.avatar_version,
                        cloud_user.avatar_updated_at,
                        avatar_data_url,
                    );

                    let now = chrono::Utc::now().timestamp();
                    let session_meta = auth_res.session;
                    let session_id = session_meta
                        .as_ref()
                        .map(|s| s.id.clone())
                        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
                    let idle_expires_at = session_meta
                        .as_ref()
                        .map(|s| s.idle_expires_at)
                        .unwrap_or(now + 30 * 86400);
                    let absolute_expires_at = session_meta
                        .as_ref()
                        .map(|s| s.absolute_expires_at)
                        .unwrap_or(now + 90 * 86400);

                    // Upsert local user profile (without password hash)
                    let _ = sqlx::query("DELETE FROM users WHERE username = ? OR id = ?")
                        .bind(&profile.username)
                        .bind(&profile.id)
                        .execute(&self.db_pool)
                        .await;
                    let _ = sqlx::query(
                        "INSERT INTO users (id, username, password_hash, created_at, display_name, avatar_key, avatar_public_id, avatar_url, avatar_version, avatar_updated_at)
                         VALUES (?, ?, '', ?, ?, ?, ?, ?, ?, ?)
                         ON CONFLICT(id) DO UPDATE SET
                             username = excluded.username,
                             display_name = excluded.display_name,
                             avatar_key = excluded.avatar_key,
                             avatar_public_id = excluded.avatar_public_id,
                             avatar_url = excluded.avatar_url,
                             avatar_version = excluded.avatar_version,
                             avatar_updated_at = excluded.avatar_updated_at"
                    )
                    .bind(&profile.id)
                    .bind(&profile.username)
                    .bind(profile.created_at)
                    .bind(&profile.display_name)
                    .bind(profile.avatar_key.as_deref())
                    .bind(profile.avatar_public_id.as_deref())
                    .bind(profile.avatar_url.as_deref())
                    .bind(profile.avatar_version)
                    .bind(profile.avatar_updated_at)
                    .execute(&self.db_pool)
                    .await;

                    // Store raw bearer token securely in OS keyring (fallback to private local file)
                    if let Err(e) = credentials::store_session_token(&profile.id, &token) {
                        tracing::warn!(
                            "Failed to store session token in secure credentials: {}",
                            e
                        );
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
                    let _ = self
                        .playlist_repo
                        .reassign_orphan_playlists_to_user(&profile.id)
                        .await;

                    self.playback_service.stop_flow().await;
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
                        )
                        .await
                        {
                            let _ = event_bus_clone.publish(Event::PlaylistsUpdated);
                        }
                    });

                    let val = serde_json::to_value(&profile).unwrap_or_default();
                    let _ = self.event_bus.publish(Event::SessionChanged {
                        user: Some(val.clone()),
                    });
                    return Ok(CommandResponse::UserProfile(val));
                }
                Err(AppError::Validation(
                    "Cloud login did not return user info".to_string(),
                ))
            }
            Command::Logout => {
                let worker_url = self.get_cloud_worker_url().await;
                if let Ok(Some(session)) = SyncManager::get_active_session(&self.db_pool).await {
                    let client = self.cloud_client.clone();
                    let user_id = session.user_id.clone();
                    let token = credentials::get_session_token(&user_id);
                    if let Some(tok) = token {
                        let _ = client.logout(&worker_url, &tok).await;
                    }
                    let _ = SyncManager::delete_session(&self.db_pool, &user_id).await;
                    let _ = credentials::delete_session_token(&user_id);
                }
                self.playback_service.stop_flow().await;
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
                let session = SyncManager::get_active_session(&self.db_pool)
                    .await?
                    .ok_or_else(|| {
                        AppError::Validation(
                            "No active session. Sign in again to log out all devices.".to_string(),
                        )
                    })?;
                let user_id = session.user_id.clone();
                let token = credentials::get_session_token(&user_id).ok_or_else(|| {
                    AppError::Validation(
                        "Session credentials unavailable. Sign in again to log out all devices."
                            .to_string(),
                    )
                })?;
                // Require server confirmation before clearing this device. Otherwise
                // an offline/failed request would falsely report global revocation.
                self.cloud_client.logout_all(&worker_url, &token).await?;
                let _ = SyncManager::delete_session(&self.db_pool, &user_id).await;
                let _ = credentials::delete_session_token(&user_id);
                self.playback_service.stop_flow().await;
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
                let active = SyncManager::get_active_session(&self.db_pool)
                    .await
                    .ok()
                    .flatten();
                if let Some(session) = active {
                    let token =
                        credentials::get_session_token(&session.user_id).ok_or_else(|| {
                            AppError::Validation("No active session token".to_string())
                        })?;
                    self.cloud_client
                        .revoke_session(&worker_url, &token, &session_id)
                        .await?;
                    if session.session_id == session_id {
                        let _ = SyncManager::delete_session(&self.db_pool, &session.user_id).await;
                        self.playback_service.stop_flow().await;
                        *self.current_user.write().await = None;
                        *self.session_state.write().await = AuthSessionState::SignedOut;
                    }
                }
                Ok(CommandResponse::Ok)
            }
            Command::UploadAvatar { file_path } => {
                let worker_url = self.get_cloud_worker_url().await;
                let session = SyncManager::get_active_session(&self.db_pool)
                    .await?
                    .ok_or_else(|| {
                        AppError::Validation(
                            "You must be logged in to upload a profile photo".to_string(),
                        )
                    })?;
                let token = credentials::get_session_token(&session.user_id).ok_or_else(|| {
                    AppError::Validation(
                        "Session credentials not found locally. Please log in again.".to_string(),
                    )
                })?;

                let path = match file_path {
                    Some(p) if !p.trim().is_empty() => std::path::PathBuf::from(p),
                    _ => match ProfileService::pick_avatar_file()? {
                        Some(p) => p,
                        None => return Ok(CommandResponse::Ok),
                    },
                };

                let raw_bytes = std::fs::read(&path)
                    .map_err(|e| AppError::Io(format!("Failed to read selected image: {}", e)))?;
                let normalized_webp = ProfileService::normalize_avatar_image(&raw_bytes)?;

                // Save to local cache immediately so the user has their photo locally regardless of cloud R2 status
                let _ = self
                    .profile_service
                    .save_cached_avatar(&session.user_id, &normalized_webp);
                let data_url = self
                    .profile_service
                    .get_cached_avatar_data_url(&session.user_id);
                let now = chrono::Utc::now().timestamp();
                let local_avatar_key = format!("avatars/{}.webp", session.user_id);

                // Attempt cloud upload to Cloudinary (via Worker), gracefully fall back to local if offline
                let (avatar_key, avatar_public_id, avatar_url, avatar_version, avatar_updated_at) =
                    match self
                        .cloud_client
                        .upload_avatar(&worker_url, &token, normalized_webp.clone())
                        .await
                    {
                        Ok(resp) => (
                            resp.avatar_key,
                            resp.avatar_public_id,
                            resp.avatar_url,
                            resp.avatar_version,
                            resp.avatar_updated_at,
                        ),
                        Err(e) => {
                            tracing::warn!("Cloud avatar upload deferred / unavailable: {}", e);
                            (Some(local_avatar_key), None, None, None, Some(now))
                        }
                    };

                let _ = self
                    .user_repo
                    .update_avatar_metadata(
                        &session.user_id,
                        avatar_key.as_deref(),
                        avatar_public_id.as_deref(),
                        avatar_url.as_deref(),
                        avatar_version,
                        avatar_updated_at,
                    )
                    .await;

                let updated_profile = {
                    let mut guard = self.current_user.write().await;
                    if let Some(ref mut u) = *guard {
                        u.avatar_key = avatar_key.clone();
                        u.avatar_public_id = avatar_public_id.clone();
                        u.avatar_url = avatar_url.clone();
                        u.avatar_version = avatar_version;
                        u.avatar_updated_at = avatar_updated_at;
                        u.avatar_data_url = data_url.clone();
                    }
                    guard.clone()
                };

                let val = serde_json::to_value(&updated_profile).unwrap_or_default();
                let _ = self
                    .event_bus
                    .publish(Event::UserProfileUpdated { user: val.clone() });
                let _ = self.event_bus.publish(Event::SessionChanged {
                    user: Some(val.clone()),
                });

                Ok(CommandResponse::UserProfile(val))
            }
            Command::RemoveAvatar => {
                let worker_url = self.get_cloud_worker_url().await;
                let session = SyncManager::get_active_session(&self.db_pool)
                    .await?
                    .ok_or_else(|| {
                        AppError::Validation(
                            "You must be logged in to remove your profile photo".to_string(),
                        )
                    })?;
                let token = credentials::get_session_token(&session.user_id).ok_or_else(|| {
                    AppError::Validation(
                        "Session credentials not found locally. Please log in again.".to_string(),
                    )
                })?;

                // Attempt cloud delete, ignore 503/offline errors
                if let Err(e) = self.cloud_client.delete_avatar(&worker_url, &token).await {
                    tracing::warn!("Cloud avatar delete deferred / unavailable: {}", e);
                }

                self.profile_service.remove_cached_avatar(&session.user_id);
                let _ = self
                    .user_repo
                    .update_avatar_metadata(&session.user_id, None, None, None, None, None)
                    .await;

                let updated_profile = {
                    let mut guard = self.current_user.write().await;
                    if let Some(ref mut u) = *guard {
                        u.avatar_key = None;
                        u.avatar_public_id = None;
                        u.avatar_url = None;
                        u.avatar_version = None;
                        u.avatar_updated_at = None;
                        u.avatar_data_url = None;
                    }
                    guard.clone()
                };

                let val = serde_json::to_value(&updated_profile).unwrap_or_default();
                let _ = self
                    .event_bus
                    .publish(Event::UserProfileUpdated { user: val.clone() });
                let _ = self.event_bus.publish(Event::SessionChanged {
                    user: Some(val.clone()),
                });

                Ok(CommandResponse::UserProfile(val))
            }
            Command::SyncCloudData => {
                let worker_url = self.get_cloud_worker_url().await;
                let session = SyncManager::get_active_session(&self.db_pool)
                    .await?
                    .ok_or_else(|| {
                        AppError::Validation("Not logged in to a cloud account".to_string())
                    })?;

                let now = chrono::Utc::now().timestamp();
                let is_idle_expired = session.idle_expires_at > 0 && now >= session.idle_expires_at;
                let is_abs_expired =
                    session.absolute_expires_at > 0 && now >= session.absolute_expires_at;
                if is_idle_expired || is_abs_expired {
                    *self.session_state.write().await = AuthSessionState::SessionExpired {
                        reason: if is_abs_expired {
                            SessionExpiredReason::AbsoluteTimeout
                        } else {
                            SessionExpiredReason::IdleTimeout
                        },
                    };
                    return Err(AppError::Validation(
                        "Session expired. Please log in again.".to_string(),
                    ));
                }

                let token = credentials::get_session_token(&session.user_id).ok_or_else(|| {
                    AppError::Validation(
                        "Session credentials not found locally. Please log in again.".to_string(),
                    )
                })?;

                // Deterministic sync: Pull first, reconcile, push
                let report = SyncManager::sync_with_cloud(
                    &self.db_pool,
                    &self.cloud_client,
                    &worker_url,
                    &session.user_id,
                    &token,
                )
                .await?;

                SyncManager::update_validation_timestamp(&self.db_pool, &session.user_id, now)
                    .await?;
                let _ = self.event_bus.publish(Event::PlaylistsUpdated);

                Ok(CommandResponse::CloudSyncCompleted {
                    synced_at: report.synced_at,
                })
            }
            Command::SetCloudServerUrl { url } => {
                let trimmed = url.trim().to_string();
                if trimmed.is_empty() {
                    return Err(AppError::Validation(
                        "Server URL cannot be empty".to_string(),
                    ));
                }
                self.settings_repo
                    .set_setting("cloud_sync_url", &trimmed)
                    .await?;
                Ok(CommandResponse::Ok)
            }
            Command::SetCloseButtonBehavior { behavior } => {
                self.set_close_button_behavior(behavior).await?;
                Ok(CommandResponse::Ok)
            }
            Command::SetInterfaceStyle { style } => {
                if !matches!(style.as_str(), "spatial" | "brutalist") {
                    return Err(AppError::Validation("Unknown interface style".to_string()));
                }
                self.settings_repo
                    .set_setting("interface_style", &style)
                    .await?;
                Ok(CommandResponse::Ok)
            }
        }
    }

    async fn load_track_artwork(&self, track_id: &str) -> AppResult<Option<String>> {
        if let Some(cached) = self.cover_art_cache.read().await.get(track_id) {
            return Ok(cached.clone());
        }

        // Source-bound online artwork belongs to the exact displayed recording.
        // Never fall through to unrelated enrichment artwork for an online alias.
        let online_cover: Option<String> = sqlx::query_scalar(
            "SELECT COALESCE((SELECT json_extract(metadata_json,'$.cover_art_url') FROM online_track_sources WHERE track_id=?),
             (SELECT cover_art_url FROM external_tracks WHERE id=? AND provider='jiosaavn'))")
            .bind(track_id).bind(track_id).fetch_one(&self.db_pool).await?;
        if let Some(url) = online_cover.filter(|url| url.starts_with("https://")) {
            self.cover_art_cache
                .write()
                .await
                .insert(track_id.into(), Some(url.clone()));
            return Ok(Some(url));
        }
        if self.online_catalog.source_id(track_id).await?.is_some() {
            self.cover_art_cache
                .write()
                .await
                .insert(track_id.into(), None);
            return Ok(None);
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
                                .or_else(|| {
                                    tagged_file
                                        .primary_tag()
                                        .and_then(|tag| tag.pictures().first())
                                })
                                .or_else(|| {
                                    tagged_file
                                        .first_tag()
                                        .and_then(|tag| tag.pictures().first())
                                });

                            if let Some(pic) = picture {
                                let mime =
                                    pic.mime_type().map(|m| m.as_str()).unwrap_or("image/jpeg");
                                let encoded =
                                    base64::engine::general_purpose::STANDARD.encode(pic.data());
                                cover_result = Some(format!("data:{};base64,{}", mime, encoded));
                            }
                        }
                    }
                }

                // If not found in tags, check folder for cover.jpg / folder.jpg / album.jpg / cover.png
                if cover_result.is_none() {
                    if let Some(parent) = path.parent() {
                        for candidate in &[
                            "cover.jpg",
                            "folder.jpg",
                            "album.jpg",
                            "cover.png",
                            "folder.png",
                            "front.jpg",
                            "front.png",
                        ] {
                            let img_path = parent.join(candidate);
                            if img_path.is_file() {
                                if let Ok(bytes) = std::fs::read(&img_path) {
                                    let mime = if candidate.ends_with(".png") {
                                        "image/png"
                                    } else {
                                        "image/jpeg"
                                    };
                                    let encoded =
                                        base64::engine::general_purpose::STANDARD.encode(&bytes);
                                    cover_result =
                                        Some(format!("data:{};base64,{}", mime, encoded));
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
        if cover_result.is_none() {
            let path: Option<String> = sqlx::query_scalar("SELECT cover_art_path FROM track_enrichment WHERE track_id = ? AND state = 'matched'")
                .bind(&track_id).fetch_optional(&self.db_pool).await?.flatten();
            if let Some(path) = path {
                if let Ok(bytes) = tokio::fs::read(path).await {
                    let mime = image::guess_format(&bytes)
                        .ok()
                        .map(|f| f.to_mime_type())
                        .unwrap_or("image/jpeg");
                    cover_result = Some(format!(
                        "data:{};base64,{}",
                        mime,
                        base64::engine::general_purpose::STANDARD.encode(bytes)
                    ));
                }
            }
        }
        self.cover_art_cache
            .write()
            .await
            .insert(track_id.to_string(), cover_result.clone());
        Ok(cover_result)
    }

    /// Returns query data with online sources bound before songs are exposed.
    pub async fn execute_query(&self, query: Query) -> AppResult<QueryResponse> {
        info!(?query, "Executing query");

        match query {
            Query::GetPlaybackState => {
                self.playback_service.initialize_saved_playback().await;
                let state = self.playback_service.get_playback_state().await;
                let mut val =
                    serde_json::to_value(&state).map_err(|e| AppError::Internal(e.to_string()))?;
                if let Some(track_id) = &state.current_track_id {
                    let user_id = self.current_user_id().await;
                    if let Ok(Some(track)) = self
                        .library_service
                        .track_repo()
                        .find_by_id_scoped(track_id, Some(&user_id))
                        .await
                    {
                        if let Ok(track_val) = serde_json::to_value(track) {
                            if let Some(obj) = val.as_object_mut() {
                                obj.insert("current_track".to_string(), track_val);
                            }
                        }
                    }
                }
                Ok(QueryResponse::PlaybackState(
                    self.online_catalog.present(val).await?,
                ))
            }
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
            Query::GetRecommendationSetup => {
                let user = self.current_user_id().await;
                let mut setup = crate::recommendations::setup::load(&self.db_pool, &user).await?;
                setup.should_prompt = setup.status == "pending"
                    && !self
                        .taste_engine
                        .compute_taste_profile(&user)
                        .await?
                        .has_learned_taste;
                Ok(QueryResponse::RecommendationSetup(setup))
            }
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
            Query::GetSettings => {
                self.initialize_close_behavior().await?;
                let mut cfg = self.config.read().await.clone();
                cfg.close_button_behavior = self.close_button_behavior().to_string();
                let mut val =
                    serde_json::to_value(&cfg).map_err(|e| AppError::Internal(e.to_string()))?;
                let style = self.settings_repo.get_setting("interface_style").await?;
                val["interface_style"] = serde_json::json!(match style.as_deref() {
                    Some("brutalist") => "brutalist",
                    _ => "spatial",
                });
                for (key, default) in [("enable_jiosaavn", true), ("auto_fill_metadata", true)] {
                    val["metadata"][key] =
                        serde_json::json!(self.gap_enrichment.enabled(key, default).await);
                }
                Ok(QueryResponse::Settings(val))
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
            Query::GetTasteProfile => {
                let user_id = self.current_user_id().await;
                let profile = self.taste_engine.compute_taste_profile(&user_id).await?;
                let val = serde_json::to_value(&profile)
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                Ok(QueryResponse::TasteProfile(val))
            }
            Query::GetLocalRecommendations { limit } => {
                let user_id = self.current_user_id().await;
                let recs = self.recommender.recommend(&user_id, limit as usize).await?;
                let val: Vec<serde_json::Value> = recs
                    .into_iter()
                    .filter_map(|r| serde_json::to_value(r).ok())
                    .collect();
                Ok(QueryResponse::Recommendations(
                    self.present_recommendations(serde_json::Value::Array(val), &user_id)
                        .await?
                        .as_array()
                        .cloned()
                        .unwrap_or_default(),
                ))
            }
            Query::GetSmartMixes | Query::RefreshSmartMixes => {
                if matches!(query, Query::RefreshSmartMixes) {
                    sqlx::query(
                        "UPDATE playlists SET expires_at=0 WHERE user_id=? AND is_smart_mix=1",
                    )
                    .bind(self.current_user_id().await)
                    .execute(&self.db_pool)
                    .await?;
                }
                let user_id = self.current_user_id().await;
                self.discovery_coordinator
                    .ensure_starter_candidates(&user_id, false)
                    .await?;
                let mixes = self
                    .smart_mix_generator
                    .ensure_default_mixes(&user_id)
                    .await?;
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
                let is_logged_in = self.current_user.read().await.is_some();
                let user_id = self.current_user_id().await;
                self.discovery_coordinator
                    .ensure_starter_candidates(&user_id, false)
                    .await?;
                let _ = self
                    .smart_mix_generator
                    .ensure_default_mixes(&user_id)
                    .await;
                if is_logged_in {
                    self.playlist_repo
                        .ensure_liked_songs_playlist(&user_id)
                        .await?;
                }
                let raw_playlists = self.playlist_repo.get_user_playlists(&user_id).await?;
                let playlists: Vec<_> = if is_logged_in {
                    raw_playlists
                } else {
                    raw_playlists
                        .into_iter()
                        .filter(|p| p.is_smart_mix == 1)
                        .collect()
                };
                let mut val: Vec<serde_json::Value> = Vec::new();
                for p in playlists {
                    let count = self.playlist_repo.get_track_count(&p.id).await.unwrap_or(0);

                    // Query the first track in the playlist to use its thumbnail/cover
                    let first_song: Option<(
                        String,
                        Option<String>,
                        Option<String>,
                        i64,
                        Option<String>,
                    )> = sqlx::query_as(
                        "SELECT pt.track_id, 
                                ext.cover_art_url, 
                                al.cover_art_path, 
                                COALESCE(t.has_cover_art, 0),
                                t.file_path
                         FROM playlist_tracks pt
                         LEFT JOIN tracks t ON t.id = pt.track_id
                         LEFT JOIN albums al ON al.id = t.album_id
                         LEFT JOIN external_tracks ext ON ext.id = pt.track_id
                         WHERE pt.playlist_id = ?
                         ORDER BY pt.position ASC
                         LIMIT 1",
                    )
                    .bind(&p.id)
                    .fetch_optional(&self.db_pool)
                    .await
                    .ok()
                    .flatten();

                    if let Ok(mut v) = serde_json::to_value(&p) {
                        if let Some(obj) = v.as_object_mut() {
                            obj.insert("track_count".to_string(), serde_json::json!(count));

                            if let Some((
                                track_id,
                                ext_cover,
                                album_cover,
                                _has_cover,
                                _file_path_opt,
                            )) = first_song
                            {
                                obj.insert(
                                    "first_track_id".to_string(),
                                    serde_json::json!(&track_id),
                                );

                                let mut resolved_cover = ext_cover.or(album_cover);

                                if resolved_cover.is_none() {
                                    resolved_cover = self.load_track_artwork(&track_id).await?;
                                }

                                if let Some(cover) = resolved_cover {
                                    obj.insert(
                                        "cover_art_url".to_string(),
                                        serde_json::json!(cover),
                                    );
                                }
                            }
                        }
                        val.push(v);
                    }
                }
                Ok(QueryResponse::Playlists(val))
            }
            Query::GetPlaylistTracks { playlist_id } => {
                let _ =
                    crate::recommendations::mixes::repair_legacy_online_tracks(&self.db_pool).await;
                let tracks = self.playlist_repo.get_playlist_tracks(&playlist_id).await?;
                let val: Vec<serde_json::Value> = tracks
                    .into_iter()
                    .filter_map(|t| serde_json::to_value(t).ok())
                    .collect();
                let smart: bool =
                    sqlx::query_scalar("SELECT is_smart_mix=1 FROM playlists WHERE id=?")
                        .bind(&playlist_id)
                        .fetch_optional(&self.db_pool)
                        .await?
                        .unwrap_or(false);
                let value = if smart {
                    self.present_recommendations(
                        serde_json::Value::Array(val),
                        &self.current_user_id().await,
                    )
                    .await?
                } else {
                    self.online_catalog
                        .present(serde_json::Value::Array(val))
                        .await?
                };
                Ok(QueryResponse::PlaylistTracks(
                    value.as_array().cloned().unwrap_or_default(),
                ))
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
                let user_id = self.current_user_id().await;
                let recs = self
                    .discovery_coordinator
                    .get_discovery_recommendations(
                        &user_id,
                        limit as usize,
                        force_refresh.unwrap_or(false),
                    )
                    .await?;
                self.flow_engine.invalidate_home_picks(&user_id).await;
                let val: Vec<serde_json::Value> = recs
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
            Query::GetRecommendationFeedback => {
                let signals = crate::recommendations::signals::SongSignals::load(
                    &self.db_pool,
                    &self.current_user_id().await,
                )
                .await?;
                let ids = signals
                    .identities
                    .keys()
                    .filter(|id| signals.blocked_id(id))
                    .cloned()
                    .collect();
                Ok(QueryResponse::RecommendationFeedback(ids))
            }
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
            Query::GetDownloads {
                status_filter,
                limit,
            } => {
                let tasks = self
                    .download_service
                    .list_downloads(status_filter.as_deref(), limit.min(10000))
                    .await?;
                let aliases: Vec<(String,String)>=sqlx::query_as("SELECT track_id,provider_id FROM online_track_sources UNION SELECT id,provider_id FROM external_tracks WHERE provider='jiosaavn'").fetch_all(&self.db_pool).await?;
                let mut sources: std::collections::HashMap<String, Vec<String>> =
                    std::collections::HashMap::new();
                for (id, source) in aliases {
                    sources.entry(source).or_default().push(id);
                }
                let val = tasks
                    .into_iter()
                    .filter_map(|task| {
                        let ids = task
                            .source_track_id
                            .as_ref()
                            .and_then(|id| sources.get(id))
                            .cloned()
                            .unwrap_or_default();
                        let mut value = serde_json::to_value(task).ok()?;
                        value["track_ids"] = serde_json::json!(ids);
                        Some(value)
                    })
                    .collect();
                Ok(QueryResponse::Downloads(val))
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
            Query::GetTrackCoverArt { track_id } => Ok(QueryResponse::CoverArt(
                self.load_track_artwork(&track_id).await?,
            )),
            Query::GetTrackPlaylistMemberships => {
                let user_id = self.current_user_id().await;
                let memberships = self
                    .playlist_repo
                    .get_track_playlist_memberships_for_user(&user_id)
                    .await?;
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
                    if let Ok(Some(track)) = self.library_service.track_repo().find_by_id(tid).await
                    {
                        let path = std::path::Path::new(&track.file_path);
                        if path.exists() {
                            if let Ok(probe) = lofty::probe::Probe::open(path) {
                                if let Ok(probe) = probe.guess_file_type() {
                                    if let Ok(tagged_file) = probe.read() {
                                        if let Some(tag) = tagged_file
                                            .primary_tag()
                                            .or_else(|| tagged_file.first_tag())
                                        {
                                            if let Some(lyrics_val) =
                                                tag.get_string(&lofty::tag::ItemKey::Lyrics)
                                            {
                                                let is_synced = lyrics_val.contains('[')
                                                    && lyrics_val.contains(']');
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

                match client
                    .get("https://lrclib.net/api/get")
                    .query(&params)
                    .send()
                    .await
                {
                    Ok(resp) if resp.status().is_success() => {
                        if let Ok(json) = resp.json::<serde_json::Value>().await {
                            let plain = json
                                .get("plainLyrics")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                            let synced = json
                                .get("syncedLyrics")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
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
            Query::GetCurrentUser => {
                let current_user_guard = self.current_user.read().await;
                if let Some(mut u) = current_user_guard.clone() {
                    if u.avatar_data_url.is_none() {
                        u.avatar_data_url = self.profile_service.get_cached_avatar_data_url(&u.id);
                    }
                    let val = serde_json::to_value(&u).ok();
                    return Ok(QueryResponse::CurrentUser(val));
                }
                drop(current_user_guard);

                // If startup background validation is still resolving, return active valid local session
                if let Ok(Some(session)) = SyncManager::get_active_session(&self.db_pool).await {
                    let now = chrono::Utc::now().timestamp();
                    let worker_url = self.get_cloud_worker_url().await;
                    let is_idle_expired =
                        session.idle_expires_at > 0 && now >= session.idle_expires_at;
                    let is_abs_expired =
                        session.absolute_expires_at > 0 && now >= session.absolute_expires_at;
                    if !is_idle_expired && !is_abs_expired && session.worker_url == worker_url {
                        if let Some(_tok) = credentials::get_session_token(&session.user_id) {
                            let cached_avatar = self
                                .profile_service
                                .get_cached_avatar_data_url(&session.user_id);
                            let local_user = self
                                .user_repo
                                .get_user_by_id(&session.user_id)
                                .await
                                .ok()
                                .flatten();
                            let profile = UserProfile::new(
                                session.user_id.clone(),
                                session.username.clone(),
                                session.created_at,
                            )
                            .with_avatar(
                                local_user.as_ref().and_then(|u| u.display_name.clone()),
                                if cached_avatar.is_some() {
                                    Some(format!("avatars/{}.webp", session.user_id))
                                } else {
                                    None
                                },
                                local_user.as_ref().and_then(|u| u.avatar_public_id.clone()),
                                local_user.as_ref().and_then(|u| u.avatar_url.clone()),
                                local_user.as_ref().and_then(|u| u.avatar_version),
                                local_user.as_ref().and_then(|u| u.avatar_updated_at),
                                cached_avatar,
                            );
                            self.playback_service.stop_flow().await;
                            *self.current_user.write().await = Some(profile.clone());
                            *self.session_state.write().await =
                                AuthSessionState::OnlineAuthenticated {
                                    user: CloudUser {
                                        id: session.user_id.clone(),
                                        username: session.username.clone(),
                                        created_at: session.created_at,
                                        display_name: profile.display_name.clone(),
                                        has_avatar: Some(profile.avatar_key.is_some()),
                                        avatar_public_id: profile.avatar_public_id.clone(),
                                        avatar_url: profile.avatar_url.clone(),
                                        avatar_version: profile.avatar_version,
                                        avatar_key: profile.avatar_key.clone(),
                                        avatar_updated_at: profile.avatar_updated_at,
                                    },
                                    session_id: session.session_id.clone(),
                                };
                            let _ = self
                                .playlist_repo
                                .reassign_orphan_playlists_to_user(&session.user_id)
                                .await;
                            return Ok(QueryResponse::CurrentUser(
                                serde_json::to_value(&profile).ok(),
                            ));
                        }
                    }
                }

                Ok(QueryResponse::CurrentUser(None))
            }
            Query::GetProfile => {
                let current_user_guard = self.current_user.read().await;
                if let Some(mut u) = current_user_guard.clone() {
                    if u.avatar_data_url.is_none() {
                        u.avatar_data_url = self.profile_service.get_cached_avatar_data_url(&u.id);
                    }
                    let val = serde_json::to_value(&u).ok();
                    return Ok(QueryResponse::Profile(val));
                }
                drop(current_user_guard);

                if let Ok(Some(session)) = SyncManager::get_active_session(&self.db_pool).await {
                    let cached_avatar = self
                        .profile_service
                        .get_cached_avatar_data_url(&session.user_id);
                    let local_user = self
                        .user_repo
                        .get_user_by_id(&session.user_id)
                        .await
                        .ok()
                        .flatten();
                    let profile = UserProfile::new(
                        session.user_id.clone(),
                        session.username.clone(),
                        session.created_at,
                    )
                    .with_avatar(
                        local_user.as_ref().and_then(|u| u.display_name.clone()),
                        if cached_avatar.is_some() {
                            Some(format!("avatars/{}.webp", session.user_id))
                        } else {
                            None
                        },
                        local_user.as_ref().and_then(|u| u.avatar_public_id.clone()),
                        local_user.as_ref().and_then(|u| u.avatar_url.clone()),
                        local_user.as_ref().and_then(|u| u.avatar_version),
                        local_user.as_ref().and_then(|u| u.avatar_updated_at),
                        cached_avatar,
                    );
                    let val = serde_json::to_value(&profile).ok();
                    return Ok(QueryResponse::Profile(val));
                }
                Ok(QueryResponse::Profile(None))
            }
            Query::GetAvatar { user_id } => {
                let target_id = match user_id {
                    Some(id) => id,
                    None => {
                        let guard = self.current_user.read().await;
                        match guard.as_ref() {
                            Some(u) => u.id.clone(),
                            None => return Ok(QueryResponse::Avatar(None)),
                        }
                    }
                };
                let data_url = self.profile_service.get_cached_avatar_data_url(&target_id);
                Ok(QueryResponse::Avatar(data_url))
            }
            Query::GetCloudSyncStatus => {
                let worker_url = self.get_cloud_worker_url().await;
                let session = SyncManager::get_active_session(&self.db_pool)
                    .await
                    .ok()
                    .flatten();
                let current_user_guard = self.current_user.read().await;
                let current_state = self.session_state.read().await.clone();

                let status = CloudSyncStatus {
                    connected: session.is_some()
                        && !matches!(
                            current_state,
                            AuthSessionState::SignedOut | AuthSessionState::SessionExpired { .. }
                        ),
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
                let session = SyncManager::get_active_session(&self.db_pool)
                    .await
                    .ok()
                    .flatten();
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
            Query::GetRemoteAudioCacheStats => {
                let stats = self.playback_service.get_remote_audio_cache_stats().await?;
                Ok(QueryResponse::RemoteAudioCacheStats {
                    total_size_bytes: stats.total_size_bytes,
                    file_count: stats.file_count,
                    max_size_bytes: stats.max_size_bytes,
                    partial_file_count: stats.partial_file_count,
                })
            }
            Query::GetHomeOverview => {
                let user_id = self.current_user_id().await;
                self.discovery_coordinator
                    .ensure_starter_candidates(&user_id, false)
                    .await?;
                let mut overview = self.flow_engine.get_home_overview(&user_id).await?;
                overview.flow_session = self
                    .playback_service
                    .flow_session(&self.flow_engine, &user_id)
                    .await?;
                let val = serde_json::to_value(overview).unwrap_or(serde_json::Value::Null);
                Ok(QueryResponse::HomeOverview(
                    self.present_recommendations(val, &user_id).await?,
                ))
            }
            Query::GetFlowQueue {
                familiarity_ratio,
                limit,
                exclude_track_ids,
            } => {
                let user_id = self.current_user_id().await;
                let ratio = familiarity_ratio.unwrap_or(0.5);
                let lim = limit.unwrap_or(20) as usize;
                let exclude = exclude_track_ids.unwrap_or_default();
                let tracks = self
                    .flow_engine
                    .generate_flow_tracks(&user_id, ratio, lim, &exclude)
                    .await?;
                let val: Vec<serde_json::Value> = tracks
                    .into_iter()
                    .filter_map(|t| serde_json::to_value(t).ok())
                    .collect();
                Ok(QueryResponse::FlowQueue(
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
            Query::GetFlowTrackReason { track_id } => {
                let user_id = self.current_user_id().await;
                let reason = match self.playback_service.flow_reason(&user_id, &track_id).await {
                    Some(reason) => Some(reason),
                    None => Some(
                        self.flow_engine
                            .get_track_explanation(&user_id, &track_id)
                            .await?,
                    ),
                };
                Ok(QueryResponse::FlowTrackReason(reason))
            }
            Query::GetFlowSession => {
                let user_id = self.current_user_id().await;
                let session = self
                    .playback_service
                    .flow_session(&self.flow_engine, &user_id)
                    .await?;
                let val = session.and_then(|s| serde_json::to_value(s).ok());
                Ok(QueryResponse::FlowSession(val))
            }
            _ => {
                warn!(?query, "Query handler routed to fallback");
                Ok(QueryResponse::Empty)
            }
        }
    }

    async fn set_song_feedback(&self, track_id: &str, feedback: i64) -> AppResult<()> {
        let user = self.current_user_id().await;
        if self
            .library_service
            .track_repo()
            .find_by_id(track_id)
            .await?
            .is_none()
        {
            let row: Option<(String,String,Option<String>,Option<f64>,Option<String>)> = sqlx::query_as("SELECT title,artist,album,duration_secs,cover_art_url FROM external_tracks WHERE id=?").bind(track_id).fetch_optional(&self.db_pool).await?;
            let Some((title, artist, album, duration, cover)) = row else {
                return Err(AppError::NotFound("Song not found".into()));
            };
            crate::recommendations::mixes::ensure_online_track(
                &self.db_pool,
                track_id,
                &title,
                Some(&artist),
                album.as_deref(),
                duration,
                cover.as_deref(),
                None,
            )
            .await?;
        }
        let index =
            crate::recommendations::signals::SongSignals::load(&self.db_pool, &user).await?;
        let ids = index.related_ids(track_id);
        let title = index
            .identities
            .get(track_id)
            .map(|song| song.title.clone())
            .unwrap_or_else(|| track_id.into());
        let liked = self
            .playlist_repo
            .ensure_liked_songs_playlist(&user)
            .await?;
        let mut transaction = self.db_pool.begin().await?;
        let identity = index
            .identities
            .get(track_id)
            .ok_or_else(|| AppError::NotFound("Song identity not found".into()))?;
        let previous: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(updated_at),0) FROM recommendation_feedback WHERE user_id=?",
        )
        .bind(&user)
        .fetch_one(&mut *transaction)
        .await?;
        let timestamp = chrono::Utc::now()
            .timestamp_millis()
            .max(previous.saturating_add(1));
        for key in index.related_feedback_keys(track_id) {
            sqlx::query("UPDATE recommendation_feedback SET manual_like=?,updated_at=? WHERE user_id=? AND song_key=?").bind(feedback).bind(timestamp).bind(&user).bind(key).execute(&mut *transaction).await?;
        }
        let key = crate::recommendations::signals::song_key(&identity.title, &identity.artist)
            .unwrap_or_else(|| format!("id:{track_id}"));
        sqlx::query("INSERT INTO recommendation_feedback(user_id,song_key,track_id,source_id,title,artist,manual_like,updated_at) VALUES(?,?,?,?,?,?,?,?) ON CONFLICT(user_id,song_key) DO UPDATE SET manual_like=excluded.manual_like,updated_at=excluded.updated_at,track_id=excluded.track_id,source_id=excluded.source_id,title=excluded.title,artist=excluded.artist")
            .bind(&user).bind(key).bind(track_id).bind(&identity.source_id).bind(&identity.title).bind(&identity.artist).bind(feedback).bind(timestamp).execute(&mut *transaction).await?;

        // One explicit opinion per recording, even when several saved IDs exist.
        for id in &ids {
            sqlx::query("UPDATE track_statistics SET manual_like=0 WHERE user_id=? AND track_id=?")
                .bind(&user)
                .bind(id)
                .execute(&mut *transaction)
                .await?;
            sqlx::query("DELETE FROM playlist_tracks WHERE playlist_id=? AND track_id=?")
                .bind(&liked)
                .bind(id)
                .execute(&mut *transaction)
                .await?;
            if feedback == -1 {
                sqlx::query("DELETE FROM playlist_tracks WHERE track_id=? AND playlist_id IN (SELECT id FROM playlists WHERE user_id=? AND is_smart_mix=1)").bind(id).bind(&user).execute(&mut *transaction).await?;
            }
        }
        sqlx::query("INSERT INTO track_statistics(user_id,track_id,manual_like) VALUES(?,?,?) ON CONFLICT(user_id,track_id) DO UPDATE SET manual_like=excluded.manual_like")
            .bind(&user).bind(track_id).bind(feedback).execute(&mut *transaction).await?;
        sqlx::query("UPDATE playlists SET expires_at=0 WHERE user_id=? AND is_smart_mix=1")
            .bind(&user)
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        if feedback == 1 {
            self.playlist_repo.add_track(&liked, track_id, None).await?;
        }
        if feedback == -1 {
            self.playback_service
                .remove_recommended_songs(&user, &ids)
                .await?;
        }
        self.flow_engine.invalidate_home_picks(&user).await;
        let _ = self
            .event_bus
            .publish(Event::RecommendationFeedbackChanged {
                user_id: user.clone(),
                track_id: track_id.into(),
                title,
                track_ids: ids,
                disliked: feedback == -1,
                manual_like: feedback,
            });
        let _ = self
            .event_bus
            .publish(Event::TasteProfileChanged { user_id: user });
        self.trigger_background_sync().await;
        Ok(())
    }

    async fn present_recommendations(
        &self,
        value: serde_json::Value,
        user: &str,
    ) -> AppResult<serde_json::Value> {
        let value = self.online_catalog.present(value).await?;
        Ok(
            crate::recommendations::signals::SongSignals::load(&self.db_pool, user)
                .await?
                .filter(value)
                .unwrap_or(serde_json::Value::Null),
        )
    }

    async fn add_missing_to_wishlist(&self, tracks: Vec<serde_json::Value>) -> AppResult<usize> {
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

    async fn import_jiosaavn_playlist(&self, reference: &str) -> AppResult<serde_json::Value> {
        if !self.provider_coordinator.enabled().await {
            return Err(AppError::Validation(
                "Enable online music in Settings to import a JioSaavn playlist".into(),
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
            songs.push(serde_json::json!({"track_id":row.id,"title":row.title,"artist":row.artist,
                "album":row.album,"duration_secs":row.duration_secs,"cover_art_url":row.cover_art_url,
                "in_library":false,"match_status":"NOT_FOUND","matched_local_track_id":null}));
        }
        Ok(
            serde_json::json!({"playlist_id":id,"title":title,"cover_url":image,
            "total_tracks":songs.len(),"matched_tracks":0,"missing_tracks":songs.len(),"tracks":songs}),
        )
    }
}

#[cfg(test)]
#[path = "processor_tests.rs"]
mod tests;
