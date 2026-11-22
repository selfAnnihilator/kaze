import { PlaybackNotice } from "./components/PlaybackNotice";
import { RecommendationNotice, type RecommendationNoticeValue } from "./components/RecommendationNotice";
import { RecommendationSetupGate } from "./components/MusicTaste";
import "./App.css";
import { useRouteScroll } from "./presentation/useRouteScroll";
import { ViewMemoryProvider } from "./presentation/ViewMemory";
import { useVisualActivity } from "./presentation/useVisualActivity";
import { usePointerFocus } from "./presentation/usePointerFocus";
import { useHorizontalDrag } from "./presentation/useHorizontalDrag";
import React, { useState, useEffect, useCallback, useRef, useMemo, Suspense, lazy } from "react";
import type { AppSettings } from "./types";
import { dispatchCommand } from "./services/api";
import { useNavigationHistory } from "./hooks/useNavigationHistory";
import { SpatialNavigation } from "./presentation/SpatialNavigation";
import { NowPlayingBar } from "./components/NowPlayingBar";
import { OnboardingModal } from "./components/OnboardingModal";
import { WhyTrackModal } from "./components/modals/WhyTrackModal";
import { AppToolbar } from "./components/layout/AppToolbar";
import { ToastContainer } from "./components/notifications/ToastContainer";
import { UpdateBanner } from "./components/UpdateBanner";
import { ConfirmModal } from "./components/modals/ConfirmModal";
import { useWindowShortcuts } from "./hooks/app/useWindowShortcuts";
import { useLoadAll } from "./hooks/app/useLoadAll";
import { useAccountSwitch } from "./hooks/app/useAccountSwitch";
import { useAuth } from "./hooks/app/useAuth";
import { useLibrary } from "./hooks/app/useLibrary";
import { useDownloadsSection } from "./hooks/app/useDownloadsSection";
import { useBackendEvents } from "./hooks/app/useBackendEvents";
import { useLikes } from "./hooks/app/useLikes";
import { AppViews } from "./components/layout/AppViews";
import { usePlaylists } from "./hooks/app/usePlaylists";
import { useWishlist } from "./hooks/app/useWishlist";
import { useDownloads } from "./hooks/app/useDownloads";
import { useDiscoveryFeeds } from "./hooks/app/useDiscoveryFeeds";
import { usePlaybackControls } from "./hooks/app/usePlaybackControls";
import { useFlow } from "./hooks/app/useFlow";
import { useCollections } from "./hooks/app/useCollections";
import { useQueue } from "./hooks/app/useQueue";
import type { CollectionData } from "./components/views/CollectionDetailView";
import { InterfaceContext } from "./presentation/InterfaceContext";
import { useInterfacePreference } from "./presentation/useInterfacePreference";
import type { PlaybackArtworkTrack } from "./presentation/queuePresentation";
import { linkableOrigin, type PlayingOrigin } from "./playingOrigin";
import { usePlaybackState } from "./hooks/app/usePlaybackState";
import { useNotifications } from "./hooks/app/useNotifications";
import { usePromptDialog } from "./hooks/app/usePromptDialog";
import { useGlobalSearch } from "./hooks/app/useGlobalSearch";
import { useAppNavigation, archiveSurface, findLikedSongs } from "./hooks/app/useAppNavigation";
import { useNowPlayingArtwork, usePlaylistMembership } from "./hooks/app/useNowPlayingDetails";
import "./presentation/interfaces.css";
import "./presentation/spatial.css";
import "./presentation/familiarity-dial.css";
import "./presentation/spatial-preview.css";
import "./presentation/brutalist.css";
import "./presentation/responsive.css";
import { BrutalistNavigation } from "./presentation/BrutalistNavigation";
import { BrutalistDeck } from "./presentation/BrutalistDeck";
import { MobileApp } from "./presentation/mobile/MobileApp";

// Lazy-loaded secondary views and modals to reduce initial memory and JS bundle overhead
const FullScreenPlayerView = lazy(() => import("./components/views/FullScreenPlayerView").then((m) => ({ default: m.FullScreenPlayerView,
  })),
);
const AuthModal = lazy(() => import("./components/modals/AuthModal").then((m) => ({ default: m.AuthModal,
  })),
);
const AddToPlaylistModal = lazy(() => import("./components/modals/AddToPlaylistModal").then((m) => ({ default: m.AddToPlaylistModal,
  })),
);

export const App: React.FC<{ initialSettings: AppSettings }> = ({ initialSettings }) => {
  // Navigation
  const { destination, navigate, reset: resetNavigation, updateDestination, back: navigateBack,
    forward: navigateForward, canGoBack, canGoForward,
  } = useNavigationHistory();
  const mainScrollRef = useRef<HTMLElement>(null);
  useRouteScroll(mainScrollRef, destination);
  useVisualActivity(mainScrollRef);
  usePointerFocus();
  useHorizontalDrag();
  const currentView = destination.view;
  const activeCollection = destination.collection;
  const isLyricsActive = destination.lyrics;
  // Collection edits and asynchronous hydration replace the current entry.
  const setActiveCollection = useCallback((value: React.SetStateAction<CollectionData | null>) => {
    updateDestination((current) => {
      const collection = typeof value === "function" ? value(current.collection) : value;
      return collection === current.collection ? current : { ...current, collection };
    });
  }, [updateDestination],
  );
  const setIsLyricsActive = useCallback((value: React.SetStateAction<boolean>) => {
    const lyrics = typeof value === "function" ? value(destination.lyrics) : value;
    navigate({ ...destination, lyrics });
  }, [destination, navigate],
  );

  // Home & Flow State
  const [recommendationNotice, setRecommendationNotice] = useState<RecommendationNoticeValue | null>(null);

  // Session identity & onboarding
  const {
    onboardingStatus, showOnboarding, setShowOnboarding,
    currentUser, setCurrentUser, userHydrated, currentUserIdRef, libraryAccountRef, userQueryRevision,
    isAuthModalOpen, setIsAuthModalOpen, afterAuthRef,
    fetchOnboardingStatus, fetchCurrentUser, handleLogin, handleSignUp, handleLogout,
  } = useAuth({ resetNavigation });
  const {
    notifications, toasts, unreadNotificationsCount, addAppNotification, dismissToast, markNotificationAsRead,
    markAllNotificationsAsRead, clearAllNotifications, deleteNotification,
  } = useNotifications();
  const { requestPrompt, promptDialog } = usePromptDialog();

  // Library & Content Data
  const {
    trackPage, albumPage, scoped, libraryScoped, tracks, setTracks, tracksRef, albums,
    settings, isScanning, setIsScanning,
    fetchTracks, fetchAlbums, fetchSettings, handleAddFolder, handleRemoveFolder, handleRescanLibrary,
  } = useLibrary({ userId: currentUser?.id || "default", destination, initialSettings, fetchOnboardingStatus });
  const interfacePreference = useInterfacePreference(settings);

  // Playback: transport state mirrored from the backend (no position ticks here).
  const {
    playbackState, setPlaybackState, loadingTrackId, setLoadingTrackId, queuedTrackIds, setQueuedTrackIds,
    queueEntries, setQueueEntries, playbackSessionId, playbackStateRevision, previousTrackRef,
    beginPlaybackLoading, endPlaybackLoading, fetchPlaybackState,
  } = usePlaybackState({ libraryAccountRef, tracksRef });

  const destinationRef = useRef(destination);
  destinationRef.current = destination;
  const { isOnline, isFullscreen, handleToggleFullscreen, handlePlayPauseRef } = useWindowShortcuts();

  // Playing Origin state: identifies where the current track is playing from
  const [playingOrigin, setPlayingOrigin] = useState<PlayingOrigin>(null);

  const {
    playlistRequest, membershipRequest, playlists, setPlaylists, playingPlaylistId, setPlayingPlaylistId, trackPlaylistMap, setTrackPlaylistMap, isPlaylistModalOpen, setIsPlaylistModalOpen, playlistModalTrack, setPlaylistModalTrack, fetchPlaylists, handleRefreshPlaylists, fetchTrackPlaylistMemberships, handleRenamePlaylist, handleDeletePlaylist, handleCreatePlaylist, handleToggleTrackInPlaylist, handleOpenAddToPlaylistModal, handleFetchPlaylistTracks, handleInspectJioSaavnPlaylist, handleSaveImportedPlaylist, handleSaveCollectionToPlaylists,
  } = usePlaylists({ libraryAccountRef, currentUser, setIsAuthModalOpen, activeCollection, setActiveCollection, addAppNotification, requestPrompt });

  const {
    wishlist, setWishlist, fetchWishlist, handleAddMissingToWishlist, handleAddToWishlist, handleUpdateWishlistStatus,
  } = useWishlist({ libraryAccountRef });
  const {
    downloads, setDownloads, fetchDownloads,
  } = useDownloads({ libraryAccountRef });

  const {
    handleStopOnlineAudio, handleStopTrack, handlePlayTrack, handlePlayOnlineTrack, handlePlayPause, handleNextTrack, handlePreviousTrack, handleSeek, handleVolumeChange, handleToggleMute, handleToggleRepeat, handleToggleShuffle, handlePlayPlaylist,
  } = usePlaybackControls({ playbackState, setPlaybackState, fetchPlaybackState, beginPlaybackLoading, endPlaybackLoading, tracks, addAppNotification, playlists, setPlayingPlaylistId, setPlayingOrigin });
  const {
    isLoadingCollectionTracks, handleOpenCollection, handlePlayDiscoveredCollection, handleOpenPlayingOrigin, handlePlayCollectionTrack, isPlayingThisCollection, isCollectionSaved, handlePlayAllCollection, handleShuffleCollection,
  } = useCollections({ navigate, destinationRef, activeCollection, setActiveCollection, setPlayingPlaylistId, playlists, playingOrigin, setPlayingOrigin, playbackState, setPlaybackState, fetchPlaybackState, beginPlaybackLoading, endPlaybackLoading, addAppNotification, handlePlayPause, handlePlayOnlineTrack, handleToggleShuffle, handleStopOnlineAudio });

  const {
    homeOverview, setHomeOverview, isHomeLoading, flowPendingAction, flowSteering, whyTrackModal, setWhyTrackModal, homeRequestRef, flowSessionRequestRef, fetchHomeOverview, fetchFlowSession, handleStartFlow, handleToggleFlowPlayback, handleFlowSteeringChange, handleInspectWhyCurrentTrack,
  } = useFlow({ libraryAccountRef, playbackState, fetchPlaybackState, onPlayPause: handlePlayPause, addAppNotification, setPlayingOrigin });

  useEffect(() => {
    if (currentView !== "home") return;
    const timer = setTimeout(() => { void fetchHomeOverview(); }, 250);
    return () => clearTimeout(timer);
  }, [currentView, playbackState.current_track?.id, currentUser?.id, fetchHomeOverview]);

  const {
    discoveryRequestRef, chartRequestRef, discoveryRecs, setDiscoveryRecs, worldTrending, setWorldTrending, isTrendingLoading, setIsTrendingLoading, trendingError, setTrendingError, fetchPersonalRecommendations, refreshTrending, refreshPersonalRecommendations, fetchDiscovery, fetchStatsOverview,
  } = useDiscoveryFeeds({ currentUserIdRef });

  useAccountSwitch(currentUser?.id, fetchDiscovery, {
    currentUserIdRef, discoveryRequestRef, chartRequestRef,
    clear: () => {
      setPlaylists([]); setTrackPlaylistMap({}); setWishlist([]); setDownloads([]); setHomeOverview(null);
      setRecommendationNotice(null); setDiscoveryRecs([]); setWorldTrending([]);
    },
    refetch: [fetchWishlist, fetchDownloads, fetchHomeOverview, fetchPlaylists, fetchTrackPlaylistMemberships],
  });

  afterAuthRef.current = async (title, message) => {
    addAppNotification("success", title, message);
    await fetchTracks();
    await fetchPlaylists();
    await fetchDiscovery(true);
  };

  // Initial load, in this order (again if a fetcher changes identity).
  useLoadAll([
    fetchOnboardingStatus, fetchPlaybackState, fetchTracks, fetchAlbums, fetchPlaylists, fetchTrackPlaylistMemberships,
    fetchWishlist, fetchDownloads, fetchDiscovery, fetchHomeOverview, fetchCurrentUser,
  ]);

  const { connectionError, retryConnection } = useBackendEvents({
    addAppNotification, chartRequestRef, currentUserIdRef, discoveryRequestRef, fetchAlbums, fetchCurrentUser, fetchDiscovery, fetchDownloads, fetchFlowSession, fetchHomeOverview, fetchOnboardingStatus, fetchPersonalRecommendations, fetchPlaybackState, fetchPlaylists, fetchTrackPlaylistMemberships, fetchTracks, fetchWishlist, flowSessionRequestRef, homeRequestRef, libraryAccountRef, membershipRequest, playbackSessionId, playbackStateRevision, playlistRequest, resetNavigation, setCurrentUser, setDiscoveryRecs, setDownloads, setHomeOverview, setIsPlaylistModalOpen, setIsScanning, setIsTrendingLoading, setLoadingTrackId, setPlaybackState, setPlayingOrigin, setPlayingPlaylistId, setPlaylistModalTrack, setPlaylists, setQueueEntries, setQueuedTrackIds, setRecommendationNotice, setTrackPlaylistMap, setTrendingError, setWishlist, setWorldTrending, tracksRef, userQueryRevision,
  });

  // --- Actions & Commands ---

  // Local Library and Kaze Downloads: each plays as its own ordered list.
  const { localPlayback, downloadsPlayback: kazeDownloadsPlayback, downloadsPage, downloadsListState } = useDownloadsSection({
    playbackState, fetchPlaybackState, beginPlaybackLoading, endPlaybackLoading, downloads, setPlayingOrigin,
    userId: currentUser?.id || "default", navigation: destination.view, query: destination.libraryQuery, downloadsDir: settings.downloads?.resolved_dir,
  });
  const { libraryShuffle, downloadsListActive, handleLibraryRowPlay, handleLibraryPlayAll, handleLibraryToggleShuffle } = localPlayback;

  const dismissRecommendationNotice = useCallback((notice: RecommendationNoticeValue) => {
    setRecommendationNotice(previous => previous === notice ? null : previous);
  }, []);
  const recommendationNoticeError = useCallback((message: string) => addAppNotification("error", "Couldn’t update recommendations", message), [addAppNotification]);
  const handleTasteSaved = useCallback(() => { void fetchPlaylists(); }, [fetchPlaylists]);

  useEffect(() => {
    handlePlayPauseRef.current = handlePlayPause;
  });

  const handleUnifiedPlayPause = handlePlayPause;

  const handleUnifiedSeek = handleSeek;

  const { likedPlaylistId, handleLike, handleRemoveFeedback, handleLikeOnline, handleToggleCollectionLike } = useLikes({
    activeCollection, setActiveCollection, addAppNotification, fetchHomeOverview, fetchPlaylists, fetchTrackPlaylistMemberships, setPlaybackState, setTrackPlaylistMap, setTracks, trackPlaylistMap, tracks, playlists,
  });

  const {
    handleEnqueueTrack, handleEnqueueRecommendation, handlePlayQueueIndex, handleDequeueTrack, handleClearQueue,
  } = useQueue({ queuedTrackIds, setQueuedTrackIds, addAppNotification });

  const {
    statsRefreshTrigger, handleSelectView, handleSelectSidebarPlaylist, handleOpenQueue, handleOpenNowPlaying,
    handleHomeDiscover, handleSearchLibrary, handleSelectAlbum,
  } = useAppNavigation({ navigate, updateDestination, handleOpenCollection, handleToggleFullscreen, albums, libraryMoment: homeOverview?.library_moment });
  const refreshAll = useCallback(
    () => Promise.allSettled([fetchDiscovery(true), fetchHomeOverview(), fetchTracks(), fetchAlbums()]),
    [fetchDiscovery, fetchHomeOverview, fetchTracks, fetchAlbums],
  );
  const {
    globalSearchQuery, setGlobalSearchQuery, isGlobalSearching, isGlobalRefreshing, currentSearchResults, setGlobalSearchResults,
    handleInitiateDirectDownloadSearch, handleGlobalOnlineSearch, handleGlobalClearSearch, handleGlobalRefresh,
  } = useGlobalSearch({ destination, navigate, updateDestination, tracks, addAppNotification, refreshAll });

  // Onboarding completion
  const handleCompleteOnboarding = async (
    folders: string[],
    startScan: boolean,
  ) => {
    await dispatchCommand({
      command: "CompleteOnboarding",
      payload: { music_folders: folders, start_scan: startScan },
    });
    setShowOnboarding(false);
    fetchOnboardingStatus();
    if (startScan) {
      setIsScanning(true);
      fetchTracks();
    }
  };

  const activePlayingTitle =
    playbackState.current_track?.title || "No Track Selected";
  const nextQueuedTrack = useMemo<PlaybackArtworkTrack | undefined>(() => {
    const entry = queueEntries[0];
    if (!entry) return undefined;
    return tracks.find((track) => track.id === entry.trackId) || {
      id: entry.trackId,
      title: entry.title || "Up next",
      artist_name: entry.artist,
    };
  }, [queueEntries, tracks]);
  const activePlayingArtist = playbackState.current_track?.artist_name || "";
  const activePlayingTrackId = playbackState.current_track?.id;
  const activePlayingDuration =
    playbackState.duration_secs ?? 0;
  const activePlayingArtwork = useNowPlayingArtwork({ track: playbackState.current_track, tracks, activeCollection, discoveryRecs, albums });
  const { likedTrackIds, isCurrentTrackLiked, regularPlaylistIds, regularTrackPlaylistMap, isCurrentTrackInPlaylist } = usePlaylistMembership({
    track: playbackState.current_track, tracks, playlists, trackPlaylistMap, likedPlaylistId,
  });

  const likedSongs = findLikedSongs(playlists);
  const handleHomeLiked = useCallback(() => { if (likedSongs) handleSelectSidebarPlaylist(likedSongs); }, [likedSongs, handleSelectSidebarPlaylist]);
  const { isArchiveLiked, isArchiveSurface } = archiveSurface({
    brutalist: interfacePreference.style === "brutalist", activeCollection, likedSongs, currentView, isLyricsActive,
  });
  const libraryTrackCount = libraryScoped ? tracks.length : trackPage.total;
  const archiveCount = isArchiveLiked
    ? activeCollection?.tracks?.length ?? likedSongs?.track_count ?? 0
    : currentView === "albums" ? albumPage.total
      : currentView === "playlists" ? regularPlaylistIds.size
        : currentView === "downloads" ? downloadsPage.total : libraryTrackCount;
  const toolbarSearch = useMemo(() => ({
    query: globalSearchQuery, setQuery: setGlobalSearchQuery, onSearch: handleGlobalOnlineSearch, onRefresh: handleGlobalRefresh,
    isSearching: isGlobalSearching, isRefreshing: isGlobalRefreshing,
  }), [globalSearchQuery, setGlobalSearchQuery, handleGlobalOnlineSearch, handleGlobalRefresh, isGlobalSearching, isGlobalRefreshing]);
  const toolbarCounts = useMemo(() => ({
    albums: albumPage.total, tracks: currentView === "downloads" ? downloadsPage.total : libraryTrackCount, archive: archiveCount,
  }), [albumPage.total, currentView, downloadsPage.total, libraryTrackCount, archiveCount]);

  const handleDeckFlow = useCallback(() => {
    if (homeOverview?.flow_session?.is_active) void handleToggleFlowPlayback();
    else void handleStartFlow(flowSteering);
  }, [homeOverview?.flow_session?.is_active, handleToggleFlowPlayback, handleStartFlow, flowSteering]);
  // The player links back only to the collection the playing song came from.
  const openableOrigin = linkableOrigin(playingOrigin, playbackState.current_track, settings.downloads?.resolved_dir);
  const toggleLyrics = useCallback(() => setIsLyricsActive((prev) => !prev), [setIsLyricsActive]);
  const toggleFullscreen = useCallback(() => handleToggleFullscreen(), [handleToggleFullscreen]);
  const closeFullscreen = useCallback(() => handleToggleFullscreen(false), [handleToggleFullscreen]);
  // Props both players share (the bar/deck and the full-screen view).
  const playerProps = {
    playbackState,
    isLoading: loadingTrackId !== null || !!playbackState.is_buffering,
    currentTrack: playbackState.current_track,
    coverArtUrl: activePlayingArtwork,
    onOpenQueue: handleOpenQueue,
    onPlayPause: handleUnifiedPlayPause,
    onNext: handleNextTrack,
    onPrevious: handlePreviousTrack,
    onSeek: handleUnifiedSeek,
    onVolumeChange: handleVolumeChange,
    onToggleMute: handleToggleMute,
    onToggleRepeat: handleToggleRepeat,
    onLike: handleLike,
    onLikeOnline: handleLikeOnline,
    isOnlineLiked: isCurrentTrackLiked,
    onRemoveFeedback: handleRemoveFeedback,
    isInPlaylist: isCurrentTrackInPlaylist,
    onOpenAddToPlaylist: handleOpenAddToPlaylistModal,
    originName: openableOrigin?.name,
    isLyricsActive,
    onToggleLyrics: toggleLyrics,
  };
  const viewContext = {
    isLyricsActive, isFullscreen, activePlayingTrackId, activePlayingArtist, activePlayingTitle, activePlayingDuration,
    handleUnifiedSeek, activeCollection, isArchiveLiked, isLoadingCollectionTracks, loadingTrackId, navigateBack,
    navigate, destination, handlePlayCollectionTrack, handlePlayAllCollection, handleShuffleCollection,
    handleAddToWishlist, handleSaveCollectionToPlaylists, playbackState, isPlayingThisCollection,
    isCollectionSaved, regularTrackPlaylistMap, regularPlaylistIds, handleOpenAddToPlaylistModal,
    handleRenamePlaylist, handleDeletePlaylist, likedTrackIds, handleToggleCollectionLike, queuedTrackIds,
    handleEnqueueRecommendation, currentView, homeOverview, isHomeLoading, flowPendingAction, fetchHomeOverview,
    handleStartFlow, handleUnifiedPlayPause, handleToggleFlowPlayback, activePlayingArtwork, previousTrackRef,
    nextQueuedTrack, handlePreviousTrack, handleNextTrack, handlePlayTrack, handleEnqueueTrack,
    handleSelectAlbum, flowSteering, handleFlowSteeringChange, handleHomeDiscover, tracks, libraryScoped,
    trackPage, scoped, handleLibraryRowPlay, handleLibraryPlayAll, downloadsListActive, libraryShuffle,
    handleLibraryToggleShuffle, handleLike, handleRemoveFeedback, handleRescanLibrary, handleSearchLibrary,
    albums, albumPage, playlists, currentUser, setIsAuthModalOpen, playingPlaylistId, handlePlayPlaylist,
    handleCreatePlaylist, handleRefreshPlaylists, handleInspectJioSaavnPlaylist, handleSaveImportedPlaylist,
    handleAddMissingToWishlist, handleInitiateDirectDownloadSearch, handleFetchPlaylistTracks,
    handleOpenCollection, discoveryRecs, worldTrending, isTrendingLoading, trendingError, refreshTrending,
    refreshPersonalRecommendations, handlePlayOnlineTrack, handleStopTrack, handlePlayDiscoveredCollection,
    downloads, globalSearchQuery, setGlobalSearchQuery, currentSearchResults, setGlobalSearchResults,
    isGlobalSearching, handleGlobalOnlineSearch, handleGlobalClearSearch, interfacePreference, queueEntries,
    handlePlayQueueIndex, handleDequeueTrack, handleClearQueue,
    fetchDownloads, settings, downloadsPage, kazeDownloadsPlayback, downloadsListState,
    wishlist, handleUpdateWishlistStatus, notifications, clearAllNotifications, deleteNotification,
    markAllNotificationsAsRead, markNotificationAsRead, fetchStatsOverview, statsRefreshTrigger, setCurrentUser,
    handleTasteSaved, handleLogout, onboardingStatus, handleAddFolder, handleRemoveFolder, fetchSettings,
    isScanning,
  };
  if (interfacePreference.style === "mobile") return <MobileApp c={viewContext} player={playerProps} />;
  const MainPlayer = interfacePreference.style === "brutalist" ? BrutalistDeck : NowPlayingBar;
  return (
    <InterfaceContext.Provider value={interfacePreference.style}>
      <div
        className="app-container kaze-interface"
        data-interface={interfacePreference.style}
      >
        {interfacePreference.style === "brutalist" && <BrutalistNavigation view={currentView} onNavigate={handleSelectView} />}
        {connectionError && <div className="connection-error" role="alert">Player connection lost. <button onClick={() => retryConnection.current()}>Retry connection</button></div>}
        <div className="app-body">
          {/* Left Sidebar Navigation */}
          {interfacePreference.style === "spatial" && <SpatialNavigation
            currentView={currentView}
            currentUser={currentUser}
            onSelectView={handleSelectView}
            unreadNotificationsCount={unreadNotificationsCount}
          />}

          {/* Main Content Area */}
          <main ref={mainScrollRef} className="main-content" data-view={currentView}
            data-library-surface={isArchiveSurface ? "true" : undefined}
            data-lyrics-active={isLyricsActive ? "true" : undefined}
            data-fixed-stage={!activeCollection && !isLyricsActive && (currentView === "discovery" || currentView === "now_playing") ? "true" : undefined}>
            <AppToolbar
              style={interfacePreference.style} currentView={currentView} collectionTitle={activeCollection?.title}
              hasCollection={!!activeCollection} isLyricsActive={isLyricsActive} isOnline={isOnline}
              isArchiveLiked={isArchiveLiked} isArchiveSurface={isArchiveSurface}
              canGoBack={canGoBack} canGoForward={canGoForward} onBack={navigateBack} onForward={navigateForward}
              onNavigate={handleSelectView} onLiked={likedSongs ? handleHomeLiked : undefined}
              unread={unreadNotificationsCount} search={toolbarSearch} counts={toolbarCounts}
            />

            <ViewMemoryProvider>
            <Suspense fallback={null}>
              <AppViews c={viewContext} />
            </Suspense>
            </ViewMemoryProvider>
          </main>
        </div>

        {/* Bottom Sticky Player Bar (Unified for local music & online streams) */}
        <MainPlayer
          {...playerProps}
          flowActive={!!homeOverview?.flow_session?.is_active}
          onToggleFlow={handleDeckFlow}
          entries={queueEntries}
          tracks={tracks}
          onPlayIndex={handlePlayQueueIndex}
          currentUser={currentUser}
          view={currentView}
          onNavigate={handleSelectView}
          onOpenNowPlaying={handleOpenNowPlaying}
          onOpenOrigin={openableOrigin ? handleOpenPlayingOrigin : undefined}
          onInspectWhyTrack={
            playingOrigin?.type === "flow" && playbackState.current_track
              ? handleInspectWhyCurrentTrack
              : undefined
          }
          isFullscreen={isFullscreen}
          onToggleFullscreen={toggleFullscreen}
        />

        {/* Full Screen Player View (matching user uploaded images) */}
        <Suspense fallback={null}>
          {isFullscreen && (
            <FullScreenPlayerView
              {...playerProps}
              brutalist={interfacePreference.style === "brutalist" ? { entries: queueEntries, tracks, onPlayIndex: handlePlayQueueIndex, currentUser, view: currentView, onNavigate: handleSelectView, flowActive: !!homeOverview?.flow_session?.is_active, onToggleFlow: handleDeckFlow, albumId: albums.find(a => a.title === playbackState.current_track?.album_title && a.artist_name === playbackState.current_track?.artist_name)?.id, onOpenAlbum: handleSelectAlbum } : undefined}
              onOpenNowPlaying={() => { handleToggleFullscreen(false); handleOpenNowPlaying(); }}
              previousTrack={previousTrackRef.current}
              nextTrack={nextQueuedTrack}
              onInspectWhyTrack={
                playingOrigin?.type === "flow"
                  ? handleInspectWhyCurrentTrack
                  : undefined
              }
              onToggleFullscreen={closeFullscreen}
              onOpenOrigin={openableOrigin ? () => { handleToggleFullscreen(false); handleOpenPlayingOrigin(); } : undefined}
            />
          )}

          {/* Top-Right Floating Toast Notifications */}
          <ToastContainer toasts={toasts} onDismiss={dismissToast} />

          {/* Auto-update notification banner */}
          <UpdateBanner />

          {/* Center Modal Popup for Playlist Management */}
          <AddToPlaylistModal
            isOpen={isPlaylistModalOpen}
            onClose={() => {
              setIsPlaylistModalOpen(false);
              setPlaylistModalTrack(null);
            }}
            track={playlistModalTrack}
            playlists={playlists.filter(
              (p) => p.is_smart_mix === 0 && p.name !== "Liked Songs",
            )}
            trackPlaylistIds={
              playlistModalTrack &&
              regularTrackPlaylistMap[playlistModalTrack.id]
                ? regularTrackPlaylistMap[playlistModalTrack.id]
                : []
            }
            onTogglePlaylist={handleToggleTrackInPlaylist}
            onCreatePlaylist={handleCreatePlaylist}
          />

          {/* Auth Modal (Login / Sign Up) */}
          {isAuthModalOpen && (
            <AuthModal
              isOpen={isAuthModalOpen}
              onClose={() => setIsAuthModalOpen(false)}
              onLogin={handleLogin}
              onSignUp={handleSignUp}
            />
          )}

        </Suspense>

        <PlaybackNotice key={`playback-notice:${currentUser?.id || "default"}`} />
        {recommendationNotice && <RecommendationNotice key={currentUser?.id || "default"} notice={recommendationNotice} onDismiss={dismissRecommendationNotice} onError={recommendationNoticeError} />}
        <RecommendationSetupGate userId={currentUser?.id || "default"} key={`taste:${currentUser?.id || "default"}`} ready={userHydrated && !!onboardingStatus?.completed && !showOnboarding} onSaved={handleTasteSaved} />

        {/* Onboarding Modal */}
        {showOnboarding && onboardingStatus && (
          <OnboardingModal
            defaultMusicDir={onboardingStatus.default_music_dir}
            onComplete={handleCompleteOnboarding}
          />
        )}

        {/* Themed In-App Prompt Dialog */}
        {promptDialog}

        {/* Yes/No questions (e.g. removing a download); store-driven, no App state */}
        <ConfirmModal />

        {whyTrackModal && <WhyTrackModal explained={whyTrackModal} onClose={() => setWhyTrackModal(null)} />}
      </div>
    </InterfaceContext.Provider>
  );
};
