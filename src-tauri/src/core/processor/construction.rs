//! Building the processor and wiring its services.

use super::*;

impl CoreProcessor {
    /// Constructs a new CoreProcessor instance with standard Rodio audio backend.
    pub fn new(db_pool: SqlitePool, config: AppConfig) -> Self {
        let backend = Box::new(RodioAudioBackend::new());
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
        ));
        let gap_enrichment = Arc::new(crate::providers::enrichment::GapEnrichmentService::new(
            db_pool.clone(),
            provider_coordinator.clone(),
            config.cache_dir.clone(),
            event_bus.clone(),
        ));
        background_tasks.push(tokio::spawn(gap_enrichment.clone().run()));
        let similarity_service = Arc::new(
            crate::providers::similarity_worker::SimilarityService::new(
                db_pool.clone(),
                event_bus.clone(),
            ),
        );
        background_tasks.push(tokio::spawn(similarity_service.clone().run()));
        let audio_features = Arc::new(crate::audio_features::AudioFeaturesService::new(
            db_pool.clone(),
            event_bus.clone(),
        ));
        background_tasks.push(tokio::spawn(audio_features.run()));
        let cover_art_cache = Arc::new(tokio::sync::RwLock::new(HashMap::new()));
        let cover_cache_updates = cover_art_cache.clone();
        let metadata_playback = playback_service.clone();
        let mut cover_events = event_bus.subscribe();
        background_tasks.push(tokio::spawn(async move {
            loop {
                match cover_events.recv().await {
                    Ok(Event::LibraryMetadataUpdated | Event::LibraryScanCompleted { .. }) => {
                        cover_cache_updates.write().await.clear();
                        crate::library::artwork::bump_version();
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
                // Kaze owns this folder; the user's own folders stay read-only.
                .and_then(|u| u.audio_dir().map(|p| p.join("Kaze")))
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
                            AvatarFields {
                                display_name: cloud_user.display_name.clone(),
                                avatar_key,
                                avatar_public_id: cloud_user.avatar_public_id.clone(),
                                avatar_url: cloud_user.avatar_url.clone(),
                                avatar_version: cloud_user.avatar_version,
                                avatar_updated_at: cloud_user.avatar_updated_at,
                                avatar_data_url,
                            },
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
                                AvatarFields {
                                    display_name: None,
                                    avatar_key: if cached_avatar.is_some() { Some(format!("avatars/{}.webp", session.user_id)) } else { None },
                                    avatar_public_id: None,
                                    avatar_url: None,
                                    avatar_version: None,
                                    avatar_updated_at: None,
                                    avatar_data_url: cached_avatar,
                                },
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
            similarity_service,
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
}
