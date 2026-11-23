//! Processor handlers for the profile and avatar.

use super::routing::misrouted;
use super::*;

impl CoreProcessor {
    pub(super) async fn profile_command(&self, cmd: Command) -> AppResult<CommandResponse> {
        match cmd {
            Command::UploadAvatar { file_path, image_base64 } => {
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

                let Some(raw_bytes) = ProfileService::avatar_upload_bytes(file_path, image_base64)? else {
                    return Ok(CommandResponse::Ok);
                };
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
            other => Err(misrouted(&other)),
        }
    }

    pub(super) async fn profile_query(&self, query: Query) -> AppResult<QueryResponse> {
        match query {
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
            other => Err(misrouted(&other)),
        }
    }
}
