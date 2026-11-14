//! Routes every command and query to its domain handler file. The matches are
//! exhaustive: a new command or query must be assigned a domain here.

use super::*;

impl CoreProcessor {
    pub(super) async fn route_command(&self, cmd: Command) -> AppResult<CommandResponse> {
        match &cmd {
            Command::Pause
            | Command::Resume
            | Command::Stop
            | Command::TogglePlayPause
            | Command::NextTrack
            | Command::PreviousTrack
            | Command::Seek { .. }
            | Command::SetVolume { .. }
            | Command::ToggleMute
            | Command::SetRepeatMode { .. }
            | Command::SetShuffle { .. }
            | Command::PlayTrack { .. }
            | Command::PlayTrackList { .. }
            | Command::PlayQueueIndex { .. }
            | Command::PlayOnlineTrack { .. }
            | Command::EnqueueTrack { .. }
            | Command::EnqueueOnlineTrack { .. }
            | Command::DequeueTrack { .. }
            | Command::ClearQueue
            | Command::SetArchiveShuffle { .. }
            | Command::ClearRemoteAudioCache => self.playback_command(cmd).await,
            Command::AddLibraryFolder { .. }
            | Command::RemoveLibraryFolder { .. }
            | Command::ScanLibrary { .. }
            | Command::CancelScan
            | Command::TriggerMetadataRefresh { .. }
            | Command::SetMetadataPreference { .. }
            | Command::RemoveProviderEnrichment
            | Command::FillLibraryGaps { .. }
            | Command::CompleteOnboarding { .. }
            | Command::ResetOnboarding => self.library_command(cmd).await,
            Command::CreatePlaylist { .. }
            | Command::DeletePlaylist { .. }
            | Command::RenamePlaylist { .. }
            | Command::AddTrackToPlaylist { .. }
            | Command::RemoveTrackFromPlaylist { .. }
            | Command::EnsureLikedSongsPlaylist
            | Command::LikeTrack { .. }
            | Command::DislikeTrack { .. }
            | Command::RemoveTrackFeedback { .. } => self.collections_command(cmd).await,
            Command::DownloadTrack { .. }
            | Command::RemoveDownload { .. }
            | Command::StartDownload { .. }
            | Command::CancelDownload { .. }
            | Command::PollDownloadProgress { .. }
            | Command::SearchDownloads { .. }
            | Command::AddToWishlist { .. }
            | Command::AddMissingToWishlist { .. }
            | Command::UpdateWishlistStatus { .. } => self.downloads_command(cmd).await,
            Command::Login { .. } | Command::SignUp { .. } => self.account_command(cmd).await,
            Command::SetCloudServerUrl { .. } | Command::SyncCloudData => {
                self.cloud_command(cmd).await
            }
            Command::UploadAvatar { .. } | Command::RemoveAvatar => {
                self.profile_command(cmd).await
            }
            Command::Logout | Command::LogoutAll | Command::RevokeSession { .. } => {
                self.sessions_command(cmd).await
            }
            Command::SaveRecommendationSetup { .. }
            | Command::StartFlow { .. }
            | Command::StopFlow
            | Command::ContinueFlow
            | Command::SetFlowSteering { .. }
            | Command::GenerateSmartMix { .. }
            | Command::LessLikeThis { .. }
            | Command::BlockArtist { .. }
            | Command::ClearEntityFeedback { .. } => self.recommendations_command(cmd).await,
            Command::SetInterfaceStyle { .. } | Command::SetCloseButtonBehavior { .. } => {
                self.settings_command(cmd).await
            }
        }
    }

    pub(super) async fn route_query(&self, query: Query) -> AppResult<QueryResponse> {
        match &query {
            Query::GetPlaybackState
            | Query::GetTrackCoverArt { .. }
            | Query::GetTrackLyrics { .. }
            | Query::GetRemoteAudioCacheStats => self.playback_query(query).await,
            Query::GetTracks { .. }
            | Query::GetTrackById { .. }
            | Query::GetAlbums { .. }
            | Query::GetAlbumDetails { .. }
            | Query::GetArtists { .. }
            | Query::GetLibraryOverview
            | Query::GetLibraryTracksPage { .. }
            | Query::GetLibraryAlbumsPage { .. }
            | Query::SearchLibrary { .. }
            | Query::GetOnboardingStatus => self.library_query(query).await,
            Query::GetPlaylists
            | Query::GetPlaylistTracks { .. }
            | Query::GetTrackPlaylistMemberships
            | Query::GetSmartMixes
            | Query::RefreshSmartMixes => self.collections_query(query).await,
            Query::GetDownloads { .. } | Query::GetWishlist => {
                self.downloads_query(query).await
            }
            Query::GetCloudSyncStatus => self.cloud_query(query).await,
            Query::GetProfile | Query::GetAvatar { .. } => self.profile_query(query).await,
            Query::GetCurrentUser
            | Query::GetSessionState
            | Query::ListSessions => self.sessions_query(query).await,
            Query::GetDiscoveryRecommendations { .. }
            | Query::GetLocalRecommendations { .. }
            | Query::GetFlowSession
            | Query::GetFlowQueue { .. }
            | Query::GetFlowTrackReason { .. }
            | Query::GetRecommendationFeedback
            | Query::GetRecommendationSetup
            | Query::GetEntityFeedback
            | Query::GetTasteProfile
            | Query::GetHomeOverview => self.recommendations_query(query).await,
            Query::GetCharts
            | Query::GetChartSongs { .. }
            | Query::GetWorldTrending { .. }
            | Query::GetCachedWorldTrending { .. }
            | Query::RefreshCharts
            | Query::SearchOnlineMusic { .. }
            | Query::ImportJioSaavnPlaylist { .. } => self.discovery_query(query).await,
            Query::GetStatsOverview { .. } | Query::GetTopRankings { .. } => {
                self.stats_query(query).await
            }
            Query::GetSettings => self.settings_query(query).await,
        }
    }
}

/// A handler received a request that routing sends elsewhere (a bug).
pub(super) fn misrouted(request: &impl std::fmt::Debug) -> AppError {
    AppError::Internal(format!("Request routed to the wrong handler: {request:?}"))
}
