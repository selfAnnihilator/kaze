//! Live cloud sync while the app is open: the signed-in profile (name, photo)
//! is checked every `PROFILE_EVERY`, and listening data and playlists are
//! synced every `DATA_EVERY`, so a change made on another device shows up
//! without restarting. The worker has no push channel, so this polls.

use crate::cloud::client::CloudClient;
use crate::cloud::credentials;
use crate::cloud::models::CloudUser;
use crate::cloud::sync_manager::SyncManager;
use crate::core::event::Event;
use crate::core::event_bus::EventBus;
use crate::database::repositories::user_repo::{SqliteUserRepository, UserProfile, UserRepository};
use crate::profile::ProfileService;
use sqlx::SqlitePool;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// How often the profile is checked against the cloud.
pub const PROFILE_EVERY: Duration = Duration::from_secs(20);
/// How many profile checks pass between full data syncs (6 × 20 s = 2 min).
pub const DATA_SYNC_TICKS: u32 = 6;

/// What the live sync loop needs from the processor.
pub struct LiveSync {
    pub pool: SqlitePool,
    pub client: Arc<CloudClient>,
    pub profiles: Arc<ProfileService>,
    pub current_user: Arc<RwLock<Option<UserProfile>>>,
    pub events: Arc<EventBus>,
    pub worker_url: String,
}

/// True when the cloud profile differs from what this device shows.
pub fn profile_changed(local: &UserProfile, cloud: &CloudUser) -> bool {
    let has_avatar = cloud.has_avatar.unwrap_or(false);
    local.display_name != cloud.display_name
        || has_avatar != local.avatar_data_url.is_some()
        || (has_avatar
            && (local.avatar_version != cloud.avatar_version
                || local.avatar_updated_at != cloud.avatar_updated_at))
}

impl LiveSync {
    /// Runs forever; each tick is skipped while nobody is signed in.
    pub async fn run(self) {
        let mut tick: u32 = 0;
        loop {
            tokio::time::sleep(PROFILE_EVERY).await;
            tick = tick.wrapping_add(1);
            let Some((user_id, token)) = self.signed_in().await else { continue };
            self.refresh_profile(&user_id, &token).await;
            if tick.is_multiple_of(DATA_SYNC_TICKS) {
                self.sync_data(&user_id, &token).await;
            }
        }
    }

    async fn signed_in(&self) -> Option<(String, String)> {
        let user_id = self.current_user.read().await.as_ref()?.id.clone();
        let session = SyncManager::get_active_session(&self.pool).await.ok()??;
        if session.user_id != user_id {
            return None;
        }
        Some((user_id.clone(), credentials::get_session_token(&user_id)?))
    }

    async fn refresh_profile(&self, user_id: &str, token: &str) {
        let Ok(cloud) = self.client.get_me(&self.worker_url, token).await else { return };
        let Some(local) = self.current_user.read().await.clone() else { return };
        if cloud.id != user_id || !profile_changed(&local, &cloud) {
            return;
        }
        let avatar_data_url = self.fetch_avatar(&cloud, token).await;
        let avatar_key = avatar_data_url.as_ref().map(|_| format!("avatars/{}.webp", cloud.id));
        let _ = SqliteUserRepository::new(self.pool.clone())
            .update_avatar_metadata(
                user_id,
                avatar_key.as_deref(),
                cloud.avatar_public_id.as_deref(),
                cloud.avatar_url.as_deref(),
                cloud.avatar_version,
                cloud.avatar_updated_at,
            )
            .await;
        let updated = {
            let mut guard = self.current_user.write().await;
            let Some(user) = guard.as_mut().filter(|u| u.id == user_id) else { return };
            user.display_name = cloud.display_name.clone();
            user.avatar_key = avatar_key;
            user.avatar_public_id = cloud.avatar_public_id.clone();
            user.avatar_url = cloud.avatar_url.clone();
            user.avatar_version = cloud.avatar_version;
            user.avatar_updated_at = cloud.avatar_updated_at;
            user.avatar_data_url = avatar_data_url;
            serde_json::to_value(&*user).unwrap_or_default()
        };
        let _ = self.events.publish(Event::UserProfileUpdated { user: updated });
    }

    /// The cloud photo as a data URL, cached locally; `None` once it was removed.
    async fn fetch_avatar(&self, cloud: &CloudUser, token: &str) -> Option<String> {
        if !cloud.has_avatar.unwrap_or(false) {
            self.profiles.remove_cached_avatar(&cloud.id);
            return None;
        }
        if let Ok(Some(bytes)) = self.client.download_avatar(&self.worker_url, Some(token), &cloud.id).await {
            let _ = self.profiles.save_cached_avatar(&cloud.id, &bytes);
        }
        self.profiles.get_cached_avatar_data_url(&cloud.id)
    }

    async fn sync_data(&self, user_id: &str, token: &str) {
        match SyncManager::sync_with_cloud(&self.pool, &self.client, &self.worker_url, user_id, token).await {
            Ok(_) => {
                let _ = self.events.publish(Event::PlaylistsUpdated);
                let _ = self.events.publish(Event::CloudDataSynced);
            }
            Err(e) => tracing::warn!("Live cloud sync failed: {}", e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cloud(has_avatar: bool, version: Option<i64>) -> CloudUser {
        serde_json::from_value(serde_json::json!({
            "id": "u1", "username": "zanken", "created_at": 1,
            "has_avatar": has_avatar, "avatar_version": version, "avatar_updated_at": version,
        }))
        .expect("cloud user")
    }

    fn local(photo: bool, version: Option<i64>) -> UserProfile {
        let mut user = UserProfile::new("u1".into(), "zanken".into(), 1);
        user.avatar_data_url = photo.then(|| "data:image/webp;base64,AA".into());
        user.avatar_version = version;
        user.avatar_updated_at = version;
        user
    }

    #[test]
    fn a_new_photo_elsewhere_is_a_change() {
        assert!(profile_changed(&local(true, Some(1)), &cloud(true, Some(2))));
        assert!(profile_changed(&local(false, None), &cloud(true, Some(1))));
    }

    #[test]
    fn a_removed_photo_is_a_change_and_same_photo_is_not() {
        assert!(profile_changed(&local(true, Some(1)), &cloud(false, None)));
        assert!(!profile_changed(&local(true, Some(1)), &cloud(true, Some(1))));
        assert!(!profile_changed(&local(false, None), &cloud(false, None)));
    }
}
