mod artwork;
mod routing;
mod cloud;
mod profile;
mod sessions;
mod construction;
mod lifecycle;
mod feedback;
mod account;
mod collections;
mod discovery;
mod downloads;
mod library;
mod playback;
mod recommendations;
mod settings;
mod stats;
mod live_sync;

use crate::recommendations::mixes::OnlineTrackInput;
use crate::database::repositories::user_repo::AvatarFields;
use crate::core::sync::MutexExt;
use crate::cloud::{
    credentials, device, AuthSessionState, CloudClient, CloudSessionMetadata, CloudSyncStatus,
    CloudUser, SessionExpiredReason, SyncManager,
};
use crate::config::AppConfig;
use crate::core::command::{Command, CommandResponse};
use crate::core::error::{AppError, AppResult};
use crate::core::event::Event;
use crate::core::event_bus::EventBus;
use crate::core::query::{Query, QueryResponse};
use crate::database::models::PlaylistRecord;
use crate::database::repositories::LibraryScope;
use crate::database::repositories::{
    PlaylistRepository, SettingsRepository, SqliteDownloadRepository, SqliteHistoryRepository,
    SqlitePlaylistRepository, SqliteSettingsRepository, SqliteStatsRepository,
    SqliteUserRepository, SqliteWishlistRepository, StatsRepository, UserProfile, UserRepository,
};
use crate::discovery::{DiscoveryCoordinator, WishlistManager};
use crate::downloads::{DownloadProvider, DownloadService, JioSaavnDownloadProvider};
use crate::history::HistoryService;
use crate::library::LibraryService;
use crate::playback::backend::{AudioBackend, RodioAudioBackend};
use crate::playback::PlaybackService;
use crate::profile::ProfileService;
use crate::providers::ProviderCoordinator;
use crate::ranking::RankingEngine;
use crate::recommendations::{LocalRecommender, SmartMixGenerator, TasteProfileEngine};
use lofty::file::TaggedFileExt;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::{Mutex, OnceCell, RwLock};
use tracing::info;

/// Central processing coordinator for commands, queries, and event dispatching.
pub struct CoreProcessor {
    event_bus: Arc<EventBus>,
    db_pool: SqlitePool,
    library_service: Arc<LibraryService>,
    playback_service: Arc<PlaybackService>,
    history_service: Arc<HistoryService>,
    ranking_engine: Arc<RankingEngine>,
    playlist_repo: Arc<SqlitePlaylistRepository>,
    stats_repo: Arc<SqliteStatsRepository>,
    user_repo: Arc<SqliteUserRepository>,
    cloud_client: Arc<CloudClient>,
    settings_repo: Arc<SqliteSettingsRepository>,
    pub profile_service: Arc<ProfileService>,
    pub current_user: Arc<RwLock<Option<UserProfile>>>,
    pub session_state: Arc<RwLock<AuthSessionState>>,
    app_start_date: Arc<RwLock<i64>>,
    taste_engine: Arc<TasteProfileEngine>,
    recommender: Arc<LocalRecommender>,
    smart_mix_generator: Arc<SmartMixGenerator>,
    provider_coordinator: Arc<ProviderCoordinator>,
    gap_enrichment: Arc<crate::providers::enrichment::GapEnrichmentService>,
    similarity_service: Arc<crate::providers::similarity_worker::SimilarityService>,
    wishlist_manager: Arc<WishlistManager>,
    discovery_coordinator: Arc<DiscoveryCoordinator>,
    download_service: Arc<DownloadService>,
    flow_engine: Arc<crate::recommendations::FlowEngine>,
    online_catalog: Arc<crate::providers::online_catalog::OnlineCatalog>,
    config: Arc<RwLock<AppConfig>>,
    cover_art_cache: Arc<tokio::sync::RwLock<HashMap<String, artwork::ResolvedArt>>>,
    is_quitting: AtomicBool,
    close_button_quits: AtomicBool,
    close_behavior_ready: OnceCell<()>,
    close_behavior_updates: Mutex<()>,
    command_gate: RwLock<()>,
    shutdown_done: Mutex<bool>,
    background_tasks: StdMutex<Vec<tokio::task::JoinHandle<()>>>,
}

impl CoreProcessor {

    pub async fn current_user_id(&self) -> String {
        let current_user_guard = self.current_user.read().await;
        current_user_guard
            .as_ref()
            .map(|u| u.id.clone())
            .unwrap_or_else(|| "default".to_string())
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

    /// Access the PlaybackService handle.
    pub fn playback_service(&self) -> Arc<PlaybackService> {
        self.playback_service.clone()
    }

    /// Access the HistoryService handle.
    pub fn history_service(&self) -> Arc<HistoryService> {
        self.history_service.clone()
    }

    /// Access the RankingEngine handle.
    pub fn ranking_engine(&self) -> Arc<RankingEngine> {
        self.ranking_engine.clone()
    }

    /// Access the PlaylistRepository handle.
    pub fn playlist_repo(&self) -> Arc<SqlitePlaylistRepository> {
        self.playlist_repo.clone()
    }

    /// Access the TasteProfileEngine handle.
    pub fn taste_engine(&self) -> Arc<TasteProfileEngine> {
        self.taste_engine.clone()
    }

    /// Access the LocalRecommender handle.
    pub fn recommender(&self) -> Arc<LocalRecommender> {
        self.recommender.clone()
    }

    /// Access the SmartMixGenerator handle.
    pub fn smart_mix_generator(&self) -> Arc<SmartMixGenerator> {
        self.smart_mix_generator.clone()
    }

    /// Access the FlowEngine handle.
    pub fn flow_engine(&self) -> Arc<crate::recommendations::FlowEngine> {
        self.flow_engine.clone()
    }

    /// Access the ProviderCoordinator handle.
    pub fn provider_coordinator(&self) -> Arc<ProviderCoordinator> {
        self.provider_coordinator.clone()
    }

    /// Access the WishlistManager handle.
    pub fn wishlist_manager(&self) -> Arc<WishlistManager> {
        self.wishlist_manager.clone()
    }

    /// Access the DiscoveryCoordinator handle.
    pub fn discovery_coordinator(&self) -> Arc<DiscoveryCoordinator> {
        self.discovery_coordinator.clone()
    }

    /// Access the DownloadService handle.
    pub fn download_service(&self) -> Arc<DownloadService> {
        self.download_service.clone()
    }

    /// Dispatches and executes an incoming Command, emitting events and returning the result.
    pub async fn dispatch_command(&self, cmd: Command) -> AppResult<CommandResponse> {
        let _commands = self.command_gate.read().await;
        if self.is_quitting() {
            return Err(AppError::Validation("Kaze is shutting down".into()));
        }
        self.playback_service.initialize_saved_playback().await;
        info!(?cmd, "Dispatching command");

        self.route_command(cmd).await
    }

    /// Returns query data with online sources bound before songs are exposed.
    pub async fn execute_query(&self, query: Query) -> AppResult<QueryResponse> {
        info!(?query, "Executing query");

        self.route_query(query).await
    }

}


#[cfg(test)]
mod tests;
