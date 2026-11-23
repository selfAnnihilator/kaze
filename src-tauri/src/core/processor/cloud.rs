//! Processor handlers for cloud sync and its server.

use super::routing::misrouted;
use super::*;

impl CoreProcessor {
    pub(super) async fn cloud_command(&self, cmd: Command) -> AppResult<CommandResponse> {
        match cmd {
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
                let _ = self.event_bus.publish(Event::CloudDataSynced);

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
            other => Err(misrouted(&other)),
        }
    }

    pub(super) async fn cloud_query(&self, query: Query) -> AppResult<QueryResponse> {
        match query {
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
            other => Err(misrouted(&other)),
        }
    }
}
