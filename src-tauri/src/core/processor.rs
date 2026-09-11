use crate::config::AppConfig;
use crate::core::command::{Command, CommandResponse};
use crate::core::error::{AppError, AppResult};
use crate::core::event::Event;
use crate::core::event_bus::EventBus;
use crate::core::query::{Query, QueryResponse};
use crate::database::repositories::{LibraryFolderRepository, SqliteFolderRepository};
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Central processing coordinator for commands, queries, and event dispatching.
pub struct CoreProcessor {
    event_bus: Arc<EventBus>,
    db_pool: SqlitePool,
    folder_repo: Arc<dyn LibraryFolderRepository>,
    config: Arc<RwLock<AppConfig>>,
}

impl CoreProcessor {
    /// Constructs a new CoreProcessor instance with the supplied database pool and configuration.
    pub fn new(db_pool: SqlitePool, config: AppConfig) -> Self {
        let event_bus = Arc::new(EventBus::default());
        let folder_repo = Arc::new(SqliteFolderRepository::new(db_pool.clone()));

        Self {
            event_bus,
            db_pool,
            folder_repo,
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

    /// Dispatches and executes an incoming Command, emitting events and returning the result.
    pub async fn dispatch_command(&self, cmd: Command) -> AppResult<CommandResponse> {
        info!(?cmd, "Dispatching command");

        match cmd {
            Command::AddLibraryFolder { path } => {
                if path.trim().is_empty() {
                    return Err(AppError::Validation("Folder path cannot be empty".into()));
                }
                let folder = self.folder_repo.add_folder(&path).await?;
                info!(folder_id = %folder.id, path = %folder.path, "Added music library folder");
                Ok(CommandResponse::EntityId(folder.id))
            }
            Command::RemoveLibraryFolder { folder_id } => {
                self.folder_repo.remove_folder(&folder_id).await?;
                info!(%folder_id, "Removed music library folder");
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
                // Future PlaybackService integration placeholder
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
                warn!(?cmd, "Command handler routed to stub during Phase 1");
                Ok(CommandResponse::Ok)
            }
        }
    }

    /// Executes a read-only Query and returns the typed QueryResponse.
    pub async fn execute_query(&self, query: Query) -> AppResult<QueryResponse> {
        info!(?query, "Executing query");

        match query {
            Query::GetSettings => {
                let cfg = self.config.read().await;
                let val = serde_json::to_value(&*cfg)
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                Ok(QueryResponse::Settings(val))
            }
            Query::GetLibraryOverview => {
                let folders = self.folder_repo.get_all().await?;
                let val = serde_json::to_value(&folders)
                    .map_err(|e| AppError::Internal(e.to_string()))?;
                Ok(QueryResponse::Tracks(vec![val]))
            }
            _ => {
                warn!(?query, "Query handler routed to stub during Phase 1");
                Ok(QueryResponse::Empty)
            }
        }
    }
}
