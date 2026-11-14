import { isOnlineTrack } from "./trackKind";
import { ownBackendSubscription } from "./services/backendSubscription";
import { PlaybackNotice } from "./components/PlaybackNotice";
import { RecommendationNotice, type RecommendationNoticeValue } from "./components/RecommendationNotice";
import { RecommendationSetupGate } from "./components/MusicTaste";
import { recommendationPreferences } from "./services/recommendationPreferences";
import "./App.css";
import { removeRecommendationTracks } from "./recommendationFeedback";
import { useRouteScroll } from "./presentation/useRouteScroll";
import { ViewMemoryProvider } from "./presentation/ViewMemory";
import { useVisualActivity } from "./presentation/useVisualActivity";
import { usePointerFocus } from "./presentation/usePointerFocus";
import { useHorizontalDrag } from "./presentation/useHorizontalDrag";
import React, { useState, useEffect, useCallback, useRef, useMemo, useSyncExternalStore, Suspense, lazy,
} from "react";
import {
  Track,
  Playlist,
  DiscoveryRecommendation,
  PlaybackState,
  AppSettings,
  AppNotification,
} from "./types";
import { dispatchCommand, executeQuery, subscribeBackendEvents,
} from "./services/api";
import { playbackProgress } from "./services/playbackProgress";
import { downloadProgress } from "./services/downloadProgress";
import { subscribeCoverArt, coverArtGeneration, loadCoverArt, clearCoverArtCache } from "./services/coverArt";
import { viewDestination } from "./navigationHistory";
import { useNavigationHistory } from "./hooks/useNavigationHistory";
import { SpatialNavigation } from "./presentation/SpatialNavigation";
import { NowPlayingBar } from "./components/NowPlayingBar";
import { OnboardingModal } from "./components/OnboardingModal";
import { TrackExplanation } from "./components/TrackExplanation";
import { HomeView } from "./components/views/HomeView";
import { LibraryView } from "./components/views/LibraryView";
import { AlbumsView } from "./components/views/AlbumsView";
import { PlaylistsView } from "./components/views/PlaylistsView";
import { SettingsView } from "./components/views/SettingsView";
import { GlobalTopSearchBar } from "./components/layout/GlobalTopSearchBar";
import { ToastContainer } from "./components/notifications/ToastContainer";
import { UpdateBanner } from "./components/UpdateBanner";
import { attachLocalSearchMatches } from "./localTrackMatch";
import { PromptModal } from "./components/modals/PromptModal";
import { ConfirmModal } from "./components/modals/ConfirmModal";
import { useWindowShortcuts } from "./hooks/app/useWindowShortcuts";
import { useAuth } from "./hooks/app/useAuth";
import { useLibrary } from "./hooks/app/useLibrary";
import { useDownloadsSection } from "./hooks/app/useDownloadsSection";
import { usePlaylists } from "./hooks/app/usePlaylists";
import { useWishlist } from "./hooks/app/useWishlist";
import { useDownloads } from "./hooks/app/useDownloads";
import { useDiscoveryFeeds } from "./hooks/app/useDiscoveryFeeds";
import { usePlaybackControls } from "./hooks/app/usePlaybackControls";
import { useFlow } from "./hooks/app/useFlow";
import { useCollections } from "./hooks/app/useCollections";
import { useQueue } from "./hooks/app/useQueue";
import {
  CollectionDetailView,
  CollectionData,
  CollectionTrackItem,
} from "./components/views/CollectionDetailView";
import type { EntityFeedbackRow } from "./types";
import { X, Sparkles } from "lucide-react";
import { InterfaceContext } from "./presentation/InterfaceContext";
import { useInterfacePreference } from "./presentation/useInterfacePreference";
import { PlaybackStage } from "./presentation/PlaybackStage";
import { QueueView } from "./presentation/QueueView";
import {
  upcomingFromEvent,
  upcomingFromPlayback,
  withKnownMetadata,
  type QueueEntry,
  type PlaybackArtworkTrack,
} from "./presentation/queuePresentation";
import { ArrowLeft, ArrowRight } from "lucide-react";
import "./presentation/interfaces.css";
import "./presentation/spatial.css";
import "./presentation/familiarity-dial.css";
import "./presentation/spatial-preview.css";
import "./presentation/brutalist.css";
import "./presentation/responsive.css";
import { BrutalistNavigation, BrutalistLibraryHeader } from "./presentation/BrutalistNavigation";
import { BrutalistDeck } from "./presentation/BrutalistDeck";
import { BrutalistPoster } from "./presentation/BrutalistPoster";
import { SpatialUtilities, SpatialLibraryHeader } from "./presentation/SpatialUtilities";

// Lazy-loaded secondary views and modals to reduce initial memory and JS bundle overhead
const DownloadsView = lazy(() =>
  import("./components/views/DownloadsView").then((m) => ({
    default: m.DownloadsView,
  })),
);
const WishlistView = lazy(() =>
  import("./components/views/WishlistView").then((m) => ({
    default: m.WishlistView,
  })),
);
const DiscoveryView = lazy(() => import("./components/views/DiscoveryView").then((m) => ({ default: m.DiscoveryView,
  })),
);
const NotificationsView = lazy(() => import("./components/views/NotificationsView").then((m) => ({ default: m.NotificationsView,
  })),
);
const StatsView = lazy(() => import("./components/views/StatsView").then((m) => ({ default: m.StatsView,
  })),
);
const FullScreenPlayerView = lazy(() => import("./components/views/FullScreenPlayerView").then((m) => ({ default: m.FullScreenPlayerView,
  })),
);
const LyricsView = lazy(() => import("./components/views/LyricsView").then((m) => ({ default: m.LyricsView,
  })),
);
const AuthModal = lazy(() => import("./components/modals/AuthModal").then((m) => ({ default: m.AuthModal,
  })),
);
const AddToPlaylistModal = lazy(() => import("./components/modals/AddToPlaylistModal").then((m) => ({ default: m.AddToPlaylistModal,
  })),
);

export const App: React.FC<{ initialSettings: AppSettings }> = ({
  initialSettings,
}) => {
  // Navigation
  const { destination, navigate, reset: resetNavigation, updateDestination, back: navigateBack,
    forward: navigateForward,
    canGoBack,
    canGoForward,
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
  const [statsRefreshTrigger, setStatsRefreshTrigger] = useState(0);

  // Playback
  const [playbackState, setPlaybackState] = useState<PlaybackState>({
    is_playing: false,
    position_secs: 0,
    duration_secs: 0,
    volume: 0.8,
    is_muted: false,
    repeat_mode: "off",
    is_shuffled: false,
  });
  const [loadingTrackId, setLoadingTrackId] = useState<string | null>(null);
  const playbackRequestId = useRef(0);
  const playbackSessionId = useRef<string | null>(null);
  const playbackStateRevision = useRef(0);
  const playbackStateQueryId = useRef(0);
  const beginPlaybackLoading = useCallback((trackId: string) => {
    ++playbackStateRevision.current;
    const requestId = ++playbackRequestId.current;
    setLoadingTrackId(trackId);
    return requestId;
  }, []);
  const endPlaybackLoading = useCallback((requestId: number) => {
    if (playbackRequestId.current === requestId) setLoadingTrackId(null);
  }, []);

  // Library & Content Data
  const {
    trackPage, albumPage, scoped, libraryScoped, tracks, setTracks, tracksRef, albums,
    settings, isScanning, setIsScanning,
    fetchTracks, fetchAlbums, fetchSettings, handleAddFolder, handleRemoveFolder, handleRescanLibrary,
  } = useLibrary({ userId: currentUser?.id || "default", destination, initialSettings, fetchOnboardingStatus });
  const [queuedTrackIds, setQueuedTrackIds] = useState<Set<string>>(new Set());
  const interfacePreference = useInterfacePreference(settings);
  const [queueEntries, setQueueEntries] = useState<QueueEntry[]>([]);
  const previousTrackRef = useRef<Track | undefined>(undefined);
  const observedTrackRef = useRef<Track | undefined>(undefined);
  if (!playbackState.is_buffering && playbackState.current_track && playbackState.current_track?.id !== observedTrackRef.current?.id) {
    previousTrackRef.current = observedTrackRef.current;
    observedTrackRef.current = playbackState.current_track;
  }

  // Notifications & Toasts System
  const [notifications, setNotifications] = useState<AppNotification[]>([]);
  const [toasts, setToasts] = useState<AppNotification[]>([]);

  // Global Top Search Bar State
  const [globalSearchQuery, setGlobalSearchQuery] = useState("");
  const [isGlobalSearching, setIsGlobalSearching] = useState(false);
  const [searchRefreshRevision, setSearchRefreshRevision] = useState(0);
  const [isGlobalRefreshing, setIsGlobalRefreshing] = useState(false);
  const globalSearchResults = destination.searchResults;
  const setGlobalSearchResults = useCallback((results: React.SetStateAction<DiscoveryRecommendation[] | null>) => {
    updateDestination((current) => ({ ...current, searchResults: typeof results === "function" ? results(current.searchResults) : results,
      }));
  }, [updateDestination],
  );
  const destinationRef = useRef(destination);
  destinationRef.current = destination;
  useEffect(() => { setGlobalSearchQuery(destination.searchQuery); }, [destination.searchQuery]);
  const currentSearchResults = useMemo(
    () => attachLocalSearchMatches(globalSearchResults, tracks),
    [globalSearchResults, tracks],
  );

  const { isOnline, isFullscreen, handleToggleFullscreen, handlePlayPauseRef } = useWindowShortcuts();

  // Playing Origin state: identifies where the current track is playing from
  const [playingOrigin, setPlayingOrigin] = useState<{
    type: "playlist" | "mix" | "chart" | "album" | "discovery" | "flow";
    id?: string;
    name?: string;
    collectionData?: CollectionData;
  } | null>(null);

  // Themed Prompt Dialog State (replaces ugly browser window.prompt)
  const [promptDialog, setPromptDialog] = useState<{
    isOpen: boolean;
    title: string;
    message: string;
    defaultValue: string;
    resolve: (val: string | null) => void;
  } | null>(null);

  const requestPrompt = useCallback((title: string, message: string, defaultValue: string = "",
    ): Promise<string | null> => {
    return new Promise((resolve) => {
      setPromptDialog({
        isOpen: true,
        title,
        message,
        defaultValue,
        resolve,
      });
    });
  }, [],
  );

  // Resolve once per track and share cached artwork with the mounted views.
  const artworkGeneration = useSyncExternalStore(subscribeCoverArt, coverArtGeneration, coverArtGeneration);
  const [resolvedTrackCover, setResolvedTrackCover] = useState<{ id: string; generation: number; url: string | null } | null>(null);
  const coverTrack = playbackState.current_track;
  const currentTrackCoverUrl = coverTrack?.cover_art_url || (resolvedTrackCover?.generation === artworkGeneration && resolvedTrackCover?.id === coverTrack?.id ? resolvedTrackCover?.url : null);
  useEffect(() => {
    const track = playbackState.current_track;
    if (!track || track.cover_art_url || track.id.startsWith("online:") || track.id.startsWith("itunes:")) return;
    const controller = new AbortController();
    loadCoverArt(track.id, controller.signal).then(url => {
      if (!controller.signal.aborted) setResolvedTrackCover({ id: track.id, generation: artworkGeneration, url });
    });
    return () => controller.abort();
  }, [playbackState.current_track?.id, playbackState.current_track?.cover_art_url, playbackState.current_track?.has_cover_art, artworkGeneration]);

  // --- Notification Handlers ---
  const addAppNotification = useCallback(
    (type: "success" | "error" | "info" | "warning", title: string, message: string,
    ) => {
      const newNotification: AppNotification = {
        id: "notif_" + Date.now() + "_" + Math.random().toString(36).substring(2, 7),
        type,
        title,
        message,
        timestamp: Date.now(),
        read: false,
      };
      setNotifications((prev) => [newNotification, ...prev]);
      setToasts((prev) => [...prev, newNotification]);

      // Automatically dismiss popup toast notification after 3 seconds
      setTimeout(() => {
        setToasts((prev) => prev.filter((t) => t.id !== newNotification.id));
      }, 3000);
    },
    [],
  );

  const dismissToast = useCallback((id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  const markNotificationAsRead = useCallback((id: string) => {
    setNotifications((prev) =>
      prev.map((n) => (n.id === id ? { ...n, read: true } : n)),
    );
  }, []);

  const markAllNotificationsAsRead = useCallback(() => {
    setNotifications((prev) => prev.map((n) => ({ ...n, read: true })));
  }, []);

  const clearAllNotifications = useCallback(() => {
    setNotifications([]);
  }, []);

  const deleteNotification = useCallback((id: string) => {
    setNotifications((prev) => prev.filter((n) => n.id !== id));
  }, []);

  // --- Fetch Data Handlers ---


  const fetchPlaybackState = useCallback(async () => {
    const account = libraryAccountRef.current;
    const revision = playbackStateRevision.current;
    const queryId = ++playbackStateQueryId.current;
    try {
      const res = await executeQuery({ query: "GetPlaybackState" });
      // Source/catalog work can delay this response beyond a newer transport
      // event. Never reinstate the old song or its buffering/paused state.
      if (account !== libraryAccountRef.current || queryId !== playbackStateQueryId.current) return;
      if (revision !== playbackStateRevision.current) {
        // A newer transport event won: keep its play/pause/buffering state, but
        // still resolve a placeholder for the same song, or it stays pending
        // (greyed-out controls) until the next manual refresh.
        const resolved: Track | undefined = res.data?.current_track;
        if (resolved) {
          setPlaybackState((prev) =>
            prev.current_track?.pending && prev.current_track.id === resolved.id
              ? { ...prev, current_track: resolved }
              : prev);
        }
        return;
      }
      if (res.data) {
        playbackProgress.update(
          res.data.position_secs ?? 0,
          res.data.duration_secs ?? 0,
        );
        setPlaybackState((prev) => {
          const updated = { ...prev, ...res.data, duration_secs: res.data.duration_secs ?? 0, current_track: undefined,
            is_shuffled:
              res.data.is_shuffle ?? res.data.is_shuffled ?? prev.is_shuffled,
          };
          if (res.data.current_track) {
            updated.current_track = res.data.current_track;
          } else if (res.data.current_track_id) {
            const found = tracksRef.current.find((t) => t.id === res.data.current_track_id,
            );
            if (found) updated.current_track = found;
          }
          return updated;
        });
        if (Array.isArray(res.data.queue_track_ids)) {
          setQueuedTrackIds(new Set(res.data.queue_track_ids));
          setQueueEntries((prev) => withKnownMetadata(upcomingFromPlayback(res.data), prev));
        }
      }
    } catch (err) {
      console.error("Failed to fetch playback state:", err);
    }
  }, []);


  const {
    playlistRequest, membershipRequest, playlists, setPlaylists, playingPlaylistId, setPlayingPlaylistId, trackPlaylistMap, setTrackPlaylistMap, isPlaylistModalOpen, setIsPlaylistModalOpen, playlistModalTrack, setPlaylistModalTrack, fetchPlaylists, handleRefreshPlaylists, fetchTrackPlaylistMemberships, handleRenamePlaylist, handleDeletePlaylist, handleCreatePlaylist, handleToggleTrackInPlaylist, handleOpenAddToPlaylistModal, handleFetchPlaylistTracks, handleInspectJioSaavnPlaylist, handleSaveImportedPlaylist, handleSaveCollectionToPlaylists,
  } = usePlaylists({ libraryAccountRef, currentUser, setIsAuthModalOpen, activeCollection, setActiveCollection, addAppNotification, requestPrompt });

  const {
    wishlist, setWishlist, fetchWishlist, handleAddMissingToWishlist, handleAddToWishlist, handleUpdateWishlistStatus,
  } = useWishlist({ libraryAccountRef });
  const {
    downloads, setDownloads, fetchDownloads, handleSearchDownloads, handleStartDownload, handleCancelDownload,
  } = useDownloads({ libraryAccountRef });

  const {
    handleStopOnlineAudio, handleStopTrack, handlePlayTrack, handlePlayOnlineTrack, handlePlayPause, handleNextTrack, handlePreviousTrack, handleSeek, handleVolumeChange, handleToggleMute, handleToggleRepeat, handleToggleShuffle, handlePlayPlaylist,
  } = usePlaybackControls({ playbackState, setPlaybackState, fetchPlaybackState, beginPlaybackLoading, endPlaybackLoading, tracks, addAppNotification, playlists, setPlayingPlaylistId, setPlayingOrigin, queuedTrackIds });
  const {
    isLoadingCollectionTracks, handleOpenCollection, handlePlayDiscoveredCollection, handleOpenPlayingOrigin, handlePlayCollectionTrack, isPlayingThisCollection, isCollectionSaved, handlePlayAllCollection, handleShuffleCollection,
  } = useCollections({ navigate, destinationRef, activeCollection, setActiveCollection, setPlayingPlaylistId, playlists, playingOrigin, setPlayingOrigin, playbackState, setPlaybackState, fetchPlaybackState, beginPlaybackLoading, endPlaybackLoading, queuedTrackIds, addAppNotification, handlePlayPause, handlePlayOnlineTrack, handleToggleShuffle, handleStopOnlineAudio });


  const {
    homeOverview, setHomeOverview, isHomeLoading, flowPendingAction, flowSteering, whyTrackModal, setWhyTrackModal, homeRequestRef, flowSessionRequestRef, fetchHomeOverview, fetchFlowSession, handleStartFlow, handleToggleFlowPlayback, handleFlowSteeringChange, handleInspectWhyCurrentTrack,
  } = useFlow({ libraryAccountRef, playbackState, fetchPlaybackState, onPlayPause: handlePlayPause, addAppNotification, setPlayingOrigin });

  useEffect(() => {
    if (currentView !== "home") return;
    const timer = setTimeout(() => { void fetchHomeOverview(); }, 250);
    return () => clearTimeout(timer);
  }, [currentView, playbackState.current_track?.id, currentUser?.id, fetchHomeOverview,
  ]);

  const {
    discoveryRequestRef, chartRequestRef, discoveryRecs, setDiscoveryRecs, worldTrending, setWorldTrending, isTrendingLoading, setIsTrendingLoading, trendingError, setTrendingError, fetchPersonalRecommendations, refreshTrending, refreshPersonalRecommendations, fetchDiscovery, fetchStatsOverview,
  } = useDiscoveryFeeds({ currentUserIdRef });

  useEffect(() => {
    const userId = currentUser?.id || "default";
    if (userId === currentUserIdRef.current) return;
    currentUserIdRef.current = userId;
    recommendationPreferences.reset();
    setPlaylists([]); setTrackPlaylistMap({});
    setWishlist([]); setDownloads([]); setHomeOverview(null);
    void fetchWishlist(); void fetchDownloads(); void fetchHomeOverview();
    void fetchPlaylists(); void fetchTrackPlaylistMemberships();
    setRecommendationNotice(null);
    void recommendationPreferences.refresh().catch(console.error);
    ++discoveryRequestRef.current;
    ++chartRequestRef.current;
    setDiscoveryRecs([]);
    setWorldTrending([]);
    void fetchDiscovery();
  }, [currentUser?.id, fetchDiscovery]);


  afterAuthRef.current = async (title, message) => {
    addAppNotification("success", title, message);
    await fetchTracks();
    await fetchPlaylists();
    await fetchDiscovery(true);
  };

  // Initial load
  useEffect(() => {
    fetchOnboardingStatus();
    fetchPlaybackState();
    fetchTracks();
    fetchAlbums();
    fetchPlaylists();
    fetchTrackPlaylistMemberships();
    fetchWishlist();
    fetchDownloads();
    fetchDiscovery();
    fetchHomeOverview();
    fetchCurrentUser();
  }, [
    fetchOnboardingStatus,
    fetchPlaybackState,
    fetchTracks,
    fetchAlbums,
    fetchPlaylists,
    fetchTrackPlaylistMemberships,
    fetchWishlist,
    fetchDownloads,
    fetchDiscovery,
    fetchHomeOverview,
    fetchCurrentUser,
  ]);

  const [connectionError, setConnectionError] = useState(false);
  const retryConnection = useRef<() => void>(() => {});

  // Backend Event Subscriptions
  useEffect(() => {
    let subscriptionDisposed = false;
    let tasteRefreshTimer: ReturnType<typeof setTimeout> | undefined;

    const owner = ownBackendSubscription(() => subscribeBackendEvents((event: any) => {
      if (subscriptionDisposed || !event || !event.event) return;

      switch (event.event) {
        case "PlaybackStarted": {
          ++playbackStateRevision.current;
          playbackSessionId.current = event.payload?.session_id || null;
          playbackProgress.update(0, event.payload?.duration_secs || 0);
          const trackId = event.payload?.track_id;
          setLoadingTrackId((pending) => pending === trackId ? null : pending);
          const found = tracksRef.current.find((t) => t.id === trackId);
          const currentTrackDetail = event.payload?.current_track;
          const track: Track =
            found ||
            (currentTrackDetail
              ? {
                  id: currentTrackDetail.id,
                  file_path: currentTrackDetail.file_path,
                  title: currentTrackDetail.title,
                  artist_name: currentTrackDetail.artist_name,
                  album_title: currentTrackDetail.album_title,
                  duration_secs: currentTrackDetail.duration_secs,
                  format: currentTrackDetail.format,
                  has_cover_art: currentTrackDetail.cover_art_url ? 1 : 0,
                  cover_art_url: currentTrackDetail.cover_art_url,
                }
              : {
                  id: trackId,
                  file_path: "",
                  title: event.payload?.title || "Unknown Track",
                  artist_name: event.payload?.artist || "Unknown Artist",
                  duration_secs: event.payload?.duration_secs || 0,
                  format: "",
                  has_cover_art: 0,
                  // Unknown until the backend answers: never guess local vs online.
                  pending: true,
                });
          setPlaybackState((prev) => ({
            ...prev,
            // Keep an already-resolved copy of this song instead of a placeholder.
            current_track: track.pending && prev.current_track?.id === trackId && !prev.current_track?.pending
              ? prev.current_track
              : track,
            is_playing: true,
            is_paused: false,
            is_buffering: false,
            position_secs: 0,
            duration_secs:
              event.payload?.duration_secs ||
              0,
          }));
          // The event only names the song, and the library list is paged, so load
          // the authoritative (presented) track rather than trusting a guess.
          if (!found) void fetchPlaybackState();
          break;
        }

        case "PlaybackBuffering": {
          ++playbackStateRevision.current;
          // Only PlaybackService emits this initial-resolution event. Reader
          // stalls use the session-scoped event below and cannot select a song.
          playbackSessionId.current = null;
          playbackProgress.update(0, 0);
          const trackId = event.payload?.track_id;
          const found = tracksRef.current.find((t) => t.id === trackId);
          setPlaybackState((prev) => ({
            ...prev,
            current_track:
              found ||
              (trackId && prev.current_track?.id === trackId && !prev.current_track?.pending
                ? prev.current_track
                : trackId
                ? {
                    id: trackId,
                    file_path: "",
                    title:
                      (prev.current_track?.id === trackId
                        ? prev.current_track?.title
                        : undefined) || "Resolving Audio...",
                    artist_name:
                      (prev.current_track?.id === trackId
                        ? prev.current_track?.artist_name
                        : undefined) || "Please wait",
                    duration_secs:
                      (prev.current_track?.id === trackId
                        ? prev.current_track?.duration_secs
                        : undefined) || 0,
                    format: "",
                    has_cover_art: 0,
                    pending: true,
                  }
                : prev.current_track),
            is_buffering: true,
            duration_secs: 0,
            position_secs: 0,
          }));
          break;
        }

        case "PlaybackBufferingChanged": {
          if (!event.payload?.session_id || event.payload.session_id !== playbackSessionId.current) break;
          ++playbackStateRevision.current;
          setPlaybackState((prev) => {
            if (prev.current_track?.id !== event.payload?.track_id) return prev;
            const buffering = !!event.payload?.is_buffering && prev.is_playing;
            return prev.is_buffering === buffering ? prev : { ...prev, is_buffering: buffering };
          });
          break;
        }

        case "PlaybackError":
          ++playbackStateRevision.current;
          setPlaybackState((prev) => ({ ...prev, is_buffering: false }));
          addAppNotification("error", "Playback unavailable", event.payload?.message || "This track couldn’t be played. Try another track or retry playback.");
          void fetchPlaybackState();
          break;

        case "PlaybackPaused":
          ++playbackStateRevision.current;
          playbackProgress.update(
            event.payload?.position_secs ?? playbackProgress.getPosition(),
            playbackProgress.getDuration(),
          );
          setPlaybackState((prev) => ({
            ...prev,
            is_playing: false,
            is_paused: true,
            is_buffering: false,
            position_secs: event.payload?.position_secs ?? prev.position_secs,
          }));
          break;

        case "PlaybackResumed":
          ++playbackStateRevision.current;
          playbackSessionId.current = event.payload?.session_id || playbackSessionId.current;
          playbackProgress.update(
            event.payload?.position_secs ?? playbackProgress.getPosition(),
            playbackProgress.getDuration(),
          );
          setPlaybackState((prev) => ({
            ...prev,
            is_playing: true,
            is_paused: false,
            is_buffering: false,
            position_secs: event.payload?.position_secs ?? prev.position_secs,
          }));
          break;

        case "PlaybackStopped":
          ++playbackStateRevision.current;
          playbackSessionId.current = null;
          playbackProgress.update(0, playbackProgress.getDuration());
          setPlaybackState((prev) => ({
            ...prev,
            is_playing: false,
            is_paused: false,
            is_buffering: false,
            position_secs: 0,
          }));
          break;

        case "PlaybackSeeked": {
          const pos = event.payload?.position_secs ?? 0;
          playbackProgress.update(pos, playbackProgress.getDuration());
          break;
        }

        case "PlaybackPositionChanged": {
          if (!playbackSessionId.current && event.payload?.session_id) playbackSessionId.current = event.payload.session_id;
          if (event.payload?.session_id && playbackSessionId.current && event.payload.session_id !== playbackSessionId.current) break;
          const pos = event.payload?.position_secs ?? 0;
          const dur = event.payload?.duration_secs ?? 0;
          playbackProgress.update(pos, dur);
          break;
        }

        case "PlaybackVolumeChanged":
        case "VolumeChanged":
          setPlaybackState((prev) => ({
            ...prev,
            volume: event.payload?.volume ?? prev.volume,
            is_muted: event.payload?.is_muted ?? prev.is_muted,
          }));
          break;

        case "QueueUpdated": {
          void fetchFlowSession().catch(console.error);
          if (Array.isArray(event.payload?.items)) {
            setQueueEntries(
              upcomingFromEvent(
                event.payload.items,
                event.payload.current_index ?? null,
              ),
            );
            setPlaybackState((prev) => ({
              ...prev,
              current_queue_index: event.payload.current_index ?? null,
              source: event.payload.source || prev.source,
            }));
          }
          if (Array.isArray(event.payload?.queue_track_ids)) {
            setQueuedTrackIds(new Set(event.payload.queue_track_ids));
          } else {
            const items: any[] = event.payload?.items || [];
            const curr = event.payload?.current_index;
            const upcoming =
              curr !== null && curr !== undefined
                ? items.slice(curr + 1)
                : items;
            setQueuedTrackIds(new Set(upcoming.map((i: any) => i.track_id)));
          }
          break;
        }

        case "PlaybackStateChanged":
          setPlaybackState((prev) => ({
            ...prev,
            is_playing: event.payload?.is_playing ?? prev.is_playing,
          }));
          break;

        case "TrackChanged":
          setPlaybackState((prev) => ({
            ...prev,
            current_track: event.payload?.track ?? prev.current_track,
            position_secs: 0,
            duration_secs: event.payload?.track?.duration_secs ?? 0,
          }));
          break;

        case "RepeatModeChanged":
          setPlaybackState((prev) => ({
            ...prev,
            repeat_mode: event.payload?.mode ?? prev.repeat_mode,
          }));
          break;

        case "ShuffleModeChanged":
          setPlaybackState((prev) => ({
            ...prev,
            is_shuffled: event.payload?.enabled ?? prev.is_shuffled,
          }));
          break;

        case "LibraryScanStarted":
          setIsScanning(true);
          break;

        case "LibraryMetadataUpdated":
          clearCoverArtCache();
          fetchTracks();
          fetchAlbums();
          fetchPlaybackState();
          break;
        case "LibraryScanCompleted":
          clearCoverArtCache();
          setIsScanning(false);
          fetchDownloads();
          addAppNotification(
            "info",
            "Library Scan Complete",
            "Your local music library has been updated with the latest tracks.",
          );
          fetchTracks();
          fetchAlbums();
          fetchOnboardingStatus();
          fetchDiscovery();
          break;

        case "LibraryScanFailed":
          setIsScanning(false);
          addAppNotification(
            "error",
            "Library Scan Failed",
            "An error occurred while scanning your music folders.",
          );
          break;

        case "DownloadQueued":
          fetchDownloads();
          break;

        // Files in Kaze's folder were added, removed or renamed (watched live).
        case "DownloadsChanged":
          fetchDownloads();
          fetchTracks();
          // A removed download's likes and playlist entries move to its online song.
          fetchTrackPlaylistMemberships();
          break;

        case "DownloadProgress":
        case "DownloadProgressChanged":
          downloadProgress.update(event.payload?.task_id, event.payload?.bytes_downloaded,
            event.payload?.total_bytes || event.payload?.file_size);
          break;

        case "DownloadCompleted": {
          const trackTitle = event.payload?.title || "Track";
          const artist = event.payload?.artist
            ? ` by ${event.payload.artist}`
            : "";
          addAppNotification(
            "success",
            "Download Complete",
            `"${trackTitle}"${artist} has been downloaded and added to your library.`,
          );
          fetchDownloads();
          fetchWishlist();
          fetchTracks();
          fetchAlbums();
          fetchDiscovery();
          break;
        }

        case "DownloadFailed": {
          const trackTitle = event.payload?.title || "Track";
          const errReason =
            event.payload?.error ||
            event.payload?.message ||
            "Connection timed out or peer unavailable.";
          addAppNotification(
            "error",
            "Download Failed",
            `Failed downloading "${trackTitle}": ${errReason}`,
          );
          fetchDownloads();
          fetchWishlist();
          fetchTracks();
          fetchAlbums();
          fetchDiscovery();
          break;
        }

        case "WishlistUpdated":
          fetchWishlist();
          break;

        case "RecommendationFeedbackChanged": {
          if (event.payload?.user_id !== currentUserIdRef.current) break;
          ++homeRequestRef.current; ++discoveryRequestRef.current; ++chartRequestRef.current;
          setIsTrendingLoading(false);
          recommendationPreferences.update(event.payload?.track_ids || [], !!event.payload?.disliked);
          const ids = new Set<string>(event.payload?.track_ids || []);
          if (event.payload?.disliked) {
            setDiscoveryRecs(previous => removeRecommendationTracks(previous, ids));
            setWorldTrending(previous => removeRecommendationTracks(previous, ids));
            setHomeOverview(previous => removeRecommendationTracks(previous, ids));
            setRecommendationNotice({ id: event.payload.track_id, title: event.payload.title });
          } else {
            setRecommendationNotice(previous => previous && ids.has(previous.id) ? null : previous);
            void fetchDiscovery(false, false);
          }
          setPlaybackState(previous => previous.current_track && ids.has(previous.current_track.id)
            ? { ...previous, current_track: { ...previous.current_track, manual_like: event.payload.manual_like } } : previous);
          void fetchPlaylists();
          break;
        }
        case "EntityFeedbackChanged": {
          if (event.payload?.user_id !== currentUserIdRef.current) break;
          ++homeRequestRef.current; ++discoveryRequestRef.current; ++chartRequestRef.current;
          setIsTrendingLoading(false);
          void recommendationPreferences.refresh();
          const rows: EntityFeedbackRow[] = event.payload?.rows || [];
          if (!event.payload?.cleared && rows.length) {
            setRecommendationNotice({ id: `entity:${rows.map(row => `${row.kind}:${row.key}`).join("|")}`, title: rows[0].display, entities: rows });
          }
          void fetchDiscovery(false, false);
          void fetchPlaylists();
          break;
        }
        case "TasteProfileChanged": {
          if (event.payload?.user_id !== currentUserIdRef.current) break;
          // The writer emits this after commit: no race with history persistence.
          // Coalesce rapid skips without subscribing App to position ticks.
          ++discoveryRequestRef.current;
          ++homeRequestRef.current;
          clearTimeout(tasteRefreshTimer);
          tasteRefreshTimer = setTimeout(() => {
            void fetchPersonalRecommendations().catch(console.error);
            void fetchHomeOverview();
          }, 250);
          break;
        }

        case "SessionChanged": {
          const nextAccount = event.payload?.user?.id || "default";
          if (nextAccount !== libraryAccountRef.current) {
            resetNavigation();
            setHomeOverview(null);
            setIsPlaylistModalOpen(false); setPlaylistModalTrack(null);
            setPlayingPlaylistId(null);
            setPlayingOrigin(origin => origin?.type === "playlist" ? null : origin);
          }
          ++userQueryRevision.current;
          libraryAccountRef.current = nextAccount;
          ++playlistRequest.current; ++membershipRequest.current;
          setPlaylists([]); setTrackPlaylistMap({});
          setWishlist([]); setDownloads([]);
          void fetchWishlist(); void fetchDownloads();
          recommendationPreferences.reset();
          void recommendationPreferences.refresh().catch(console.error);
          currentUserIdRef.current = event.payload?.user?.id || "default";
          ++chartRequestRef.current; setWorldTrending([]); setIsTrendingLoading(true); setTrendingError(null); setRecommendationNotice(null);
          ++discoveryRequestRef.current;
          setDiscoveryRecs([]);
          clearTimeout(tasteRefreshTimer);
          void fetchDiscovery(false, false);
          ++homeRequestRef.current;
          ++flowSessionRequestRef.current;
          void fetchHomeOverview();
          void fetchFlowSession().catch(console.error);
          const user = event.payload?.user;
          if (user) {
            setCurrentUser(user);
            fetchPlaylists();
            fetchTrackPlaylistMemberships();
          } else {
            setCurrentUser(null);
            setPlaylists((prev) => prev.filter((p) => p.is_smart_mix === 1));
            setTrackPlaylistMap({});
          }
          break;
        }

        case "UserProfileUpdated": {
          const user = event.payload?.user;
          if (user) {
            setCurrentUser(user);
          }
          break;
        }

        case "UserLoggedOut": {
          window.location.reload();
          break;
        }

        case "PlaylistsUpdated": {
          fetchPlaylists();
          fetchTrackPlaylistMemberships();
          fetchTracks();
          break;
        }

        default:
          break;
      }
    }), () => {
      // Reconcile events missed while disconnected through guarded query owners.
      playbackSessionId.current = null;
      void fetchPlaybackState();
      void fetchCurrentUser();
      void fetchTracks(); void fetchAlbums();
      void fetchPlaylists(); void fetchTrackPlaylistMemberships();
      void fetchWishlist(); void fetchDownloads();
      void fetchDiscovery(); void fetchHomeOverview();
    }, setConnectionError);
    retryConnection.current = owner.retry;

    return () => {
      subscriptionDisposed = true;
      clearTimeout(tasteRefreshTimer);
      ++discoveryRequestRef.current;
      owner.dispose();
    };
  }, [
    fetchPersonalRecommendations,
    fetchDiscovery,
    fetchTracks,
    fetchAlbums,
    fetchOnboardingStatus,
    fetchDownloads,
    fetchWishlist,
    addAppNotification,
    fetchPlaylists,
    resetNavigation,
    fetchTrackPlaylistMemberships,
    fetchFlowSession,
    fetchHomeOverview,
    fetchPlaybackState,
  ]);

  // --- Actions & Commands ---

  // Local Library and Kaze Downloads: each plays as its own ordered list.
  const { localPlayback, downloadsPlayback: kazeDownloadsPlayback, downloadsPage, downloadsListState } = useDownloadsSection({
    playbackState, fetchPlaybackState, beginPlaybackLoading, endPlaybackLoading, downloads,
    userId: currentUser?.id || "default", navigation: destination.view,
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

  const handleLike = async (trackId: string) => {
    setTracks((prev) =>
      prev.map((t) => (t.id === trackId ? { ...t, manual_like: 1 } : t)),
    );
    setPlaybackState((prev) =>
      prev.current_track?.id === trackId
        ? { ...prev, current_track: { ...prev.current_track, manual_like: 1 } }
        : prev,
    );
    try {
      await dispatchCommand({
        command: "LikeTrack",
        payload: { track_id: trackId },
      });
      await Promise.all([
        fetchPlaylists(),
        fetchTrackPlaylistMemberships(),
        fetchHomeOverview(),
      ]);
      // If Liked Songs is the active collection, refresh its tracks
      if (
        activeCollection &&
        (activeCollection.title === "Liked Songs" ||
          (likedPlaylistId &&
            (activeCollection.playlistId === likedPlaylistId ||
              activeCollection.id === likedPlaylistId)))
      ) {
        const plId =
          activeCollection.playlistId || activeCollection.id || likedPlaylistId;
        if (plId) {
          const res = await executeQuery({
            query: "GetPlaylistTracks",
            payload: { playlist_id: plId },
          });
          const plTracks: Track[] = (res.data as any) || [];
          const items: CollectionTrackItem[] = plTracks.map((t) => {
            const isOnline = !!isOnlineTrack(t);
            return {
              id: t.id,
              title: t.title,
              artist: t.artist_name || "Unknown Artist",
              album: t.album_title,
              duration_secs: t.duration_secs ?? 0,
              cover_art_url: t.cover_art_url,
              preview_url: t.preview_url,
              is_downloaded: !isOnline,
              matched_local_track_id: isOnline ? undefined : t.id,
              rawLocalTrack: isOnline ? undefined : t,
            };
          });
          setActiveCollection((prev) =>
            prev
              ? { ...prev, tracks: items, subtitle: `${items.length} tracks` }
              : prev,
          );
        }
      }
    } catch (err: any) {
      console.error("Failed to like track:", err);
      setTracks((prev) =>
        prev.map((t) => (t.id === trackId ? { ...t, manual_like: 0 } : t)),
      );
      setPlaybackState((prev) =>
        prev.current_track?.id === trackId
          ? {
              ...prev,
              current_track: { ...prev.current_track, manual_like: 0 },
            }
          : prev,
      );
      addAppNotification(
        "error",
        "Like Failed",
        err?.message || "Could not like track.",
      );
    }
  };

  const handleRemoveFeedback = async (trackId: string) => {
    setTracks((prev) =>
      prev.map((t) => (t.id === trackId ? { ...t, manual_like: 0 } : t)),
    );
    setPlaybackState((prev) =>
      prev.current_track?.id === trackId
        ? { ...prev, current_track: { ...prev.current_track, manual_like: 0 } }
        : prev,
    );
    try {
      await dispatchCommand({
        command: "RemoveTrackFeedback",
        payload: { track_id: trackId },
      });
      await Promise.all([
        fetchPlaylists(),
        fetchTrackPlaylistMemberships(),
        fetchHomeOverview(),
      ]);
      // If Liked Songs is the active collection, remove the unliked track immediately
      setActiveCollection((prev) => {
        if (!prev || !prev.tracks) return prev;
        const isLikedSongs =
          prev.title === "Liked Songs" ||
          (likedPlaylistId &&
            (prev.playlistId === likedPlaylistId ||
              prev.id === likedPlaylistId));
        if (!isLikedSongs) return prev;
        const updatedTracks = prev.tracks.filter(
          (t) => t.id !== trackId && t.matched_local_track_id !== trackId,
        );
        return {
          ...prev,
          tracks: updatedTracks,
          subtitle: `${updatedTracks.length} tracks`,
        };
      });
    } catch (err: any) {
      console.error("Failed to remove feedback:", err);
      setTracks((prev) =>
        prev.map((t) => (t.id === trackId ? { ...t, manual_like: 1 } : t)),
      );
      setPlaybackState((prev) =>
        prev.current_track?.id === trackId
          ? {
              ...prev,
              current_track: { ...prev.current_track, manual_like: 1 },
            }
          : prev,
      );
      addAppNotification(
        "error",
        "Action Failed",
        err?.message || "Could not update track rating.",
      );
    }
  };

  const {
    handleEnqueueTrack, handleEnqueueRecommendation, handlePlayQueueIndex, handleDequeueTrack, handleClearQueue,
  } = useQueue({ queuedTrackIds, setQueuedTrackIds, addAppNotification });

  const handleSearchLibrary = useCallback(
    (queryText: string) => {
      updateDestination((current) => ({
        ...current,
        libraryQuery: queryText.trim(),
        libraryAlbumId: undefined,
      }));
    },
    [updateDestination],
  );

  const handleSelectAlbum = useCallback(
    (albumId: string) => {
      const album = albums.find((al) => al.id === albumId);
      const moment = homeOverview?.library_moment;
      navigate({
        ...viewDestination("library"),
        libraryQuery: album?.title || (moment?.album_id === albumId ? moment.album_title : ""),
        libraryAlbumId: albumId,
      });
    },
    [albums, homeOverview?.library_moment?.album_id, homeOverview?.library_moment?.album_title, navigate],
  );

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


  // Online likes are represented by membership in the account's Liked Songs playlist.
  const handleLikeOnline = async (track: {
    id: string;
    title: string;
    artist: string;
    album?: string;
    cover_art_url?: string;
    preview_url?: string;
    duration_secs?: number;
  }) => {
    try {
      const res = await dispatchCommand({
        command: "EnsureLikedSongsPlaylist",
      });
      const likedId = (res as any)?.data;
      if (likedId) {
        const isLiked = (trackPlaylistMap[track.id] || []).includes(likedId);
        if (isLiked) {
          await dispatchCommand({
            command: "RemoveTrackFromPlaylist",
            payload: { playlist_id: likedId, track_id: track.id },
          });
          setTrackPlaylistMap((prev) => {
            const next = { ...prev };
            if (next[track.id]) {
              next[track.id] = next[track.id].filter((id) => id !== likedId);
            }
            return next;
          });
          setActiveCollection((prev) => {
            if (!prev || !prev.tracks) return prev;
            const isLikedSongs =
              prev.title === "Liked Songs" ||
              prev.playlistId === likedId ||
              prev.id === likedId;
            if (!isLikedSongs) return prev;
            const updatedTracks = prev.tracks.filter((t) => t.id !== track.id);
            return {
              ...prev,
              tracks: updatedTracks,
              subtitle: `${updatedTracks.length} tracks`,
            };
          });
        } else {
          await dispatchCommand({
            command: "AddTrackToPlaylist",
            payload: {
              playlist_id: likedId,
              track_id: track.id,
              title: track.title,
              artist: track.artist,
              album: track.album,
              duration_secs: track.duration_secs,
              cover_art_url: track.cover_art_url,
              preview_url: track.preview_url,
            },
          });
          setTrackPlaylistMap((prev) => {
            const next = { ...prev };
            const existing = next[track.id] || [];
            if (!existing.includes(likedId)) {
              next[track.id] = [...existing, likedId];
            }
            return next;
          });
        }
        await fetchPlaylists();
        await fetchTrackPlaylistMemberships();
      }
    } catch (err: any) {
      console.error("Failed to toggle online like:", err);
      addAppNotification(
        "error",
        "Like Failed",
        err?.message || "Could not update online track like.",
      );
    }
  };

  const handleToggleCollectionLike = async (
    track: CollectionTrackItem,
    isLiked: boolean,
  ) => {
    const localId =
      (track.matched_local_track_id &&
      !track.matched_local_track_id.startsWith("online:") &&
      !track.matched_local_track_id.startsWith("itunes:")
        ? track.matched_local_track_id
        : undefined) ||
      (!track.id.startsWith("online:") &&
      !track.id.startsWith("itunes:") &&
      tracks.some((t) => t.id === track.id)
        ? track.id
        : undefined);

    const isLikedSongsView =
      activeCollection?.title === "Liked Songs" ||
      (likedPlaylistId &&
        (activeCollection?.playlistId === likedPlaylistId ||
          activeCollection?.id === likedPlaylistId));

    try {
      if (localId) {
        if (isLiked) {
          await handleRemoveFeedback(localId);
        } else {
          await handleLike(localId);
        }
      } else {
        await handleLikeOnline({
          id: track.id,
          title: track.title,
          artist: track.artist,
          album: track.album,
          cover_art_url: track.cover_art_url,
          preview_url: track.preview_url,
          duration_secs: track.duration_secs,
        });
      }

      if (isLiked && isLikedSongsView) {
        setActiveCollection((prev) => {
          if (!prev || !prev.tracks) return prev;
          const updatedTracks = prev.tracks.filter(
            (t) =>
              t.id !== track.id &&
              (!localId ||
                (t.matched_local_track_id !== localId && t.id !== localId)),
          );
          return {
            ...prev,
            tracks: updatedTracks,
            subtitle: `${updatedTracks.length} tracks`,
          };
        });
      }
    } catch (err: any) {
      console.error("Failed to toggle collection like:", err);
      addAppNotification(
        "error",
        "Like Failed",
        err?.message || "Could not update track like state.",
      );
    }
  };

  // Metadata-only legacy wishlist entries open explicit search, never select a recording silently.
  const handleInitiateDirectDownloadSearch = (artist: string, title: string) => {
    navigate({ ...viewDestination("discovery"), searchQuery: `${title} ${artist}`, searchResults: [] });
    setSearchRefreshRevision(revision => revision + 1);
  };

  const handleGlobalOnlineSearch = async (queryOverride?: string) => {
    const q = (
      typeof queryOverride === "string" ? queryOverride : globalSearchQuery
    ).trim();
    navigate({ ...viewDestination("discovery"), searchQuery: q, searchResults: q ? [] : null });
    setSearchRefreshRevision((revision) => revision + 1);
  };

  useEffect(() => {
    const q = destination.searchQuery;
    if (
      !q ||
      destination.view !== "discovery" ||
      destination.collection ||
      destination.lyrics
    ) {
      setIsGlobalSearching(false);
      return;
    }
    let cancelled = false;
    setIsGlobalSearching(true);
    const load = async () => {
      try {
        const res = await executeQuery({
          query: "SearchOnlineMusic",
          payload: { query: q, limit: 35, provider: "jiosaavn" },
        });
        let incoming: DiscoveryRecommendation[] = [];
        if (res && Array.isArray(res.data)) {
          incoming = res.data;
        }

        // Relevant results filter: Keep items that directly match query words or artist/title
        const qLower = q.toLowerCase();
        const qTokens = qLower.split(/\s+/).filter((t) => t.length > 1);

        const relevant = incoming.filter((r) => {
          const titleLower = r.title.toLowerCase();
          const artistLower = r.artist.toLowerCase();
          const albumLower = r.album ? r.album.toLowerCase() : "";

          // Direct substring match
          if (
            titleLower.includes(qLower) ||
            artistLower.includes(qLower) ||
            albumLower.includes(qLower)
          ) {
            return true;
          }

          // Token match: title or artist contains any significant token
          return qTokens.some(
            (tok) => titleLower.includes(tok) || artistLower.includes(tok),
          );
        });

        if (!cancelled) void recommendationPreferences.refresh().catch(console.error);
        updateDestination((current) =>
          !cancelled &&
          current.view === "discovery" &&
          current.searchQuery === q
            ? {
                ...current,
                searchResults: relevant.length > 0 ? relevant : incoming,
              }
            : current,
        );
      } catch (err) {
        console.error("Global online music search failed:", err);
        if (!cancelled) addAppNotification("error", "Online search unavailable", typeof err === "string" ? err : "Couldn’t reach online music. Please try again.");
        updateDestination((current) =>
          !cancelled &&
          current.view === "discovery" &&
          current.searchQuery === q
            ? { ...current, searchResults: [] }
            : current,
        );
      } finally {
        if (!cancelled) setIsGlobalSearching(false);
      }
    };
    const timer = setTimeout(() => void load(), 200);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [
    destination.searchQuery,
    destination.view,
    destination.collection,
    destination.lyrics,
    searchRefreshRevision,
    updateDestination,
    addAppNotification,
  ]);

  const handleGlobalClearSearch = () => {
    navigate(viewDestination("discovery"));
  };

  const handleGlobalRefresh = async () => {
    setIsGlobalRefreshing(true);
    try {
      if (globalSearchQuery.trim()) {
        await handleGlobalOnlineSearch(globalSearchQuery.trim());
      }
      await Promise.allSettled([
        fetchDiscovery(true),
        fetchHomeOverview(),
        fetchTracks(),
        fetchAlbums(),
      ]);
    } finally {
      setIsGlobalRefreshing(false);
    }
  };

  const unreadNotificationsCount = notifications.filter((n) => !n.read).length;

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
  const activePlayingArtwork = useMemo(() => {
    // 1. Direct local cover extracted from audio file or cache
    if (currentTrackCoverUrl) {
      return currentTrackCoverUrl;
    }
    // 2. Track's own cover_art_url property
    if (playbackState.current_track?.cover_art_url) {
      return playbackState.current_track.cover_art_url;
    }
    // 3. Track in loaded library tracks
    if (playbackState.current_track?.id) {
      const matchInLibrary = tracks.find(
        (t) => t.id === playbackState.current_track?.id,
      );
      if (matchInLibrary?.cover_art_url) {
        return matchInLibrary.cover_art_url;
      }
    }
    // 4. Track in activeCollection (playlist, mix, collection)
    if (activeCollection?.tracks && playbackState.current_track) {
      const matchInCollection = activeCollection.tracks.find(
        (t) =>
          t.matched_local_track_id === playbackState.current_track?.id ||
          t.id === playbackState.current_track?.id ||
          (t.title.toLowerCase().trim() ===
            playbackState.current_track?.title.toLowerCase().trim() &&
            t.artist.toLowerCase().trim() ===
              playbackState.current_track?.artist_name?.toLowerCase().trim()),
      );
      if (matchInCollection?.cover_art_url) {
        return matchInCollection.cover_art_url;
      }
    }
    // 5. Track in discovery/trending recommendations
    if (playbackState.current_track) {
      const matchInDiscovery = discoveryRecs.find(
        (r) =>
          r.matched_local_track_id === playbackState.current_track?.id ||
          r.external_track_id === playbackState.current_track?.id ||
          (r.title.toLowerCase().trim() ===
            playbackState.current_track?.title.toLowerCase().trim() &&
            r.artist.toLowerCase().trim() ===
              playbackState.current_track?.artist_name?.toLowerCase().trim()),
      );
      if (matchInDiscovery?.cover_art_url) {
        return matchInDiscovery.cover_art_url;
      }
    }
    // 6. Track's album cover in albums list
    if (playbackState.current_track?.album_title) {
      const matchAlbum = albums.find(
        (a) =>
          a.title.toLowerCase().trim() ===
          playbackState.current_track?.album_title?.toLowerCase().trim(),
      );
      if (matchAlbum?.cover_art_path) {
        return matchAlbum.cover_art_path;
      }
    }
    return null;
  }, [
    currentTrackCoverUrl,
    playbackState.current_track,
    tracks,
    activeCollection?.tracks,
    discoveryRecs,
    albums,
  ]);

  const likedPlaylistId = playlists.find(
    (playlist) => playlist.name === "Liked Songs",
  )?.id;
  const likedTrackIds = useMemo(() => {
    const set = new Set<string>();
    if (likedPlaylistId) {
      for (const [trackId, playlistIds] of Object.entries(trackPlaylistMap)) {
        if (playlistIds.includes(likedPlaylistId)) {
          set.add(trackId);
        }
      }
    }
    for (const track of tracks) {
      if (track.manual_like === 1) {
        set.add(track.id);
      }
    }
    return set;
  }, [trackPlaylistMap, likedPlaylistId, tracks]);

  const isCurrentTrackLiked = useMemo(() => {
    if (!playbackState.current_track) return false;
    const tid = playbackState.current_track.id;
    return (
      likedTrackIds.has(tid) || playbackState.current_track.manual_like === 1
    );
  }, [playbackState.current_track, likedTrackIds]);

  const regularPlaylistIds = useMemo(
    () =>
      new Set(
        playlists
          .filter(
            (playlist) =>
              playlist.is_smart_mix !== 1 && playlist.name !== "Liked Songs",
          )
          .map((playlist) => playlist.id),
      ),
    [playlists],
  );

  const regularTrackPlaylistMap = useMemo(() => {
    const filtered: Record<string, string[]> = {};
    for (const [trackId, playlistIds] of Object.entries(trackPlaylistMap)) {
      filtered[trackId] = playlistIds.filter((playlistId) =>
        regularPlaylistIds.has(playlistId),
      );
    }
    return filtered;
  }, [trackPlaylistMap, regularPlaylistIds]);

  const isCurrentTrackInPlaylist = useMemo(() => {
    if (!playbackState.current_track) return false;
    const tid = playbackState.current_track.id;
    const altId = `online:${playbackState.current_track.artist_name || ""}-${playbackState.current_track.title}`;
    return (
      (tid &&
        regularTrackPlaylistMap[tid] &&
        regularTrackPlaylistMap[tid].length > 0) ||
      (altId &&
        regularTrackPlaylistMap[altId] &&
        regularTrackPlaylistMap[altId].length > 0) ||
      false
    );
  }, [playbackState.current_track, regularTrackPlaylistMap]);

  const handleSelectView = useCallback(
    (view: import("./viewTypes").ViewType) => {
      if (view === "stats") setStatsRefreshTrigger((prev) => prev + 1);
      navigate(viewDestination(view));
    },
    [navigate],
  );
  const handleSelectSidebarPlaylist = useCallback(
    (playlist: Playlist) => {
      handleOpenCollection({
        id: playlist.id,
        type: playlist.is_smart_mix === 1 ? "mix" : "playlist",
        title: playlist.name,
        subtitle:
          playlist.description ||
          (playlist.is_smart_mix === 1
            ? "Custom algorithmic smart mix"
            : "User Playlist"),
        tag: playlist.is_smart_mix === 1 ? "SMART MIX" : "PUBLIC PLAYLIST",
        playlistId: playlist.id,
        cover_art_url: playlist.cover_art_url,
      });
    },
    [handleOpenCollection],
  );
  const handleOpenQueue = useCallback(() => {
    handleToggleFullscreen(false);
    navigate(viewDestination("queue"));
  }, [handleToggleFullscreen, navigate]);
  const handleOpenNowPlaying = useCallback(
    () => navigate(viewDestination("now_playing")),
    [navigate],
  );

  const likedSongs = playlists.find(playlist => playlist.name === "Liked Songs" && !playlist.is_smart_mix);
  const handleHomeLiked = useCallback(() => { if (likedSongs) handleSelectSidebarPlaylist(likedSongs); }, [likedSongs, handleSelectSidebarPlaylist]);
  const handleHomeDiscover = useCallback(() => handleSelectView("discovery"), [handleSelectView]);
  const isArchiveLiked =
    interfacePreference.style === "brutalist" &&
    activeCollection?.type === "playlist" &&
    (activeCollection.title === "Liked Songs" ||
      (!!likedSongs &&
        (activeCollection.playlistId === likedSongs.id ||
          activeCollection.id === likedSongs.id)));
  const isArchiveSurface =
    !isLyricsActive &&
    (isArchiveLiked ||
      (!activeCollection && ["library", "downloads", "albums", "playlists"].includes(currentView)));
  const archiveCount = isArchiveLiked
    ? activeCollection?.tracks?.length ?? likedSongs?.track_count ?? 0
    : currentView === "albums"
      ? albumPage.total
      : currentView === "playlists"
        ? regularPlaylistIds.size
        : currentView === "downloads"
          ? downloadsPage.total
          : libraryScoped ? tracks.length : trackPage.total;

  const handleDeckFlow = useCallback(() => {
    if (homeOverview?.flow_session?.is_active) void handleToggleFlowPlayback();
    else void handleStartFlow(flowSteering);
  }, [homeOverview?.flow_session?.is_active, handleToggleFlowPlayback, handleStartFlow, flowSteering]);
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
            <div className="interface-toolbar">
              <div className="history-controls">
                <button
                  className="player-icon-btn"
                  aria-label="Go back"
                  disabled={!canGoBack}
                  onClick={navigateBack}
                >
                  <ArrowLeft size={18} />
                </button>
                <button
                  className="player-icon-btn"
                  aria-label="Go forward"
                  disabled={!canGoForward}
                  onClick={navigateForward}
                >
                  <ArrowRight size={18} />
                </button>
              </div>
              <span className="interface-name">
                {interfacePreference.style === "spatial"
                  ? "Spatial"
                  : "Brutalist"}
              </span>
              <span className="interface-route">
                {!isLyricsActive && (isArchiveLiked ? "Library" : activeCollection?.title ||
                  (currentView === "discovery" ? "Discover" : ["library", "downloads", "albums", "playlists"].includes(currentView) ? "Library" : currentView === "stats" ? "Profile" : currentView.replace(/_/g, " ")))}
              </span>
            {/* Persistent Global Top Search Bar (hidden on stats/profile view, lyrics active, or offline discovery) */}
            {(!activeCollection && currentView === "discovery" && !isOnline) ||
            isLyricsActive ||
            currentView === "stats" ? null : (
              <GlobalTopSearchBar
                placeholder={interfacePreference.style === "spatial" ? "Find your next favourite" : undefined}
                searchQuery={globalSearchQuery}
                setSearchQuery={setGlobalSearchQuery}
                onSearch={handleGlobalOnlineSearch}
                onRefresh={handleGlobalRefresh}
                isSearching={isGlobalSearching}
                isRefreshing={isGlobalRefreshing}
              />
            )}
              {interfacePreference.style === "spatial" && <SpatialUtilities onNavigate={handleSelectView} unread={unreadNotificationsCount} />}
            </div>
            {interfacePreference.style === "spatial" && !activeCollection && !isLyricsActive && ["library", "downloads", "albums", "playlists"].includes(currentView) && <SpatialLibraryHeader view={currentView} albums={albumPage.total} tracks={currentView === "downloads" ? downloadsPage.total : libraryScoped ? tracks.length : trackPage.total} onNavigate={handleSelectView} onLiked={likedSongs ? handleHomeLiked : undefined} />}

            {interfacePreference.style === "brutalist" && isArchiveSurface && <BrutalistLibraryHeader view={currentView} count={archiveCount} likedSelected={isArchiveLiked} onNavigate={handleSelectView} onLiked={likedSongs ? handleHomeLiked : undefined} />}

            <ViewMemoryProvider>
            <Suspense fallback={null}>
              {isLyricsActive ? (
                <div className="lyrics-stage">
                  <LyricsView
                    trackId={activePlayingTrackId}
                    artist={activePlayingArtist}
                    title={activePlayingTitle}
                    durationSecs={activePlayingDuration}
                    onSeek={handleUnifiedSeek}
                    isFullScreen={false}
                  />
                </div>
              ) : activeCollection ? (
                <CollectionDetailView
                  key={`${activeCollection.type}:${activeCollection.id}`}
                  collection={activeCollection}
                  withinArchive={isArchiveLiked}
                  isLoadingTracks={isLoadingCollectionTracks}
                  loadingTrackId={loadingTrackId}
                  onBack={() => {
                    if (!navigateBack())
                      navigate({ ...destination, collection: null });
                  }}
                  onPlayTrack={handlePlayCollectionTrack}
                  onPlayAll={handlePlayAllCollection}
                  onShuffleAll={handleShuffleCollection}
                  onAddToWishlist={(track) =>
                    handleAddToWishlist(track.title, track.artist, track.album)
                  }
                  onSaveToPlaylists={handleSaveCollectionToPlaylists}
                  currentPlayingTrackId={
                    playbackState.is_playing
                      ? playbackState.current_track?.id
                      : undefined
                  }
                  isPlaying={isPlayingThisCollection}
                  isSaved={isCollectionSaved}
                  isShuffled={playbackState.is_shuffled}
                  trackPlaylistMap={regularTrackPlaylistMap}
                  userPlaylistIds={regularPlaylistIds}
                  onAddToPlaylist={(track) =>
                    handleOpenAddToPlaylistModal({
                      id:
                        track.matched_local_track_id &&
                        !track.matched_local_track_id.startsWith("online:") &&
                        !track.matched_local_track_id.startsWith("itunes:")
                          ? track.matched_local_track_id
                          : track.id,
                      title: track.title,
                      artist: track.artist,
                      album: track.album,
                      cover_art_url: track.cover_art_url,
                    })
                  }
                  onRenamePlaylist={handleRenamePlaylist}
                  onDeletePlaylist={handleDeletePlaylist}
                  likedTrackIds={likedTrackIds}
                  onToggleLike={handleToggleCollectionLike}
                  queuedTrackIds={queuedTrackIds}
                  onEnqueueTrack={(track) =>
                    handleEnqueueRecommendation(
                      track.rawRecommendation || {
                        external_track_id: track.id,
                        provider: track.matched_local_track_id
                          ? "library"
                          : "online",
                        provider_id: track.id,
                        title: track.title,
                        artist: track.artist,
                        album: track.album,
                        duration_secs: track.duration_secs,
                        cover_art_url: track.cover_art_url,
                        preview_url: track.preview_url,
                        matched_local_track_id: track.matched_local_track_id,
                        match_status: track.matched_local_track_id
                          ? "EXACT_MATCH"
                          : "NOT_FOUND",
                        recommendation_reason: "From collection",
                        in_wishlist: false,
                      },
                    )
                  }
                />
              ) : (
                <>
                  {currentView === "home" && (
                    <HomeView
                      overview={homeOverview}
                      isLoading={isHomeLoading}
                      flowPendingAction={flowPendingAction}
                      onRefresh={fetchHomeOverview}
                      onStartFlow={handleStartFlow}
                      onTogglePlayback={handleUnifiedPlayPause}
                      onToggleFlowPlayback={handleToggleFlowPlayback}
                      currentTrack={playbackState.current_track}
                      currentCover={activePlayingArtwork}
                      previousTrack={previousTrackRef.current}
                      nextTrack={nextQueuedTrack}
                      onPreviousTrack={handlePreviousTrack}
                      onNextTrack={handleNextTrack}
                      onPlayTrack={handlePlayTrack}
                      onEnqueueTrack={handleEnqueueTrack}
                      onOpenAlbum={handleSelectAlbum}
                      loadingTrackId={loadingTrackId}
                      isPlaying={playbackState.is_playing}
                      flowSteering={flowSteering}
                      onFlowSteeringChange={handleFlowSteeringChange}
                      onDiscover={handleHomeDiscover}
                      queuedTrackIds={queuedTrackIds}
                    />
                  )}

                  {currentView === "library" && (
                    <LibraryView
                      tracks={tracks}
                      total={libraryScoped ? tracks.length : trackPage.total}
                      onEndReached={libraryScoped ? undefined : trackPage.next}
                      loading={libraryScoped ? scoped.loading : trackPage.loading}
                      pagingError={libraryScoped ? scoped.error : trackPage.error}
                      onRetry={libraryScoped ? scoped.refresh : trackPage.retry}
                      currentTrackId={playbackState.current_track?.id}
                      isPlaying={playbackState.is_playing}
                      queuedTrackIds={queuedTrackIds}
                      onPlayTrack={handleLibraryRowPlay}
                      onPlayAll={handleLibraryPlayAll}
                      listState={downloadsListActive ? (playbackState.is_playing ? "playing" : "paused") : "idle"}
                      shuffleEnabled={libraryShuffle}
                      onToggleShuffle={handleLibraryToggleShuffle}
                      loadingTrackId={loadingTrackId}
                      onEnqueueTrack={handleEnqueueTrack}
                      onLikeTrack={handleLike}
                      onRemoveFeedback={handleRemoveFeedback}
                      onRescan={handleRescanLibrary}
                      onSearch={handleSearchLibrary}
                      searchQuery={destination.libraryQuery}
                      trackPlaylistMap={regularTrackPlaylistMap}
                      likedTrackIds={likedTrackIds}
                      onOpenAddToPlaylistModal={(t) =>
                        handleOpenAddToPlaylistModal({
                          id: t.id,
                          title: t.title,
                          artist: t.artist_name,
                          album: t.album_title,
                        })
                      }
                    />
                  )}

                  {currentView === "albums" && (
                    <AlbumsView
                      albums={albums}
                      total={albumPage.total}
                      onEndReached={albumPage.next}
                      loading={albumPage.loading}
                      pagingError={albumPage.error}
                      onRetry={albumPage.retry}
                      onSelectAlbum={handleSelectAlbum}
                    />
                  )}

                  {currentView === "playlists" && (
                    <PlaylistsView
                      viewMode="playlists"
                      playlists={playlists}
                      currentUser={currentUser}
                      onOpenAuthModal={() => setIsAuthModalOpen(true)}
                      activePlaylistId={playingPlaylistId}
                      onPlayPlaylist={handlePlayPlaylist}
                      onCreatePlaylist={handleCreatePlaylist}
                      onRefreshPlaylists={handleRefreshPlaylists}
                      onInspectJioSaavnPlaylist={handleInspectJioSaavnPlaylist}
                      onSaveImportedPlaylist={handleSaveImportedPlaylist}
                      onAddMissingToWishlist={handleAddMissingToWishlist}
                      onSearchDirect={handleInitiateDirectDownloadSearch}
                      onFetchPlaylistTracks={handleFetchPlaylistTracks}
                      onPlayTrack={handlePlayTrack}
                      loadingTrackId={loadingTrackId}
                      queuedTrackIds={queuedTrackIds}
                      onEnqueueTrack={handleEnqueueTrack}
                      onOpenCollection={handleOpenCollection}
                      onRenamePlaylist={handleRenamePlaylist}
                      onDeletePlaylist={handleDeletePlaylist}
                    />
                  )}

                  {currentView === "smart_mixes" && (
                    <PlaylistsView
                      viewMode="smart_mixes"
                      playlists={playlists}
                      currentUser={currentUser}
                      onOpenAuthModal={() => setIsAuthModalOpen(true)}
                      activePlaylistId={playingPlaylistId}
                      onPlayPlaylist={handlePlayPlaylist}
                      onCreatePlaylist={handleCreatePlaylist}
                      onRefreshPlaylists={handleRefreshPlaylists}
                      onInspectJioSaavnPlaylist={handleInspectJioSaavnPlaylist}
                      onSaveImportedPlaylist={handleSaveImportedPlaylist}
                      onAddMissingToWishlist={handleAddMissingToWishlist}
                      onSearchDirect={handleInitiateDirectDownloadSearch}
                      onFetchPlaylistTracks={handleFetchPlaylistTracks}
                      onPlayTrack={handlePlayTrack}
                      loadingTrackId={loadingTrackId}
                      queuedTrackIds={queuedTrackIds}
                      onEnqueueTrack={handleEnqueueTrack}
                      onOpenCollection={handleOpenCollection}
                      onRenamePlaylist={handleRenamePlaylist}
                      onDeletePlaylist={handleDeletePlaylist}
                    />
                  )}

                  {currentView === "discovery" && (
                    <DiscoveryView
                      recommendations={discoveryRecs}
                      worldTrending={worldTrending}
                      localTracks={tracks}
                      playlists={playlists}
                      onPlayPlaylist={handlePlayPlaylist}
                      onAddToWishlist={(rec) =>
                        handleAddToWishlist(rec.title, rec.artist, rec.album)
                      }
                      onSearchDirect={(artist, title) =>
                        handleInitiateDirectDownloadSearch(artist, title)
                      }
                      isTrendingLoading={isTrendingLoading}
                      trendingError={trendingError}
                      onRefreshTrending={refreshTrending}
                      onRefresh={refreshPersonalRecommendations}
                      onPlayOnlineTrack={handlePlayOnlineTrack}
                      onStopTrack={handleStopTrack}
                      activeOnlineTrackId={
                        playbackState.current_track?.id || null
                      }
                      isOnlinePlaying={playbackState.is_playing}
                      loadingTrackId={loadingTrackId}
                      currentLocalTrack={playbackState.current_track}
                      isLocalPlaying={playbackState.is_playing}
                      onOpenCollection={handleOpenCollection}
                      onPlayCollection={handlePlayDiscoveredCollection}
                      downloads={downloads}
                      searchQuery={globalSearchQuery}
                      setSearchQuery={setGlobalSearchQuery}
                      searchResults={currentSearchResults}
                      setSearchResults={setGlobalSearchResults}
                      isSearchingOnline={isGlobalSearching}
                      onSearchOnline={handleGlobalOnlineSearch}
                      onClearSearch={handleGlobalClearSearch}
                      trackPlaylistMap={regularTrackPlaylistMap}
                      onAddToPlaylist={(rec) =>
                        handleOpenAddToPlaylistModal({
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
                      onGoToLibrary={() => navigate(viewDestination("library"))}
                      queuedTrackIds={queuedTrackIds}
                      onEnqueueTrack={handleEnqueueRecommendation}
                      likedTrackIds={likedTrackIds}
                    />
                  )}

                  {currentView === "now_playing" && (interfacePreference.style === "brutalist" ? <BrutalistPoster albumId={albums.find(a => a.title === playbackState.current_track?.album_title && a.artist_name === playbackState.current_track?.artist_name)?.id} track={playbackState.current_track} cover={activePlayingArtwork} index={playbackState.current_queue_index} playing={playbackState.is_playing} loading={loadingTrackId !== null || playbackState.is_buffering} entries={queueEntries} tracks={tracks} onPlayIndex={handlePlayQueueIndex} onOpenAlbum={handleSelectAlbum} /> :
                    <PlaybackStage
                      track={playbackState.current_track}
                      cover={activePlayingArtwork}
                      previous={previousTrackRef.current}
                      next={nextQueuedTrack}
                      index={playbackState.current_queue_index}
                      playing={playbackState.is_playing}
                      loading={
                        loadingTrackId !== null || playbackState.is_buffering
                      }
                      onPrevious={handlePreviousTrack}
                      onNext={handleNextTrack}
                    />
                  )}
                  {currentView === "queue" && (
                    <QueueView
                      entries={queueEntries}
                      tracks={tracks}
                      currentTrack={playbackState.current_track}
                      onPlayIndex={handlePlayQueueIndex}
                      onRemove={handleDequeueTrack}
                      onClear={handleClearQueue}
                    />
                  )}
                  {currentView === "downloads" && (
                    <DownloadsView
                      downloads={downloads}
                      onSearchDownloads={handleSearchDownloads}
                      onStartDownload={handleStartDownload}
                      onCancelDownload={handleCancelDownload}
                      onRefreshDownloads={fetchDownloads}
                      folder={settings.downloads?.resolved_dir}
                      songs={
                        <LibraryView
                          tracks={downloadsPage.items}
                          total={downloadsPage.total}
                          onEndReached={downloadsPage.next}
                          loading={downloadsPage.loading}
                          pagingError={downloadsPage.error}
                          onRetry={downloadsPage.retry}
                          currentTrackId={playbackState.current_track?.id}
                          isPlaying={playbackState.is_playing}
                          queuedTrackIds={queuedTrackIds}
                          onPlayTrack={kazeDownloadsPlayback.handleLibraryRowPlay}
                          onPlayAll={kazeDownloadsPlayback.handleLibraryPlayAll}
                          listState={downloadsListState}
                          shuffleEnabled={kazeDownloadsPlayback.libraryShuffle}
                          onToggleShuffle={kazeDownloadsPlayback.handleLibraryToggleShuffle}
                          loadingTrackId={loadingTrackId}
                          onEnqueueTrack={handleEnqueueTrack}
                          onLikeTrack={handleLike}
                          onRemoveFeedback={handleRemoveFeedback}
                          trackPlaylistMap={regularTrackPlaylistMap}
                          likedTrackIds={likedTrackIds}
                          downloadControls
                          emptyTitle="No downloaded songs yet"
                          emptyHint="Use the download button on any online song. It is saved to Kaze's folder and appears here."
                          onOpenAddToPlaylistModal={(t) =>
                            handleOpenAddToPlaylistModal({
                              id: t.id,
                              title: t.title,
                              artist: t.artist_name,
                              album: t.album_title,
                            })
                          }
                        />
                      }
                    />
                  )}
                  {currentView === "wishlist" && (
                    <WishlistView
                      wishlist={wishlist}
                      onAddToWishlist={handleAddToWishlist}
                      onUpdateStatus={handleUpdateWishlistStatus}
                      onSearchDirect={handleInitiateDirectDownloadSearch}
                    />
                  )}

                  {currentView === "notifications" && (
                    <NotificationsView
                      notifications={notifications}
                      onClearAll={clearAllNotifications}
                      onRemoveNotification={deleteNotification}
                      onMarkAllRead={markAllNotificationsAsRead}
                      onMarkAsRead={markNotificationAsRead}
                    />
                  )}

                  {currentView === "stats" && (
                    <StatsView
                      currentUser={currentUser}
                      onPlayTrack={handlePlayTrack}
                      onOpenAuthModal={() => setIsAuthModalOpen(true)}
                      fetchStatsOverview={fetchStatsOverview}
                      refreshTrigger={statsRefreshTrigger}
                      onUpdateUser={(u) => setCurrentUser(u)}
                    />
                  )}

                  {currentView === "settings" && (
                    <SettingsView
                      onRecommendationSetupSaved={handleTasteSaved}
                      settings={settings}
                      currentUser={currentUser}
                      onLogout={handleLogout}
                      onOpenAuthModal={() => setIsAuthModalOpen(true)}
                      interfaceStyle={interfacePreference.style}
                      onSelectInterfaceStyle={interfacePreference.select}
                      isSavingInterfaceStyle={interfacePreference.saving}
                      interfaceStyleError={interfacePreference.error}
                      configuredFolders={
                        onboardingStatus?.configured_folders || []
                      }
                      onAddFolder={handleAddFolder}
                      onRemoveFolder={handleRemoveFolder}
                      onRescanLibrary={handleRescanLibrary}
                      onReloadSettings={fetchSettings}
                      isScanning={isScanning}
                    />
                  )}
                </>
              )}
            </Suspense>
            </ViewMemoryProvider>
          </main>
        </div>

        {/* Bottom Sticky Player Bar (Unified for local music & online streams) */}
        <MainPlayer
          flowActive={!!homeOverview?.flow_session?.is_active}
          onToggleFlow={handleDeckFlow}
          entries={queueEntries}
          tracks={tracks}
          onPlayIndex={handlePlayQueueIndex}
          currentUser={currentUser}
          view={currentView}
          onNavigate={handleSelectView}
          playbackState={playbackState}
          isLoading={loadingTrackId !== null || !!playbackState.is_buffering}
          currentTrack={playbackState.current_track}
          coverArtUrl={activePlayingArtwork}
          onOpenQueue={handleOpenQueue}
          onOpenNowPlaying={handleOpenNowPlaying}
          onPlayPause={handleUnifiedPlayPause}
          onNext={handleNextTrack}
          onPrevious={handlePreviousTrack}
          onSeek={handleUnifiedSeek}
          onVolumeChange={handleVolumeChange}
          onToggleMute={handleToggleMute}
          onToggleRepeat={handleToggleRepeat}
          onToggleShuffle={handleToggleShuffle}
          onLike={handleLike}
          onLikeOnline={handleLikeOnline}
          isOnlineLiked={isCurrentTrackLiked}
          onRemoveFeedback={handleRemoveFeedback}
          isInPlaylist={isCurrentTrackInPlaylist}
          onOpenAddToPlaylist={handleOpenAddToPlaylistModal}
          onOpenOrigin={
            playingOrigin?.type === "playlist" ||
            playingOrigin?.type === "mix" ||
            playingOrigin?.type === "chart" ||
            playingOrigin?.type === "flow"
              ? handleOpenPlayingOrigin
              : undefined
          }
          originName={
            playingOrigin?.type === "playlist" ||
            playingOrigin?.type === "mix" ||
            playingOrigin?.type === "chart" ||
            playingOrigin?.type === "flow"
              ? playingOrigin.name
              : undefined
          }
          onInspectWhyTrack={
            playingOrigin?.type === "flow" && playbackState.current_track
              ? handleInspectWhyCurrentTrack
              : undefined
          }
          isLyricsActive={isLyricsActive}
          onToggleLyrics={() => setIsLyricsActive((prev) => !prev)}
          isFullscreen={isFullscreen}
          onToggleFullscreen={() => handleToggleFullscreen()}
        />

        {/* Full Screen Player View (matching user uploaded images) */}
        <Suspense fallback={null}>
          {isFullscreen && (
            <FullScreenPlayerView
              brutalist={interfacePreference.style === "brutalist" ? { entries: queueEntries, tracks, onPlayIndex: handlePlayQueueIndex, currentUser, view: currentView, onNavigate: handleSelectView, flowActive: !!homeOverview?.flow_session?.is_active, onToggleFlow: handleDeckFlow, albumId: albums.find(a => a.title === playbackState.current_track?.album_title && a.artist_name === playbackState.current_track?.artist_name)?.id, onOpenAlbum: handleSelectAlbum } : undefined}
              onOpenNowPlaying={() => { handleToggleFullscreen(false); handleOpenNowPlaying(); }}
              playbackState={playbackState}
              previousTrack={previousTrackRef.current}
              nextTrack={nextQueuedTrack}
              onOpenQueue={handleOpenQueue}
              onInspectWhyTrack={
                playingOrigin?.type === "flow"
                  ? handleInspectWhyCurrentTrack
                  : undefined
              }
              isLoading={
                loadingTrackId !== null || !!playbackState.is_buffering
              }
              currentTrack={playbackState.current_track}
              coverArtUrl={activePlayingArtwork}
              isInPlaylist={isCurrentTrackInPlaylist}
              onOpenAddToPlaylist={handleOpenAddToPlaylistModal}
              isLyricsActive={isLyricsActive}
              onToggleLyrics={() => setIsLyricsActive((prev) => !prev)}
              onToggleFullscreen={() => handleToggleFullscreen(false)}
              onPlayPause={handleUnifiedPlayPause}
              onNext={handleNextTrack}
              onPrevious={handlePreviousTrack}
              onSeek={handleUnifiedSeek}
              onVolumeChange={handleVolumeChange}
              onToggleMute={handleToggleMute}
              onToggleRepeat={handleToggleRepeat}
              onToggleShuffle={handleToggleShuffle}
              onLike={handleLike}
              onLikeOnline={handleLikeOnline}
              isOnlineLiked={isCurrentTrackLiked}
              onRemoveFeedback={handleRemoveFeedback}
                  onOpenOrigin={
                playingOrigin?.type === "playlist" ||
                playingOrigin?.type === "mix" ||
                playingOrigin?.type === "chart" ||
                playingOrigin?.type === "flow"
                  ? () => {
                      handleToggleFullscreen(false);
                      handleOpenPlayingOrigin();
                    }
                  : undefined
              }
              originName={
                playingOrigin?.type === "playlist" ||
                playingOrigin?.type === "mix" ||
                playingOrigin?.type === "chart" ||
                playingOrigin?.type === "flow"
                  ? playingOrigin.name
                  : undefined
              }
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
        {promptDialog && promptDialog.isOpen && (
          <PromptModal
            title={promptDialog.title}
            message={promptDialog.message}
            defaultValue={promptDialog.defaultValue}
            onConfirm={(val) => {
              promptDialog.resolve(val);
              setPromptDialog(null);
            }}
            onCancel={() => {
              promptDialog.resolve(null);
              setPromptDialog(null);
            }}
          />
        )}

        {/* Yes/No questions (e.g. removing a download); store-driven, no App state */}
        <ConfirmModal />

        {/* Flow Why This Track Modal */}
        {whyTrackModal && (
          <div
            className="modal-overlay"
            onClick={() => setWhyTrackModal(null)}
            style={{
              position: "fixed",
              inset: 0,
              backgroundColor: "rgba(0, 0, 0, 0.72)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              zIndex: 1000,
            }}
          >
            <div
              className="modal-content"
              onClick={(e) => e.stopPropagation()}
              style={{
                maxWidth: "420px",
                width: "90%",
                padding: "24px",
                borderRadius: "14px",
                backgroundColor: "var(--bg-card, #1a1714)",
                border: "1px solid var(--border, rgba(232, 216, 201, 0.15))",
                boxShadow: "0 20px 48px rgba(0, 0, 0, 0.65)",
              }}
            >
              <div
                style={{
                  display: "flex",
                  justifyContent: "space-between",
                  alignItems: "center",
                  marginBottom: "16px",
                }}
              >
                <div
                  style={{ display: "flex", alignItems: "center", gap: "8px" }}
                >
                  <Sparkles
                    size={18}
                    color="var(--accent-secondary, #f3701e)"
                  />
                  <h3
                    style={{
                      fontSize: "1.1rem",
                      fontWeight: 700,
                      margin: 0,
                      color: "var(--text-main, #e8d8c9)",
                    }}
                  >
                    Why This Track?
                  </h3>
                </div>
                <button
                  onClick={() => setWhyTrackModal(null)}
                  style={{
                    background: "none",
                    border: "none",
                    color: "var(--text-muted)",
                    cursor: "pointer",
                    padding: "4px",
                  }}
                >
                  <X size={18} />
                </button>
              </div>

              <div style={{ marginBottom: "12px" }}>
                <div
                  style={{
                    fontSize: "0.95rem",
                    fontWeight: 600,
                    color: "var(--text-main, #e8d8c9)",
                  }}
                >
                  {whyTrackModal.title}
                </div>
                <div
                  style={{
                    fontSize: "0.82rem",
                    color: "var(--text-dim, #948779)",
                  }}
                >
                  {whyTrackModal.artist}
                </div>
              </div>

              <div
                style={{
                  padding: "12px 14px",
                  borderRadius: "8px",
                  backgroundColor: "rgba(243, 112, 30, 0.08)",
                  border: "1px solid rgba(243, 112, 30, 0.2)",
                  fontSize: "0.88rem",
                  color: "var(--accent-light, #ff9b60)",
                  lineHeight: 1.45,
                }}
              >
                <TrackExplanation evidence={whyTrackModal.explanation} />
              </div>

              <div
                style={{
                  marginTop: "18px",
                  display: "flex",
                  justifyContent: "flex-end",
                }}
              >
                <button
                  className="btn btn-secondary"
                  onClick={() => setWhyTrackModal(null)}
                  style={{ fontSize: "0.85rem", padding: "6px 16px" }}
                >
                  Got it
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    </InterfaceContext.Provider>
  );
};
