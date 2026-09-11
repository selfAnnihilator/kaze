use crate::core::command::WishlistStatus;
use crate::core::error::{AppError, AppResult};
use crate::database::models::{ExternalTrackRecord, WishlistItemRecord};
use crate::database::repositories::WishlistRepository;
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

pub struct WishlistManager {
    repo: Arc<dyn WishlistRepository>,
}

impl WishlistManager {
    pub fn new(repo: Arc<dyn WishlistRepository>) -> Self {
        Self { repo }
    }

    pub fn repository(&self) -> Arc<dyn WishlistRepository> {
        self.repo.clone()
    }

    /// Adds a track to the wishlist with default status 'WANT'.
    pub async fn add_to_wishlist(
        &self,
        title: String,
        artist: String,
        album: Option<String>,
        external_track_id: Option<String>,
        notes: Option<String>,
    ) -> AppResult<WishlistItemRecord> {
        let now = Utc::now().timestamp();

        // Ensure referenced external_track_id exists in external_tracks table to satisfy FK
        if let Some(ref ext_id) = external_track_id {
            if self.repo.get_external_track(ext_id).await?.is_none() {
                let stub = ExternalTrackRecord {
                    id: ext_id.clone(),
                    provider: "custom".to_string(),
                    provider_id: ext_id.clone(),
                    title: title.clone(),
                    artist: artist.clone(),
                    album: album.clone(),
                    duration_secs: None,
                    cover_art_url: None,
                    preview_url: None,
                    genre: None,
                    match_status: "NOT_FOUND".to_string(),
                    matched_local_track_id: None,
                    created_at: now,
                };
                self.repo.upsert_external_track(&stub).await?;
            }
        }

        let item = WishlistItemRecord {
            id: format!("wl_{}", Uuid::new_v4()),
            title,
            artist,
            album,
            external_track_id,
            status: "WANT".to_string(),
            notes,
            created_at: now,
            updated_at: now,
        };

        self.repo.add_item(&item).await?;
        Ok(item)
    }

    /// Updates the status of an existing wishlist item.
    pub async fn update_status(&self, id: &str, status: &str) -> AppResult<()> {
        let norm_status = match status.to_uppercase().as_str() {
            "WANT" => "WANT",
            "IGNORE" => "IGNORE",
            "ALREADY_OWN" | "ALREADYOWN" => "ALREADY_OWN",
            "DOWNLOADED" => "DOWNLOADED",
            other => return Err(AppError::Validation(format!("Invalid wishlist status: {}", other))),
        };

        self.repo.update_status(id, norm_status).await
    }

    /// Convenience method to update status using strongly-typed WishlistStatus enum.
    pub async fn update_status_enum(&self, id: &str, status: WishlistStatus) -> AppResult<()> {
        let status_str = match status {
            WishlistStatus::Want => "WANT",
            WishlistStatus::Ignore => "IGNORE",
            WishlistStatus::AlreadyOwn => "ALREADY_OWN",
            WishlistStatus::Downloaded => "DOWNLOADED",
        };
        self.repo.update_status(id, status_str).await
    }

    /// Retrieves all wishlist items, optionally filtered by status.
    pub async fn get_wishlist(&self, status_filter: Option<&str>) -> AppResult<Vec<WishlistItemRecord>> {
        self.repo.get_all(status_filter).await
    }

    /// Retrieves a single wishlist item by ID.
    pub async fn get_by_id(&self, id: &str) -> AppResult<Option<WishlistItemRecord>> {
        self.repo.get_by_id(id).await
    }

    /// Deletes a wishlist item by ID.
    pub async fn delete_item(&self, id: &str) -> AppResult<()> {
        self.repo.delete_item(id).await
    }
}
