use crate::config::AppConfig;
use crate::core::command::{Command, CommandResponse};
use crate::core::error::{AppError, AppResult};
use crate::core::event::Event;
use crate::core::event_bus::EventBus;
use crate::core::query::{Query, QueryResponse};
use crate::library::LibraryService;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Central processing coordinator for commands, queries, and event dispatching.
pub struct CoreProcessor {
    event_bus: Arc<EventBus>,
    db_pool: SqlitePool,
    library_service: Arc<LibraryService>,
    config: Arc<RwLock<AppConfig>>,
}

impl CoreProcessor {
    /// Constructs a new CoreProcessor instance with the supplied database pool and configuration.
    pub fn new(db_pool: SqlitePool, config: AppConfig) -> Self {
        let event_bus = Arc::new(EventBus::default());
        let library_service = Arc::new(LibraryService::new(db_pool.clone(), event_bus.clone()));

        Self {
            event_bus,
            db_pool,
            library_service,
            config: Arc::new(RwLock::new(config)),
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

    /// Dispatches and executes an incoming Command, emitting events and returning the result.
    pub async fn dispatch_command(&self, cmd: Command) -> AppResult<CommandResponse> {
        info!(?cmd, "Dispatching command");

        match cmd {
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
                info!(total_added = total_added, "Library scan execution finished");
                Ok(CommandResponse::Ok)
            }
            Command::CancelScan => {
                self.library_service.cancel_scan();
                info!("Library scan cancel requested");
                Ok(CommandResponse::Ok)
            }
            Command::SetVolume { volume } => {
                if !(0.0..=1.0).contains(&volume) {
                    return Err(AppError::Validation("Volume must be between 0.0 and 1.0".into()));
                }
                self.event_bus.publish(Event::PlaybackVolumeChanged {
                    volume,
                    is_muted: volume == 0.0,
                })?;
                Ok(CommandResponse::Ok)
            }
            Command::Pause => {
                self.event_bus.publish(Event::PlaybackPaused {
                    track_id: "placeholder".into(),
                    position_secs: 0.0,
                })?;
                Ok(CommandResponse::Ok)
            }
            Command::Stop => {
                self.event_bus.publish(Event::PlaybackStopped)?;
                Ok(CommandResponse::Ok)
            }
            _ => {
                warn!(?cmd, "Command handler routed to stub during Phase 2");
                Ok(CommandResponse::Ok)
            }
        }
    }

    /// Executes a read-only Query and returns the typed QueryResponse.
    pub async fn execute_query(&self, query: Query) -> AppResult<QueryResponse> {
        info!(?query, "Executing query");

        match query {
            Query::GetOnboardingStatus => {
                let completed = self.library_service.is_onboarding_completed().await?;
                let default_music_dir = LibraryService::get_default_music_dir()
                    .to_string_lossy()
                    .to_string();
                let folders = self.library_service.get_folders().await?;
                let folders_val = serde_json::to_value(&folders)
                    .map_err(|e| AppError::Internal(e.to_string()))?
                    .as_array()
                    .cloned()
                    .unwrap_or_default();

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
            _ => {
                warn!(?query, "Query handler routed to stub during Phase 2");
                Ok(QueryResponse::Empty)
            }
        }
    }
}
