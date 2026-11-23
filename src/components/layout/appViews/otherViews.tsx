import type { ReactNode } from "react";
import type { ViewType } from "../../../viewTypes";
import type { AppViewsContext } from "./context";
import { LibraryView } from "../../views/LibraryView";
import { SettingsView } from "../../views/SettingsView";
import { BrutalistPoster } from "../../../presentation/BrutalistPoster";
import { PlaybackStage } from "../../../presentation/PlaybackStage";
import { QueueView } from "../../../presentation/QueueView";
import { viewDestination } from "../../../navigationHistory";
import { DiscoveryView, NotificationsView, StatsView, WishlistView } from "./lazyViews";

/** Discovery, playback, downloads, profile and settings views, by route. */
export const OTHER_VIEWS: Partial<Record<ViewType, (c: AppViewsContext) => ReactNode>> = {
  discovery: (c) =>
    (
        <DiscoveryView
          recommendations={c.discoveryRecs}
          worldTrending={c.worldTrending}
          localTracks={c.tracks}
          playlists={c.playlists}
          onPlayPlaylist={c.handlePlayPlaylist}
          onAddToWishlist={(rec) =>
            c.handleAddToWishlist(rec.title, rec.artist, rec.album)
          }
          onSearchDirect={(artist, title) =>
            c.handleInitiateDirectDownloadSearch(artist, title)
          }
          isTrendingLoading={c.isTrendingLoading}
          trendingError={c.trendingError}
          onRefreshTrending={c.refreshTrending}
          onRefresh={c.refreshPersonalRecommendations}
          onPlayOnlineTrack={c.handlePlayOnlineTrack}
          onStopTrack={c.handleStopTrack}
          activeOnlineTrackId={
            c.playbackState.current_track?.id || null
          }
          isOnlinePlaying={c.playbackState.is_playing}
          loadingTrackId={c.loadingTrackId}
          currentLocalTrack={c.playbackState.current_track}
          isLocalPlaying={c.playbackState.is_playing}
          onOpenCollection={c.handleOpenCollection}
          onPlayCollection={c.handlePlayDiscoveredCollection}
          downloads={c.downloads}
          searchQuery={c.globalSearchQuery}
          setSearchQuery={c.setGlobalSearchQuery}
          searchResults={c.currentSearchResults}
          setSearchResults={c.setGlobalSearchResults}
          isSearchingOnline={c.isGlobalSearching}
          onSearchOnline={c.handleGlobalOnlineSearch}
          onClearSearch={c.handleGlobalClearSearch}
          trackPlaylistMap={c.regularTrackPlaylistMap}
          onAddToPlaylist={(rec) =>
            c.handleOpenAddToPlaylistModal({
              id:
                rec.matched_local_track_id &&
                !rec.matched_local_track_id.startsWith("online:") &&
                !rec.matched_local_track_id.startsWith("itunes:")
                  ? rec.matched_local_track_id
                  : rec.external_track_id,
              title: rec.title,
              artist: rec.artist,
              album: rec.album,
              cover_art_url: rec.cover_art_url,
            })
          }
          onGoToLibrary={() => c.navigate(viewDestination("library"))}
          queuedTrackIds={c.queuedTrackIds}
          onEnqueueTrack={c.handleEnqueueRecommendation}
          likedTrackIds={c.likedTrackIds}
        />
      ),
  now_playing: (c) =>
    (c.interfacePreference.style === "brutalist" ? <BrutalistPoster albumId={c.albums.find(a => a.title === c.playbackState.current_track?.album_title && a.artist_name === c.playbackState.current_track?.artist_name)?.id} track={c.playbackState.current_track} cover={c.activePlayingArtwork} index={c.playbackState.current_queue_index} playing={c.playbackState.is_playing} loading={c.loadingTrackId !== null || c.playbackState.is_buffering} entries={c.queueEntries} tracks={c.tracks} onPlayIndex={c.handlePlayQueueIndex} onOpenAlbum={c.handleSelectAlbum} /> :
        <PlaybackStage
          track={c.playbackState.current_track}
          cover={c.activePlayingArtwork}
          previous={c.previousTrack}
          next={c.nextQueuedTrack}
          index={c.playbackState.current_queue_index}
          playing={c.playbackState.is_playing}
          loading={
            c.loadingTrackId !== null || c.playbackState.is_buffering
          }
          onPrevious={c.handlePreviousTrack}
          onNext={c.handleNextTrack}
        />
      ),
  queue: (c) =>
    (
        <QueueView
          entries={c.queueEntries}
          tracks={c.tracks}
          currentTrack={c.playbackState.current_track}
          onPlayIndex={c.handlePlayQueueIndex}
          onRemove={c.handleDequeueTrack}
          onClear={c.handleClearQueue}
        />
      ),
  // Same list as Local, for the songs Kaze saved in its own folder.
  downloads: (c) =>
    (
        <LibraryView
          tracks={c.downloadsPage.items}
          total={c.downloadsPage.total}
          onEndReached={c.downloadsPage.next}
          loading={c.downloadsPage.loading}
          pagingError={c.downloadsPage.error}
          onRetry={c.downloadsPage.retry}
          currentTrackId={c.playbackState.current_track?.id}
          isPlaying={c.playbackState.is_playing}
          queuedTrackIds={c.queuedTrackIds}
          onPlayTrack={c.kazeDownloadsPlayback.handleLibraryRowPlay}
          onPlayAll={c.kazeDownloadsPlayback.handleLibraryPlayAll}
          listState={c.downloadsListState}
          shuffleEnabled={c.kazeDownloadsPlayback.libraryShuffle}
          onToggleShuffle={c.kazeDownloadsPlayback.handleLibraryToggleShuffle}
          loadingTrackId={c.loadingTrackId}
          onEnqueueTrack={c.handleEnqueueTrack}
          onLikeTrack={c.handleLike}
          onRemoveFeedback={c.handleRemoveFeedback}
          onRescan={() => void c.fetchDownloads()}
          onSearch={c.handleSearchLibrary}
          searchQuery={c.destination.libraryQuery}
          trackPlaylistMap={c.regularTrackPlaylistMap}
          likedTrackIds={c.likedTrackIds}
          downloadControls
          emptyTitle="No downloaded songs yet"
          emptyHint="Use the download button on any online song. It is saved to Kaze's folder and appears here."
          onOpenAddToPlaylistModal={(t) =>
            c.handleOpenAddToPlaylistModal({
              id: t.id,
              title: t.title,
              artist: t.artist_name,
              album: t.album_title,
            })
          }
        />
      ),
  wishlist: (c) =>
    (
        <WishlistView
          wishlist={c.wishlist}
          onAddToWishlist={c.handleAddToWishlist}
          onUpdateStatus={c.handleUpdateWishlistStatus}
          onSearchDirect={c.handleInitiateDirectDownloadSearch}
        />
      ),
  notifications: (c) =>
    (
        <NotificationsView
          notifications={c.notifications}
          onClearAll={c.clearAllNotifications}
          onRemoveNotification={c.deleteNotification}
          onMarkAllRead={c.markAllNotificationsAsRead}
          onMarkAsRead={c.markNotificationAsRead}
        />
      ),
  stats: (c) =>
    (
        <StatsView
          currentUser={c.currentUser}
          onPlayTrack={c.handlePlayTrack}
          onOpenAuthModal={() => c.setIsAuthModalOpen(true)}
          fetchStatsOverview={c.fetchStatsOverview}
          refreshTrigger={c.statsRefreshTrigger}
          onUpdateUser={(u) => c.setCurrentUser(u)}
        />
      ),
  settings: (c) =>
    (
        <SettingsView
          onRecommendationSetupSaved={c.handleTasteSaved}
          settings={c.settings}
          currentUser={c.currentUser}
          onLogout={c.handleLogout}
          onOpenAuthModal={() => c.setIsAuthModalOpen(true)}
          interfaceStyle={c.interfacePreference.style}
          onSelectInterfaceStyle={c.interfacePreference.select}
          isSavingInterfaceStyle={c.interfacePreference.saving}
          interfaceStyleError={c.interfacePreference.error}
          configuredFolders={
            c.onboardingStatus?.configured_folders || []
          }
          onAddFolder={c.handleAddFolder}
          onRemoveFolder={c.handleRemoveFolder}
          onRescanLibrary={c.handleRescanLibrary}
          onReloadSettings={c.fetchSettings}
          isScanning={c.isScanning}
        />
      ),
};
