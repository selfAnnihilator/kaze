use crate::core::error::{AppError, AppResult};
use crate::core::event_bus::EventBus;
use crate::database::models::LibraryFolderRecord;
use crate::database::repositories::{
    AlbumRepository, ArtistRepository, LibraryFolderRepository, SettingsRepository,
    SqliteAlbumRepository, SqliteArtistRepository, SqliteFolderRepository,
    SqliteSettingsRepository, SqliteTrackRepository, TrackDetail, TrackRepository,
};
use crate::library::scanner::{LibraryScanner, ScanSummary};
use directories::UserDirs;
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

pub struct LibraryService {
    folder_repo: Arc<dyn LibraryFolderRepository>,
    track_repo: Arc<dyn TrackRepository>,
    artist_repo: Arc<dyn ArtistRepository>,
    album_repo: Arc<dyn AlbumRepository>,
    settings_repo: Arc<dyn SettingsRepository>,
    scanner: Arc<LibraryScanner>,
    event_bus: Arc<EventBus>,
}

impl LibraryService {
    pub fn new(pool: SqlitePool, event_bus: Arc<EventBus>) -> Self {
        let folder_repo = Arc::new(SqliteFolderRepository::new(pool.clone()));
        let track_repo = Arc::new(SqliteTrackRepository::new(pool.clone()));
        let artist_repo = Arc::new(SqliteArtistRepository::new(pool.clone()));
        let album_repo = Arc::new(SqliteAlbumRepository::new(pool.clone()));
        let settings_repo = Arc::new(SqliteSettingsRepository::new(pool.clone()));

        let scanner = Arc::new(LibraryScanner::new(
            track_repo.clone(),
            folder_repo.clone(),
            event_bus.clone(),
        ));

        Self {
            folder_repo,
            track_repo,
            artist_repo,
            album_repo,
            settings_repo,
            scanner,
            event_bus,
        }
    }

    /// Discovers the system's default audio/music folder path.
    pub fn get_default_music_dir() -> PathBuf {
        if let Some(user_dirs) = UserDirs::new() {
            if let Some(audio_dir) = user_dirs.audio_dir() {
                return audio_dir.to_path_buf();
            }
            return user_dirs.home_dir().join("Music");
        }
        PathBuf::from("./Music")
    }

    /// Checks if onboarding has been completed.
    pub async fn is_onboarding_completed(&self) -> AppResult<bool> {
        let val = self
            .settings_repo
            .get_setting("onboarding_completed")
            .await?;
        Ok(val.map(|v| v == "true").unwrap_or(false))
    }

    /// Completes the onboarding procedure by registering user approved music directories.
    pub async fn complete_onboarding(
        &self,
        chosen_folders: Vec<String>,
        start_scan: bool,
    ) -> AppResult<usize> {
        let mut folders_to_add = chosen_folders;
        if folders_to_add.is_empty() {
            let default_dir = Self::get_default_music_dir();
            folders_to_add.push(default_dir.to_string_lossy().to_string());
        }

        let mut count = 0;
        for folder_path in &folders_to_add {
            let path = PathBuf::from(folder_path);
            // Ensure folder exists or can be accessed
            if path.exists() && path.is_dir() {
                let _ = self.folder_repo.add_folder(folder_path).await?;
                count += 1;
            }
        }

        self.settings_repo
            .set_setting("onboarding_completed", "true")
            .await?;

        info!(
            folders_configured = count,
            start_scan = start_scan,
            "Onboarding completed successfully"
        );

        if start_scan && count > 0 {
            let scanner = self.scanner.clone();
            let folder_repo = self.folder_repo.clone();
            tokio::spawn(async move {
                if let Ok(folders) = folder_repo.get_all().await {
                    for f in folders {
                        let _ = scanner.scan_folder(&f.id, &f.path, false).await;
                    }
                }
            });
        }

        Ok(count)
    }

    /// Registers an additional music folder to the library.
    pub async fn add_folder(&self, path: &str) -> AppResult<LibraryFolderRecord> {
        let p = PathBuf::from(path);
        if !p.exists() || !p.is_dir() {
            return Err(AppError::Validation(format!(
                "Path does not exist or is not a directory: {}",
                path
            )));
        }
        self.folder_repo.add_folder(path).await
    }

    /// Removes a registered folder.
    pub async fn remove_folder(&self, folder_id: &str) -> AppResult<()> {
        self.folder_repo.remove_folder(folder_id).await
    }

    /// Returns all registered music library folders.
    pub async fn get_folders(&self) -> AppResult<Vec<LibraryFolderRecord>> {
        self.folder_repo.get_all().await
    }

    /// Scans a specific folder or all registered folders.
    pub async fn scan_library(
        &self,
        folder_id: Option<String>,
        incremental: bool,
    ) -> AppResult<Vec<ScanSummary>> {
        let folders = if let Some(id) = folder_id {
            let all = self.folder_repo.get_all().await?;
            all.into_iter().filter(|f| f.id == id).collect()
        } else {
            self.folder_repo.get_all().await?
        };

        let mut results = Vec::new();
        for folder in folders {
            let summary = self
                .scanner
                .scan_folder(&folder.id, &folder.path, incremental)
                .await?;
            results.push(summary);
        }

        Ok(results)
    }

    /// Cancels any active scan.
    pub fn cancel_scan(&self) {
        self.scanner.cancel();
    }

    /// Search across tracks via FTS5.
    pub async fn search(&self, query: &str, limit: u32) -> AppResult<Vec<TrackDetail>> {
        self.track_repo.search_tracks(query, limit).await
    }

    pub fn event_bus(&self) -> Arc<EventBus> {
        self.event_bus.clone()
    }

    pub fn track_repo(&self) -> Arc<dyn TrackRepository> {
        self.track_repo.clone()
    }

    pub fn artist_repo(&self) -> Arc<dyn ArtistRepository> {
        self.artist_repo.clone()
    }

    pub fn album_repo(&self) -> Arc<dyn AlbumRepository> {
        self.album_repo.clone()
    }
}
