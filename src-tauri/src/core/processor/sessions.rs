//! Processor handlers for sign-out, sessions and the signed-in user.

use super::routing::misrouted;
use super::*;

impl CoreProcessor {
    pub(super) async fn sessions_command(&self, cmd: Command) -> AppResult<CommandResponse> {
        match cmd {
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
            other => Err(misrouted(&other)),
        }
    }

    pub(super) async fn sessions_query(&self, query: Query) -> AppResult<QueryResponse> {
        match query {
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
                                AvatarFields {
                                    display_name: local_user.as_ref().and_then(|u| u.display_name.clone()),
                                    avatar_key: if cached_avatar.is_some() {
                                    Some(format!("avatars/{}.webp", session.user_id))
                                } else {
                                    None
                                },
                                    avatar_public_id: local_user.as_ref().and_then(|u| u.avatar_public_id.clone()),
                                    avatar_url: local_user.as_ref().and_then(|u| u.avatar_url.clone()),
                                    avatar_version: local_user.as_ref().and_then(|u| u.avatar_version),
                                    avatar_updated_at: local_user.as_ref().and_then(|u| u.avatar_updated_at),
                                    avatar_data_url: cached_avatar,
                                },
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
            other => Err(misrouted(&other)),
        }
    }
}
