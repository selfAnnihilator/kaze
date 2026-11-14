import type { ReactNode } from "react";
import type { ViewType } from "../../../viewTypes";
import type { AppViewsContext } from "./context";
import { HomeView } from "../../views/HomeView";
import { LibraryView } from "../../views/LibraryView";
import { AlbumsView } from "../../views/AlbumsView";
import { PlaylistsView } from "../../views/PlaylistsView";

/** Home and library views, by route. */
export const LIBRARY_VIEWS: Partial<Record<ViewType, (c: AppViewsContext) => ReactNode>> = {
  home: (c) =>
    (
        <HomeView
          overview={c.homeOverview}
          isLoading={c.isHomeLoading}
          flowPendingAction={c.flowPendingAction}
          onRefresh={c.fetchHomeOverview}
          onStartFlow={c.handleStartFlow}
          onTogglePlayback={c.handleUnifiedPlayPause}
          onToggleFlowPlayback={c.handleToggleFlowPlayback}
          currentTrack={c.playbackState.current_track}
          currentCover={c.activePlayingArtwork}
          previousTrack={c.previousTrackRef.current}
          nextTrack={c.nextQueuedTrack}
          onPreviousTrack={c.handlePreviousTrack}
          onNextTrack={c.handleNextTrack}
          onPlayTrack={c.handlePlayTrack}
          onEnqueueTrack={c.handleEnqueueTrack}
          onOpenAlbum={c.handleSelectAlbum}
          loadingTrackId={c.loadingTrackId}
          isPlaying={c.playbackState.is_playing}
          flowSteering={c.flowSteering}
          onFlowSteeringChange={c.handleFlowSteeringChange}
          onDiscover={c.handleHomeDiscover}
          queuedTrackIds={c.queuedTrackIds}
        />
      ),
  library: (c) =>
    (
        <LibraryView
          tracks={c.tracks}
          total={c.libraryScoped ? c.tracks.length : c.trackPage.total}
          onEndReached={c.libraryScoped ? undefined : c.trackPage.next}
          loading={c.libraryScoped ? c.scoped.loading : c.trackPage.loading}
          pagingError={c.libraryScoped ? c.scoped.error : c.trackPage.error}
          onRetry={c.libraryScoped ? c.scoped.refresh : c.trackPage.retry}
          currentTrackId={c.playbackState.current_track?.id}
          isPlaying={c.playbackState.is_playing}
          queuedTrackIds={c.queuedTrackIds}
          onPlayTrack={c.handleLibraryRowPlay}
          onPlayAll={c.handleLibraryPlayAll}
          listState={c.downloadsListActive ? (c.playbackState.is_playing ? "playing" : "paused") : "idle"}
          shuffleEnabled={c.libraryShuffle}
          onToggleShuffle={c.handleLibraryToggleShuffle}
          loadingTrackId={c.loadingTrackId}
          onEnqueueTrack={c.handleEnqueueTrack}
          onLikeTrack={c.handleLike}
          onRemoveFeedback={c.handleRemoveFeedback}
          onRescan={c.handleRescanLibrary}
          onSearch={c.handleSearchLibrary}
          searchQuery={c.destination.libraryQuery}
          trackPlaylistMap={c.regularTrackPlaylistMap}
          likedTrackIds={c.likedTrackIds}
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
  albums: (c) =>
    (
        <AlbumsView
          albums={c.albums}
          total={c.albumPage.total}
          onEndReached={c.albumPage.next}
          loading={c.albumPage.loading}
          pagingError={c.albumPage.error}
          onRetry={c.albumPage.retry}
          onSelectAlbum={c.handleSelectAlbum}
        />
      ),
  playlists: (c) =>
    (
        <PlaylistsView
          viewMode="playlists"
          playlists={c.playlists}
          currentUser={c.currentUser}
          onOpenAuthModal={() => c.setIsAuthModalOpen(true)}
          activePlaylistId={c.playingPlaylistId}
          onPlayPlaylist={c.handlePlayPlaylist}
          onCreatePlaylist={c.handleCreatePlaylist}
          onRefreshPlaylists={c.handleRefreshPlaylists}
          onInspectJioSaavnPlaylist={c.handleInspectJioSaavnPlaylist}
          onSaveImportedPlaylist={c.handleSaveImportedPlaylist}
          onAddMissingToWishlist={c.handleAddMissingToWishlist}
          onSearchDirect={c.handleInitiateDirectDownloadSearch}
          onFetchPlaylistTracks={c.handleFetchPlaylistTracks}
          onPlayTrack={c.handlePlayTrack}
          loadingTrackId={c.loadingTrackId}
          queuedTrackIds={c.queuedTrackIds}
          onEnqueueTrack={c.handleEnqueueTrack}
          onOpenCollection={c.handleOpenCollection}
          onRenamePlaylist={c.handleRenamePlaylist}
          onDeletePlaylist={c.handleDeletePlaylist}
        />
      ),
  smart_mixes: (c) =>
    (
        <PlaylistsView
          viewMode="smart_mixes"
          playlists={c.playlists}
          currentUser={c.currentUser}
          onOpenAuthModal={() => c.setIsAuthModalOpen(true)}
          activePlaylistId={c.playingPlaylistId}
          onPlayPlaylist={c.handlePlayPlaylist}
          onCreatePlaylist={c.handleCreatePlaylist}
          onRefreshPlaylists={c.handleRefreshPlaylists}
          onInspectJioSaavnPlaylist={c.handleInspectJioSaavnPlaylist}
          onSaveImportedPlaylist={c.handleSaveImportedPlaylist}
          onAddMissingToWishlist={c.handleAddMissingToWishlist}
          onSearchDirect={c.handleInitiateDirectDownloadSearch}
          onFetchPlaylistTracks={c.handleFetchPlaylistTracks}
          onPlayTrack={c.handlePlayTrack}
          loadingTrackId={c.loadingTrackId}
          queuedTrackIds={c.queuedTrackIds}
          onEnqueueTrack={c.handleEnqueueTrack}
          onOpenCollection={c.handleOpenCollection}
          onRenamePlaylist={c.handleRenamePlaylist}
          onDeletePlaylist={c.handleDeletePlaylist}
        />
      ),
};
