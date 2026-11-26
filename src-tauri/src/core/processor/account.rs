//! Processor handlers for sign-in, sessions, profile/avatar and cloud sync.

use super::routing::misrouted;
use super::*;

impl CoreProcessor {
    pub(super) async fn account_command(&self, cmd: Command) -> AppResult<CommandResponse> {
        match cmd {
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
                        match SyncManager::sync_with_cloud_retrying(
                            &pool_clone,
                            &client_clone,
                            &worker_url,
                            &user_id_clone,
                            &token,
                        )
                        .await
                        {
                            Ok(_) => {
                                let _ = event_bus_clone.publish(Event::PlaylistsUpdated);
                                let _ = event_bus_clone.publish(Event::CloudDataSynced);
                            }
                            Err(e) => tracing::warn!("Sign-in cloud sync failed: {}", e),
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

                    if let Err(e) = credentials::store_session_token(&profile.id, &token) {
                        tracing::warn!(
                            "Failed to store session token in secure credentials: {}",
                            e
                        );
                    }

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

                    crate::recommendations::setup::remember_returning_listener(&self.db_pool, &profile.id).await;
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

                    let pool_clone = self.db_pool.clone();
                    let client_clone = self.cloud_client.clone();
                    let user_id_clone = profile.id.clone();
                    let event_bus_clone = self.event_bus.clone();
                    tokio::spawn(async move {
                        match SyncManager::sync_with_cloud_retrying(
                            &pool_clone,
                            &client_clone,
                            &worker_url,
                            &user_id_clone,
                            &token,
                        )
                        .await
                        {
                            Ok(_) => {
                                let _ = event_bus_clone.publish(Event::PlaylistsUpdated);
                                let _ = event_bus_clone.publish(Event::CloudDataSynced);
                            }
                            Err(e) => tracing::warn!("Sign-in cloud sync failed: {}", e),
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
            other => Err(misrouted(&other)),
        }
    }

}
