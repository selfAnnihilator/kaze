import { recommendationPreferences } from "./services/recommendationPreferences";
import "./App.css";
import { removeRecommendationTracks } from "./recommendationFeedback";
import { useRouteScroll } from "./presentation/useRouteScroll";
import { ViewMemoryProvider } from "./presentation/ViewMemory";
import { useVisualActivity } from "./presentation/useVisualActivity";
import { usePointerFocus } from "./presentation/usePointerFocus";
import React, { useState, useEffect, useCallback, useRef, useMemo, Suspense, lazy,
} from "react";
import {
  Track,
  Album,
  Playlist,
  WishlistItem,
  DownloadTask,
  DownloadSearchResult,
  DiscoveryRecommendation,
  PlaybackState,
  AppSettings,
  OnboardingStatus,
  JioSaavnPlaylistImport,
  AppNotification,
  HomeOverview, FlowExplanation,
} from "./types";
import { dispatchCommand, executeQuery, subscribeBackendEvents,
} from "./services/api";
import { playbackProgress } from "./services/playbackProgress";
import { downloadProgress } from "./services/downloadProgress";
import { loadCoverArt, clearCoverArtCache } from "./services/coverArt";
import { viewDestination } from "./navigationHistory";
import { useNavigationHistory } from "./hooks/useNavigationHistory";
import { Sidebar } from "./components/Sidebar";
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
import { filterAndRankDownloadResults } from "./downloadMatch";
import { attachLocalSearchMatches } from "./localTrackMatch";
import { AddToPlaylistModalTrack } from "./components/modals/AddToPlaylistModal";
import {
  CollectionDetailView,
  CollectionData,
  CollectionTrackItem,
} from "./components/views/CollectionDetailView";
import { StatsOverview, UserProfile } from "./types";
import { X, Sparkles } from "lucide-react";
import { InterfaceContext } from "./presentation/InterfaceContext";
import { useInterfacePreference } from "./presentation/useInterfacePreference";
import { PlaybackStage } from "./presentation/PlaybackStage";
import { QueueView } from "./presentation/QueueView";
import {
  upcomingFromEvent,
  upcomingFromPlayback,
  type QueueEntry,
  type PlaybackArtworkTrack,
} from "./presentation/queuePresentation";
import { ArrowLeft, ArrowRight } from "lucide-react";
import "./presentation/interfaces.css";
import "./presentation/spatial.css";
import "./presentation/familiarity-dial.css";
import "./presentation/spatial-preview.css";
import "./presentation/brutalist.css";
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

interface PromptModalProps {
  title: string;
  message: string;
  defaultValue?: string;
  onConfirm: (value: string) => void;
  onCancel: () => void;
}

const PromptModal: React.FC<PromptModalProps> = ({
  title,
  message,
  defaultValue = "",
  onConfirm,
  onCancel,
}) => {
  const [val, setVal] = useState(defaultValue);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!val.trim()) return;
    onConfirm(val.trim());
  };

  return (
    <div className="modal-overlay" onClick={onCancel}>
      <div
        className="modal-content"
        style={{
          maxWidth: "460px",
          width: "92%",
          padding: "24px",
          borderRadius: "14px",
          backgroundColor: "var(--bg-card, #1a1714)",
          border: "1px solid var(--border, rgba(232, 216, 201, 0.15))",
          boxShadow: "0 24px 50px rgba(0, 0, 0, 0.65)",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "16px",
          }}>
          <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
            <Sparkles size={20} color="var(--accent-secondary, #f3701e)" />
            <h3 style={{ fontSize: "1.15rem", fontWeight: 700, margin: 0, color: "#e8d8c9",
              }}>
              {title}
            </h3>
          </div>
          <button
            onClick={onCancel}
            style={{ background: "none", border: "none", color: "var(--text-muted, #a89f91)", cursor: "pointer", padding: "4px",
            }}
          >
            <X size={18} />
          </button>
        </div>

        <p style={{ fontSize: "0.88rem", color: "var(--text-dim, #8c8273)", margin: "0 0 16px 0", lineHeight: 1.5,
          }}>
          {message}
        </p>

        <form onSubmit={handleSubmit}>
          <input
            type="text"
            autoFocus
            className="input-field"
            value={val}
            onChange={(e) => setVal(e.target.value)}
            style={{ width: "100%", fontSize: "0.95rem", marginBottom: "20px" }}
          />
          <div style={{ display: "flex", gap: "10px", justifyContent: "flex-end" }}>
            <button type="button" className="btn btn-secondary" onClick={onCancel}>
              Cancel
            </button>
            <button type="submit" className="btn btn-primary" disabled={!val.trim()}>
              Save Name
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};

export const App: React.FC<{ initialSettings: AppSettings }> = ({
  initialSettings,
}) => {
  // Navigation
  const { destination, navigate, updateDestination, back: navigateBack,
    forward: navigateForward,
    canGoBack,
    canGoForward,
  } = useNavigationHistory();
  const mainScrollRef = useRef<HTMLElement>(null);
  useRouteScroll(mainScrollRef, destination);
  useVisualActivity(mainScrollRef);
  usePointerFocus();
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
  const [isLoadingCollectionTracks, setIsLoadingCollectionTracks] = useState<boolean>(false);

  // Home & Flow State
  const [recommendationNotice, setRecommendationNotice] = useState<{ id: string; title: string } | null>(null);
  const [homeOverview, setHomeOverview] = useState<HomeOverview | null>(null);
  const [isHomeLoading, setIsHomeLoading] = useState<boolean>(false);
  const [flowPendingAction, setFlowPendingAction] = useState<"start" | "resume" | "pause" | null>(null);
  const flowCommandPendingRef = useRef(false);
  const [isTrendingLoading, setIsTrendingLoading] = useState(true);
  const [trendingError, setTrendingError] = useState<string | null>(null);
  const [flowSteering, setFlowSteering] = useState<number>(0.5);
  const [whyTrackModal, setWhyTrackModal] = useState<{
    title: string;
    artist: string;
    explanation: FlowExplanation;
  } | null>(null);

  // Playlist Management Modal
  const [isPlaylistModalOpen, setIsPlaylistModalOpen] = useState(false);
  const [playlistModalTrack, setPlaylistModalTrack] = useState<AddToPlaylistModalTrack | null>(null);
  const [trackPlaylistMap, setTrackPlaylistMap] = useState<Record<string, string[]>>({});

  // Onboarding
  const [onboardingStatus, setOnboardingStatus] = useState<OnboardingStatus | null>(null);
  const [showOnboarding, setShowOnboarding] = useState(false);

  // User Authentication & Profile
  const [currentUser, setCurrentUser] = useState<UserProfile | null>(null);
  const currentUserIdRef = useRef("default");
  const [isAuthModalOpen, setIsAuthModalOpen] = useState(false);
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
  const [tracks, setTracks] = useState<Track[]>([]);
  const [queuedTrackIds, setQueuedTrackIds] = useState<Set<string>>(new Set());
  const tracksRef = useRef<Track[]>([]);
  tracksRef.current = tracks;
  const [albums, setAlbums] = useState<Album[]>([]);
  const [playlists, setPlaylists] = useState<Playlist[]>([]);
  const [playingPlaylistId, setPlayingPlaylistId] = useState<string | null>(null,
  );
  const [wishlist, setWishlist] = useState<WishlistItem[]>([]);
  const [downloads, setDownloads] = useState<DownloadTask[]>([]);
  const [discoveryRecs, setDiscoveryRecs] = useState<DiscoveryRecommendation[]>([],
  );
  const [worldTrending, setWorldTrending] = useState<DiscoveryRecommendation[]>([],
  );
  const discoveryRecsRef = useRef<DiscoveryRecommendation[]>([]);
  discoveryRecsRef.current = discoveryRecs;
  const [settings, setSettings] = useState<AppSettings>(initialSettings);
  const interfacePreference = useInterfacePreference(settings);
  const [queueEntries, setQueueEntries] = useState<QueueEntry[]>([]);
  const previousTrackRef = useRef<Track | undefined>(undefined);
  const observedTrackRef = useRef<Track | undefined>(undefined);
  if (!playbackState.is_buffering && (playbackState.current_track?.duration_secs || 0) > 0 && playbackState.current_track?.id !== observedTrackRef.current?.id) {
    previousTrackRef.current = observedTrackRef.current;
    observedTrackRef.current = playbackState.current_track;
  }
  const [isScanning, setIsScanning] = useState(false);

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

  // Online / Offline Connectivity State
  const [isOnline, setIsOnline] = useState<boolean>(
    typeof navigator !== "undefined" ? navigator.onLine : true,
  );

  useEffect(() => {
    const handleOnline = () => setIsOnline(true);
    const handleOffline = () => setIsOnline(false);
    window.addEventListener("online", handleOnline);
    window.addEventListener("offline", handleOffline);
    return () => {
      window.removeEventListener("online", handleOnline);
      window.removeEventListener("offline", handleOffline);
    };
  }, []);

  // Full Screen & Lyrics View State
  const [isFullscreen, setIsFullscreen] = useState(false);


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
  const [resolvedTrackCover, setResolvedTrackCover] = useState<{ id: string; url: string | null } | null>(null);
  const coverTrack = playbackState.current_track;
  const currentTrackCoverUrl = coverTrack?.cover_art_url || (resolvedTrackCover?.id === coverTrack?.id ? resolvedTrackCover?.url : null);
  useEffect(() => {
    const track = playbackState.current_track;
    if (!track || track.cover_art_url || track.id.startsWith("online:") || track.id.startsWith("itunes:")) return;
    const controller = new AbortController();
    loadCoverArt(track.id, controller.signal).then(url => {
      if (!controller.signal.aborted) setResolvedTrackCover({ id: track.id, url });
    });
    return () => controller.abort();
  }, [playbackState.current_track?.id, playbackState.current_track?.cover_art_url, playbackState.current_track?.has_cover_art]);

  const handleToggleFullscreen = useCallback(
    async (override?: boolean) => {
      const nextState = typeof override === "boolean" ? override : !isFullscreen;
      setIsFullscreen(nextState);

      // 1. Tauri OS Window Fullscreen (takes entire monitor, hides titlebar/OS panel like YouTube video full screen)
      try {
        const { getCurrentWindow } = await import("@tauri-apps/api/window");
        const appWindow = getCurrentWindow();
        await appWindow.setFullscreen(nextState);
      } catch (err) {
        console.warn("Tauri window setFullscreen failed:", err);
      }

      // 2. HTML5 document fullscreen standard
      try {
        if (nextState) {
          if (!document.fullscreenElement && document.documentElement.requestFullscreen) {
            await document.documentElement.requestFullscreen();
          }
        } else {
          if (document.fullscreenElement && document.exitFullscreen) {
            await document.exitFullscreen();
          }
        }
      } catch (err) {
        // Ignore fallback errors
      }
    },
    [isFullscreen],
  );

  const handlePlayPauseRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.key === "Escape" && isFullscreen) || e.key === "F11") {
        e.preventDefault();
        handleToggleFullscreen(e.key === "Escape" ? false : undefined);
        return;
      }

      if (
        e.key.toLowerCase() === "f" &&
        !e.defaultPrevented && !e.repeat && !e.isComposing &&
        !e.ctrlKey && !e.altKey && !e.metaKey
      ) {
        const editing = e.composedPath().some(target =>
          target instanceof Element && Boolean(target.closest(
            "input, textarea, select, [contenteditable]:not([contenteditable='false']), [role='textbox'], [role='searchbox'], [role='combobox'], [role='menu'], [role='listbox']",
          )),
        );
        const dialog = document.querySelector(
          ".modal-overlay, [role='dialog']:not(.fullscreen-player), [aria-modal='true']:not(.fullscreen-player)",
        );
        if (!editing && !dialog) {
          e.preventDefault();
          handleToggleFullscreen();
        }
        return;
      }

      if (
        e.code === "Space" &&
        !e.repeat &&
        !e.ctrlKey &&
        !e.altKey &&
        !e.metaKey &&
        !e.shiftKey
      ) {
        const target = e.target as HTMLElement | null;
        if (
          target &&
          (target.tagName === "INPUT" ||
            target.tagName === "TEXTAREA" ||
            target.tagName === "SELECT" ||
            target.tagName === "BUTTON" ||
            Boolean(target.closest?.("button, [role='button']")) ||
            target.isContentEditable ||
            Boolean(target.closest?.("[contenteditable='true'], [contenteditable='']",
              ),
            ))
        ) {
          return;
        }

        e.preventDefault();
        handlePlayPauseRef.current?.();
      }
    };

    const handleFullscreenChange = () => {
      if (!document.fullscreenElement && isFullscreen) {
        handleToggleFullscreen(false);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    document.addEventListener("fullscreenchange", handleFullscreenChange);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      document.removeEventListener("fullscreenchange", handleFullscreenChange);
    };
  }, [isFullscreen, handleToggleFullscreen]);

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

  const fetchOnboardingStatus = useCallback(async () => {
    try {
      const res = await executeQuery({ query: "GetOnboardingStatus" });
      const status: OnboardingStatus = {
        completed: res.data?.completed ?? res.completed ?? false,
        default_music_dir: res.data?.default_music_dir ?? res.default_music_dir ?? "",
        configured_folders: res.data?.configured_folders ?? res.configured_folders ?? [],
      };
      setOnboardingStatus(status);
      if (!status.completed) {
        setShowOnboarding(true);
      }
    } catch (err) {
      console.error("Failed to check onboarding status:", err);
    }
  }, []);

  const fetchPlaybackState = useCallback(async () => {
    const revision = playbackStateRevision.current;
    const queryId = ++playbackStateQueryId.current;
    try {
      const res = await executeQuery({ query: "GetPlaybackState" });
      // Source/catalog work can delay this response beyond a newer transport
      // event. Never reinstate the old song or its buffering/paused state.
      if (revision !== playbackStateRevision.current || queryId !== playbackStateQueryId.current) return;
      if (res.data) {
        playbackProgress.update(
          res.data.position_secs ?? 0,
          res.data.duration_secs ?? 0,
        );
        setPlaybackState((prev) => {
          const updated = { ...prev, ...res.data,
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
          setQueueEntries(upcomingFromPlayback(res.data));
        }
      }
    } catch (err) {
      console.error("Failed to fetch playback state:", err);
    }
  }, []);

  const fetchTracks = useCallback(async () => {
    try {
      const res = await executeQuery({
        query: "GetTracks",
        payload: { offset: 0, limit: 1000, ascending: true },
      });
      if (Array.isArray(res.data)) {
        setTracks(res.data);
      }
    } catch (err) {
      console.error("Failed to fetch tracks:", err);
    }
  }, []);

  const fetchAlbums = useCallback(async () => {
    try {
      const res = await executeQuery({
        query: "GetAlbums",
        payload: { offset: 0, limit: 200 },
      });
      if (Array.isArray(res.data)) {
        setAlbums(res.data);
      }
    } catch (err) {
      console.error("Failed to fetch albums:", err);
    }
  }, []);

  const fetchPlaylists = useCallback(async () => {
    void recommendationPreferences.refresh().catch(console.error);
    try {
      const res = await executeQuery({ query: "GetPlaylists" });
      if (Array.isArray(res.data)) {
        setPlaylists(res.data);
      }
    } catch (err) {
      console.error("Failed to fetch playlists:", err);
    }
  }, []);

  const fetchTrackPlaylistMemberships = useCallback(async () => {
    try {
      const res = await executeQuery({ query: "GetTrackPlaylistMemberships" });
      if (res.data && typeof res.data === "object") {
        setTrackPlaylistMap(res.data);
      }
    } catch (err) {
      console.error("Failed to fetch track playlist memberships:", err);
    }
  }, []);

  const fetchWishlist = useCallback(async () => {
    try {
      const res = await executeQuery({ query: "GetWishlist" });
      if (Array.isArray(res.data)) {
        setWishlist(res.data);
      }
    } catch (err) {
      console.error("Failed to fetch wishlist:", err);
    }
  }, []);

  const fetchDownloads = useCallback(async () => {
    try {
      const res = await executeQuery({
        query: "GetDownloads",
        payload: { limit: 10000 },
      });
      if (Array.isArray(res.data)) {
        downloadProgress.replace(res.data);
        setDownloads(res.data);
      }
    } catch (err) {
      console.error("Failed to fetch downloads:", err);
    }
  }, []);

  const homeRequestRef = useRef(0);
  const steeringRevisionRef = useRef(0);
  const steeringPendingRef = useRef(false);
  const flowSessionRequestRef = useRef(0);
  const fetchHomeOverview = useCallback(async () => {
    const request = ++homeRequestRef.current;
    const steeringRevision = steeringRevisionRef.current;
    const flowRevision = flowSessionRequestRef.current;
    try {
      setIsHomeLoading(true);
      const res = await executeQuery({ query: "GetHomeOverview" });
      if (res.type === "HomeOverview" && res.data && request === homeRequestRef.current) {
        setHomeOverview(
          (prev) => ({ ...res.data, flow_session: flowRevision === flowSessionRequestRef.current ? res.data.flow_session : prev?.flow_session,
            }) as HomeOverview,
        );
        if (res.data.flow_session && !steeringPendingRef.current && steeringRevision === steeringRevisionRef.current) {
          setFlowSteering(res.data.flow_session.familiarity_ratio);
        }
      }
    } catch (err) {
      console.error("Failed to fetch home overview:", err);
    } finally {
      if (request === homeRequestRef.current) setIsHomeLoading(false);
    }
  }, []);

  const fetchFlowSession = useCallback(async () => {
    const request = ++flowSessionRequestRef.current;
    const res = await executeQuery({ query: "GetFlowSession" });
    if (res.type === "FlowSession" && request === flowSessionRequestRef.current) {
      setHomeOverview((prev) => ({ ...(prev || { recent_rotation: [], rediscover: [], for_you: [] }), flow_session: res.data,
      }));
      setPlayingOrigin((prev) => res.data?.is_active ? { type: "flow", name: "Flow" } : prev?.type === "flow" ? null : prev,
      );
    }
  }, []);

  useEffect(() => {
    if (currentView !== "home") return;
    const timer = setTimeout(() => { void fetchHomeOverview(); }, 250);
    return () => clearTimeout(timer);
  }, [currentView, playbackState.current_track?.id, currentUser?.id, fetchHomeOverview,
  ]);

  const discoveryRequestRef = useRef(0);
  const chartRequestRef = useRef(0);
  const fetchPersonalRecommendations = useCallback(async (forceRefresh = false) => {
    const request = ++discoveryRequestRef.current;
    const userId = currentUserIdRef.current;
    const res = await executeQuery({
      query: "GetDiscoveryRecommendations",
      payload: { limit: 100, force_refresh: forceRefresh },
    });
    if (request === discoveryRequestRef.current && userId === currentUserIdRef.current && Array.isArray(res.data)) {
      setDiscoveryRecs(res.data);
    }
  }, []);

  const fetchTrending = useCallback(async (forceRefresh = false) => {
    const request = ++chartRequestRef.current;
    const user = currentUserIdRef.current;
    const current = () => request === chartRequestRef.current && user === currentUserIdRef.current;
    setIsTrendingLoading(true);
    setTrendingError(null);
    try {
      if (!forceRefresh) {
        try {
          const cached = await executeQuery({ query: "GetCachedWorldTrending", payload: { limit: 50 } });
          if (current() && Array.isArray(cached.data) && cached.data.length) setWorldTrending(cached.data);
        } catch (error) { console.error("Failed to read saved trending songs:", error); }
      }
      const res = await executeQuery({ query: "GetWorldTrending", payload: { limit: 50, force_refresh: forceRefresh } });
      if (current() && Array.isArray(res.data)) setWorldTrending(res.data);
    } catch (error) {
      if (current()) setTrendingError("Couldn't update trending songs. Try Refresh again.");
      console.error("Failed to fetch world charts:", error);
    } finally {
      if (current()) setIsTrendingLoading(false);
    }
  }, []);
  const refreshTrending = useCallback(() => fetchTrending(true), [fetchTrending]);
  const refreshPersonalRecommendations = useCallback(() => fetchPersonalRecommendations(true), [fetchPersonalRecommendations]);
  const fetchDiscovery = useCallback(async (forceRefresh = false, refreshCharts = false) => {
    const results = await Promise.allSettled([
      fetchPersonalRecommendations(forceRefresh), fetchTrending(refreshCharts),
    ]);
    for (const result of results) if (result.status === "rejected") console.error("Failed to refresh discovery:", result.reason);
  }, [fetchPersonalRecommendations, fetchTrending]);

  useEffect(() => {
    const userId = currentUser?.id || "default";
    if (userId === currentUserIdRef.current) return;
    currentUserIdRef.current = userId;
    recommendationPreferences.reset();
    void recommendationPreferences.refresh().catch(console.error);
    ++discoveryRequestRef.current;
    ++chartRequestRef.current;
    setDiscoveryRecs([]);
    setWorldTrending([]);
    void fetchDiscovery();
  }, [currentUser?.id, fetchDiscovery]);

  const fetchSettings = useCallback(async () => {
    try {
      const res = await executeQuery({ query: "GetSettings" });
      if (res.data) {
        setSettings(res.data as AppSettings);
      }
    } catch (err) {
      console.error("Failed to fetch settings:", err);
    }
  }, []);

  const fetchCurrentUser = useCallback(async () => {
    try {
      const res = await executeQuery({ query: "GetCurrentUser" });
      if (res && res.data) {
        setCurrentUser(res.data);
      } else {
        setCurrentUser(null);
      }
    } catch (err) {
      console.error("Failed to fetch current user:", err);
    }
  }, []);

  const handleLogin = async (
    username: string,
    password: string,
  ): Promise<UserProfile | null> => {
    const res = await dispatchCommand({
      command: "Login",
      payload: { username, password },
    });
    if (res && res.status === "UserProfile" && res.data) {
      setCurrentUser(res.data);
      addAppNotification(
        "success",
        "Logged In",
        `Welcome back, ${res.data.username}!`,
      );
      await fetchTracks();
      await fetchPlaylists();
      await fetchDiscovery(true);
      return res.data;
    }
    return null;
  };

  const handleSignUp = async (
    username: string,
    password: string,
  ): Promise<UserProfile | null> => {
    const res = await dispatchCommand({
      command: "SignUp",
      payload: { username, password },
    });
    if (res && res.status === "UserProfile" && res.data) {
      setCurrentUser(res.data);
      addAppNotification(
        "success",
        "Account Created",
        `Welcome to Kaze, ${res.data.username}!`,
      );
      await fetchTracks();
      await fetchPlaylists();
      await fetchDiscovery(true);
      return res.data;
    }
    return null;
  };

  const handleLogout = async (scope: "device" | "all") => {
    await dispatchCommand({ command: scope === "all" ? "LogoutAll" : "Logout" });
    window.location.reload();
  };

  const fetchStatsOverview = useCallback(
    async (year?: number, month?: number): Promise<StatsOverview | null> => {
      try {
        const payload: { year?: number; month?: number } = {};
        if (year !== undefined) payload.year = year;
        if (month !== undefined) payload.month = month;
        const res = await executeQuery({
          query: "GetStatsOverview",
          payload,
        });
        if (res && res.data) {
          return res.data as StatsOverview;
        }
      } catch (err) {
        console.error("Failed to fetch stats overview:", err);
      }
      return null;
    },
    [],
  );

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

  // Backend Event Subscriptions
  useEffect(() => {
    let unlistenFn: (() => void) | null = null;
    let subscriptionDisposed = false;
    let tasteRefreshTimer: ReturnType<typeof setTimeout> | undefined;

    subscribeBackendEvents((event: any) => {
      if (!event || !event.event) return;

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
                  format: "online",
                  has_cover_art: 0,
                });
          setPlaybackState((prev) => ({
            ...prev,
            current_track: track,
            is_playing: true,
            is_paused: false,
            is_buffering: false,
            position_secs: 0,
            duration_secs:
              event.payload?.duration_secs ||
              track.duration_secs ||
              prev.duration_secs,
          }));
          break;
        }

        case "PlaybackBuffering": {
          ++playbackStateRevision.current;
          // Only PlaybackService emits this initial-resolution event. Reader
          // stalls use the session-scoped event below and cannot select a song.
          playbackSessionId.current = null;
          const trackId = event.payload?.track_id;
          const found = tracksRef.current.find((t) => t.id === trackId);
          setPlaybackState((prev) => ({
            ...prev,
            current_track:
              found ||
              (trackId
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
                    format: "online",
                    has_cover_art: 0,
                  }
                : prev.current_track),
            is_buffering: true,
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
        case "ScanStarted":
          setIsScanning(true);
          break;

        case "LibraryMetadataUpdated":
          clearCoverArtCache();
          fetchTracks();
          fetchAlbums();
          fetchPlaybackState();
          break;
        case "LibraryScanCompleted":
        case "ScanCompleted":
          clearCoverArtCache();
          setIsScanning(false);
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
            setRecommendationNotice(null);
            void fetchDiscovery(false, false);
          }
          setPlaybackState(previous => previous.current_track && ids.has(previous.current_track.id)
            ? { ...previous, current_track: { ...previous.current_track, manual_like: event.payload.manual_like } } : previous);
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
    }).then((fn) => {
      if (subscriptionDisposed) fn();
      else unlistenFn = fn;
    });

    return () => {
      subscriptionDisposed = true;
      clearTimeout(tasteRefreshTimer);
      ++discoveryRequestRef.current;
      if (unlistenFn) unlistenFn();
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
    fetchTrackPlaylistMemberships,
    fetchFlowSession,
    fetchHomeOverview,
    fetchPlaybackState,
  ]);

  // --- Actions & Commands ---

  const handleStopOnlineAudio = useCallback(() => {}, []);

  const handleStopTrack = useCallback(async () => {
    if (playbackState.is_playing) {
      await dispatchCommand({ command: "Pause" });
      setPlaybackState((prev) => ({ ...prev, is_playing: false }));
    }
  }, [playbackState.is_playing]);

  const handlePlayTrack = useCallback(
    async (trackId: string) => {
      if (playbackState.current_track?.id === trackId && (playbackState.is_playing || playbackState.is_paused)) {
        await dispatchCommand({
          command: playbackState.is_playing ? "Pause" : "Resume",
        });
        await fetchPlaybackState();
        return;
      }
      const requestId = beginPlaybackLoading(trackId);
      try {
        const found = tracks.find((t) => t.id === trackId);
        if (found) {
          setPlaybackState((prev) => ({
            ...prev,
            current_track: found,
            is_playing: true,
            position_secs: 0,
            duration_secs: found.duration_secs || prev.duration_secs,
          }));
        }
        await dispatchCommand({
          command: "PlayTrack",
          payload: { track_id: trackId },
        });
        await fetchPlaybackState();
      } catch (err) {
        await fetchPlaybackState();
        throw err;
      } finally {
        endPlaybackLoading(requestId);
      }
    },
    [
      tracks,
      playbackState.current_track?.id,
      playbackState.is_playing,
      playbackState.is_paused,
      fetchPlaybackState,
      beginPlaybackLoading,
      endPlaybackLoading,
    ],
  );

  const handlePlayOnlineTrack = useCallback(
    async (
      rec: DiscoveryRecommendation,
      _fromBackendQueue = false,
      playbackSource: "online" | "collection" | "playlist" = "online",
    ) => {
      const exactProvider = !!rec.playback_source || rec.provider === "jiosaavn" || rec.external_track_id.startsWith("jiosaavn:");
      const hasRealLocalMatch =
        !exactProvider && !!rec.matched_local_track_id &&
        !rec.matched_local_track_id.startsWith("itunes:") &&
        !rec.matched_local_track_id.startsWith("online:");

      // 1. Check if this track is currently playing
      const isCurrentlyPlaying =
        (playbackState.is_playing || playbackState.is_paused) &&
        playbackState.current_track &&
        ((hasRealLocalMatch &&
          playbackState.current_track.id === rec.matched_local_track_id) ||
          rec.external_track_id === playbackState.current_track.id ||
          (!!rec.playback_source && !!playbackState.current_track.playback_source &&
            rec.playback_source.track_id === playbackState.current_track.playback_source.track_id));

      if (isCurrentlyPlaying) {
        await dispatchCommand({
          command: playbackState.is_playing ? "Pause" : "Resume",
        });
        setPlaybackState((prev) => ({ ...prev, is_playing: !playbackState.is_playing }));
        return;
      }

      // 2. If this track is in local library, play it locally
      if (hasRealLocalMatch) {
        await handlePlayTrack(rec.matched_local_track_id!);
        return;
      }

      // Dispatch PlayOnlineTrack to unified Rust PlaybackService
      const requestId = beginPlaybackLoading(rec.external_track_id);
      try {
        await dispatchCommand({
          command: "PlayOnlineTrack",
          payload: {
            track_id: rec.external_track_id,
            title: rec.title,
            artist: rec.artist,
            album: rec.album || undefined,
            duration_secs: rec.duration_secs || undefined,
            cover_art_url: rec.cover_art_url || undefined,
            preview_url: rec.preview_url || undefined,
            source: playbackSource,
          },
        });
        await fetchPlaybackState();
      } catch (err) {
        console.error("Failed to play online track via Rust engine:", err);
        addAppNotification(
          "error",
          "Playback Failed",
          `Could not stream "${rec.title}": ${String(err)}`,
        );
      } finally {
        endPlaybackLoading(requestId);
      }
    },
    [
      playbackState.is_playing,
      playbackState.is_paused,
      playbackState.current_track,
      handlePlayTrack,
      addAppNotification,
      fetchPlaybackState,
      beginPlaybackLoading,
      endPlaybackLoading,
    ],
  );

  const handlePlayPause = useCallback(async () => {
    if (playbackState.is_playing) {
      await dispatchCommand({ command: "Pause" });
      setPlaybackState((prev) => ({ ...prev, is_playing: false }));
    } else {
      await dispatchCommand({ command: "Resume" });
      setPlaybackState((prev) => ({ ...prev, is_playing: true }));
    }
  }, [playbackState.is_playing]);

  useEffect(() => {
    handlePlayPauseRef.current = handlePlayPause;
  });

  const handleUnifiedPlayPause = handlePlayPause;

  const handleNextTrack = useCallback(async () => {
    await dispatchCommand({ command: "NextTrack" });
  }, []);

  const handlePreviousTrack = useCallback(async () => {
    await dispatchCommand({ command: "PreviousTrack" });
  }, []);

  const handleSeek = async (position_secs: number) => {
    playbackProgress.update(position_secs, playbackProgress.getDuration());
    await dispatchCommand({
      command: "Seek",
      payload: { position_secs },
    });
  };

  const handleUnifiedSeek = handleSeek;

  const handleVolumeChange = async (volume: number) => {
    await dispatchCommand({
      command: "SetVolume",
      payload: { volume },
    });
    setPlaybackState((prev) => ({ ...prev, volume }));
  };

  const handleToggleMute = async () => {
    await dispatchCommand({ command: "ToggleMute" });
    setPlaybackState((prev) => ({ ...prev, is_muted: !prev.is_muted }));
  };

  const handleToggleRepeat = async () => {
    const nextMode = playbackState.repeat_mode === "one" ? "off" : "one";

    await dispatchCommand({
      command: "SetRepeatMode",
      payload: { mode: nextMode },
    });
    setPlaybackState((prev) => ({ ...prev, repeat_mode: nextMode }));
  };

  const handleToggleShuffle = async () => {
    const next = !playbackState.is_shuffled;
    await dispatchCommand({
      command: "SetShuffle",
      payload: { enabled: next },
    });
    setPlaybackState((prev) => ({ ...prev, is_shuffled: next }));
  };

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
            const isOnline =
              t.format === "online" ||
              t.file_path?.startsWith("online://") ||
              t.id.startsWith("itunes:") ||
              t.id.startsWith("online:");
            return {
              id: t.id,
              title: t.title,
              artist: t.artist_name || "Unknown Artist",
              album: t.album_title,
              duration_secs: t.duration_secs || 210,
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

  const handleEnqueueTrack = async (trackId: string) => {
    if (queuedTrackIds.has(trackId)) {
      addAppNotification(
        "info",
        "Already in Queue",
        "This song is already waiting in the queue.",
      );
      return;
    }
    try {
      await dispatchCommand({
        command: "EnqueueTrack",
        payload: { track_id: trackId, play_next: false },
      });
      setQueuedTrackIds((prev) => new Set(prev).add(trackId));
    } catch (err: any) {
      if (
        String(err?.message || err)
          .toLowerCase()
          .includes("already in the queue")
      ) {
        setQueuedTrackIds((prev) => new Set(prev).add(trackId));
        addAppNotification(
          "info",
          "Already in Queue",
          "This song is already waiting in the queue.",
        );
        return;
      }
      addAppNotification(
        "error",
        "Queue Failed",
        err?.message || "Could not add this song to the queue.",
      );
    }
  };

  const handleEnqueueRecommendation = async (rec: DiscoveryRecommendation) => {
    const hasLocalMatch =
      !!rec.matched_local_track_id &&
      !rec.matched_local_track_id.startsWith("online:") &&
      !rec.matched_local_track_id.startsWith("itunes:");
    const trackId = hasLocalMatch
      ? rec.matched_local_track_id!
      : rec.external_track_id;
    if (!hasLocalMatch) {
      if (queuedTrackIds.has(trackId)) {
        addAppNotification(
          "info",
          "Already in Queue",
          "This song is already waiting in the queue.",
        );
        return;
      }
      try {
        await dispatchCommand({
          command: "EnqueueOnlineTrack",
          payload: {
            track_id: trackId,
            title: rec.title,
            artist: rec.artist,
            album: rec.album,
            duration_secs: rec.duration_secs,
            cover_art_url: rec.cover_art_url,
            preview_url: rec.preview_url,
            play_next: false,
          },
        });
        setQueuedTrackIds((prev) => new Set(prev).add(trackId));
      } catch (err: any) {
        if (
          String(err?.message || err)
            .toLowerCase()
            .includes("already in the queue")
        ) {
          setQueuedTrackIds((prev) => new Set(prev).add(trackId));
          addAppNotification(
            "info",
            "Already in Queue",
            "This song is already waiting in the queue.",
          );
        } else {
          addAppNotification(
            "error",
            "Queue Failed",
            err?.message || "Could not add this song to the queue.",
          );
        }
      }
      return;
    }
    await handleEnqueueTrack(trackId);
  };

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

  useEffect(() => {
    if (
      destination.view !== "library" ||
      destination.collection ||
      destination.lyrics
    )
      return;
    let cancelled = false;
    const load = async () => {
      try {
        const res = await executeQuery(
          destination.libraryQuery
            ? {
                query: "SearchLibrary",
                payload: { query_text: destination.libraryQuery, limit: 100 },
              }
            : {
                query: "GetTracks",
                payload: { limit: 1000, offset: 0, ascending: true },
              },
        );
        if (!cancelled && Array.isArray(res.data)) setTracks(res.data);
      } catch (err) {
        console.error("Library navigation failed:", err);
      }
    };
    void load();
    return () => {
      cancelled = true;
    };
  }, [
    destination.view,
    destination.collection,
    destination.lyrics,
    destination.libraryQuery,
    destination.libraryAlbumId,
  ]);

  const handleSelectAlbum = useCallback(
    (albumId: string) => {
      const album = albums.find((al) => al.id === albumId);
      if (album)
        navigate({
          ...viewDestination("library"),
          libraryQuery: album.title,
          libraryAlbumId: album.id,
        });
    },
    [albums, navigate],
  );

  const handleStartFlow = useCallback(
    async (familiarityRatio: number) => {
      if (flowCommandPendingRef.current) return;
      flowCommandPendingRef.current = true;
      setFlowPendingAction("start");
      try {
        await dispatchCommand({
          command: "StartFlow",
          payload: { familiarity_ratio: familiarityRatio },
        });
        await fetchFlowSession();
        // Home artwork and recommendations must not hold the playback action busy.
        void Promise.allSettled([fetchPlaybackState(), fetchHomeOverview()]);
      } catch (err) {
        console.error("Failed to start Flow:", err);
        addAppNotification(
          "error",
          "Flow Error",
          "Failed to start Flow: " + String(err),
        );
      } finally {
        flowCommandPendingRef.current = false;
        setFlowPendingAction(null);
      }
    },
    [
      fetchPlaybackState,
      fetchHomeOverview,
      fetchFlowSession,
      addAppNotification,
    ],
  );

  const handleToggleFlowPlayback = useCallback(async () => {
    if (flowCommandPendingRef.current) return;
    flowCommandPendingRef.current = true;
    setFlowPendingAction(playbackState.is_playing ? "pause" : "resume");
    try {
      if (playbackState.is_playing) await handleUnifiedPlayPause();
      else {
        await dispatchCommand({ command: "ContinueFlow" });
        void fetchPlaybackState();
      }
    } catch (error) {
      addAppNotification("error", "Flow Error", String(error));
    } finally {
      flowCommandPendingRef.current = false;
      setFlowPendingAction(null);
    }
  }, [playbackState.is_playing, handleUnifiedPlayPause, fetchPlaybackState, addAppNotification]);

  const steeringDebounceRef = useRef<ReturnType<typeof setTimeout> | null>(
    null,
  );
  const handleFlowSteeringChange = useCallback((val: number) => {
    const revision = ++steeringRevisionRef.current;
    steeringPendingRef.current = true;
    setFlowSteering(val);
    if (steeringDebounceRef.current) {
      clearTimeout(steeringDebounceRef.current);
    }
    steeringDebounceRef.current = setTimeout(async () => {
      try {
        await dispatchCommand({
          command: "SetFlowSteering",
          payload: { familiarity_ratio: val },
        });
        if (revision === steeringRevisionRef.current) {
          steeringPendingRef.current = false;
          await fetchFlowSession();
          await fetchPlaybackState();
        }
      } catch (err) {
        console.error("Failed to update Flow steering:", err);
        if (revision === steeringRevisionRef.current)
          steeringPendingRef.current = false;
      }
    }, 250);
  }, [fetchFlowSession, fetchPlaybackState]);

  useEffect(
    () => () => {
      if (steeringDebounceRef.current)
        clearTimeout(steeringDebounceRef.current);
    },
    [],
  );

  const handleInspectWhyCurrentTrack = useCallback(async () => {
    if (!playbackState.current_track) return;
    try {
      const res = await executeQuery({
        query: "GetFlowTrackReason",
        payload: { track_id: playbackState.current_track.id },
      });
      if (res.data) {
        setWhyTrackModal({
          title: playbackState.current_track.title,
          artist: playbackState.current_track.artist_name || "Unknown Artist",
          explanation: res.data,
        });
      }
    } catch (err) {
      console.error("Failed to fetch Flow track reason:", err);
    }
  }, [playbackState.current_track]);

  // Onboarding completion
  const handleCompleteOnboarding = async (
    folders: string[],
    startScan: boolean,
  ) => {
    setShowOnboarding(false);
    await dispatchCommand({
      command: "CompleteOnboarding",
      payload: { music_folders: folders, start_scan: startScan },
    });
    fetchOnboardingStatus();
    if (startScan) {
      setIsScanning(true);
      fetchTracks();
    }
  };

  // Library & folder management
  const handleAddFolder = async (path: string) => {
    await dispatchCommand({
      command: "AddLibraryFolder",
      payload: { path },
    });
    fetchOnboardingStatus();
  };

  const handleRemoveFolder = async (folderId: string) => {
    await dispatchCommand({
      command: "RemoveLibraryFolder",
      payload: { folder_id: folderId },
    });
    fetchOnboardingStatus();
  };

  const handleRescanLibrary = async () => {
    setIsScanning(true);
    try {
      await dispatchCommand({
        command: "ScanLibrary",
        payload: { incremental: false },
      });
    } catch (error) {
      setIsScanning(false);
      throw error;
    }
  };

  // Playlists & Smart Mixes
  const handleRenamePlaylist = async (playlistId: string, name: string) => {
    await dispatchCommand({
      command: "RenamePlaylist",
      payload: { playlist_id: playlistId, name },
    });
    setActiveCollection((current) =>
      current &&
      (current.playlistId === playlistId || current.id === playlistId)
        ? { ...current, title: name.trim() }
        : current,
    );
    await fetchPlaylists();
  };

  const handleDeletePlaylist = async (playlistId: string) => {
    await dispatchCommand({
      command: "DeletePlaylist",
      payload: { playlist_id: playlistId },
    });
    await fetchPlaylists();
    if (
      activeCollection?.playlistId === playlistId ||
      activeCollection?.id === playlistId
    ) {
      setActiveCollection(null);
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

  const handleCreatePlaylist = async (
    name: string,
    description?: string,
  ): Promise<string | null> => {
    if (!currentUser) {
      setIsAuthModalOpen(true);
      return null;
    }
    let targetName = name.trim();
    while (
      playlists.some((p) => p.is_smart_mix !== 1 && p.name === targetName)
    ) {
      const prompted = window.prompt(
        `A playlist named "${targetName}" already exists.\nPlease enter a new name for this playlist:`,
        `${targetName} (1)`,
      );
      if (prompted === null) {
        return null;
      }
      const trimmed = prompted.trim();
      if (!trimmed) {
        alert("Playlist name cannot be empty.");
        continue;
      }
      targetName = trimmed;
    }

    const res = await dispatchCommand({
      command: "CreatePlaylist",
      payload: { name: targetName, description },
    });
    await fetchPlaylists();
    if (res.data && typeof res.data === "string") {
      return res.data;
    }
    return null;
  };

  const handleToggleTrackInPlaylist = async (
    playlistId: string,
    isCurrentlyMember: boolean,
  ) => {
    if (!playlistModalTrack) return;
    const trackId = playlistModalTrack.id;
    try {
      if (isCurrentlyMember) {
        await dispatchCommand({
          command: "RemoveTrackFromPlaylist",
          payload: { playlist_id: playlistId, track_id: trackId },
        });
        setTrackPlaylistMap((prev) => {
          const current = prev[trackId] || [];
          return {
            ...prev,
            [trackId]: current.filter((id) => id !== playlistId),
          };
        });
      } else {
        await dispatchCommand({
          command: "AddTrackToPlaylist",
          payload: {
            playlist_id: playlistId,
            track_id: trackId,
            title: playlistModalTrack.title,
            artist: playlistModalTrack.artist,
            album: playlistModalTrack.album,
            cover_art_url: playlistModalTrack.cover_art_url,
          },
        });
        setTrackPlaylistMap((prev) => {
          const current = prev[trackId] || [];
          return { ...prev, [trackId]: [...current, playlistId] };
        });
      }
      await fetchPlaylists();
      await fetchTrackPlaylistMemberships();
    } catch (err) {
      console.error("Failed to toggle track in playlist:", err);
    }
  };

  const handleOpenAddToPlaylistModal = (track: {
    id: string;
    title: string;
    artist?: string;
    album?: string;
    cover_art_url?: string;
  }) => {
    setPlaylistModalTrack(track);
    setIsPlaylistModalOpen(true);
  };

  const handlePlayPlaylist = async (playlistId: string) => {
    setPlayingPlaylistId(playlistId);
    const pl = playlists.find((p) => p.id === playlistId);
    setPlayingOrigin({
      type: "playlist",
      id: playlistId,
      name: pl?.name || "Playlist",
    });
    try {
      const res = await executeQuery({
        query: "GetPlaylistTracks",
        payload: { playlist_id: playlistId },
      });
      const plTracks: Track[] = (res.data as any) || [];
      if (plTracks.length > 0) {
        const playlistTrackIds = new Set(plTracks.map((track) => track.id));
        const preservedQueue = Array.from(queuedTrackIds).filter(
          (trackId) => !playlistTrackIds.has(trackId),
        );
        const first = plTracks[0];
        const isOnline =
          first.format === "online" ||
          first.file_path?.startsWith("online://") ||
          first.id.startsWith("itunes:") ||
          first.id.startsWith("online:");

        if (isOnline) {
          if (!first.id.startsWith("jiosaavn:") && typeof navigator !== "undefined" && !navigator.onLine) {
            addAppNotification(
              "warning",
              "No Internet Connection",
              `Cannot play "${first.title}". Connect to the internet to stream online songs.`,
            );
            return;
          }

          const rec: DiscoveryRecommendation = {
            external_track_id: first.id,
            provider: "online",
            provider_id: first.id,
            title: first.title,
            artist: first.artist_name || "Unknown Artist",
            album: first.album_title,
            duration_secs: first.duration_secs || 210,
            cover_art_url: first.cover_art_url,
            preview_url: first.preview_url,
            match_status: "NOT_FOUND",
            recommendation_reason: "",
            in_wishlist: false,
          };
          await handlePlayOnlineTrack(rec, false, "playlist");
          for (let i = 1; i < plTracks.length; i++) {
            const nextTrack = plTracks[i];
            if (
              nextTrack.format === "online" ||
              nextTrack.file_path?.startsWith("online://")
            ) {
              await dispatchCommand({
                command: "EnqueueOnlineTrack",
                payload: {
                  track_id: nextTrack.id,
                  title: nextTrack.title,
                  artist: nextTrack.artist_name || "Unknown Artist",
                  album: nextTrack.album_title,
                  duration_secs: nextTrack.duration_secs,
                  cover_art_url: nextTrack.cover_art_url,
                  preview_url: nextTrack.preview_url,
                  play_next: false,
                },
              });
            } else {
              await dispatchCommand({
                command: "EnqueueTrack",
                payload: { track_id: nextTrack.id, play_next: false },
              });
            }
          }
        } else {
          await dispatchCommand({
            command: "PlayTrack",
            payload: { track_id: first.id, source: "playlist" },
          });
          for (let i = 1; i < plTracks.length; i++) {
            const nextTrack = plTracks[i];
            const nextIsOnline =
              nextTrack.format === "online" ||
              nextTrack.file_path?.startsWith("online://") ||
              nextTrack.id.startsWith("itunes:") ||
              nextTrack.id.startsWith("online:");
            if (nextIsOnline) {
              await dispatchCommand({
                command: "EnqueueOnlineTrack",
                payload: {
                  track_id: nextTrack.id,
                  title: nextTrack.title,
                  artist: nextTrack.artist_name || "Unknown Artist",
                  album: nextTrack.album_title,
                  duration_secs: nextTrack.duration_secs,
                  cover_art_url: nextTrack.cover_art_url,
                  preview_url: nextTrack.preview_url,
                  play_next: false,
                },
              });
            } else {
              await dispatchCommand({
                command: "EnqueueTrack",
                payload: { track_id: nextTrack.id, play_next: false },
              });
            }
          }
          fetchPlaybackState();
        }
        for (const queuedId of preservedQueue) {
          try {
            await dispatchCommand({
              command: "EnqueueTrack",
              payload: { track_id: queuedId, play_next: false },
            });
          } catch (err) {
            console.warn(
              "Could not preserve queued track after playlist:",
              err,
            );
          }
        }
      }
    } catch (err) {
      console.error("Failed to play playlist:", err);
    }
  };

  const handleFetchPlaylistTracks = async (
    playlistId: string,
  ): Promise<Track[]> => {
    try {
      const res = await executeQuery({
        query: "GetPlaylistTracks",
        payload: { playlist_id: playlistId },
      });
      return (res.data as any) || [];
    } catch (err) {
      console.error("Failed to fetch playlist tracks:", err);
      return [];
    }
  };

  // --- Collection Detail Handlers (Mixes, Top Charts, Playlists) ---
  const handleOpenCollection = useCallback(
    (collection: CollectionData) => {
      navigate({
        ...destinationRef.current,
        view: collection.playlistId ? "playlists" : destinationRef.current.view,
        collection,
        lyrics: false,
      });
      if (collection.playlistId || collection.type === "playlist") {
        setPlayingPlaylistId(collection.playlistId || collection.id);
      }
    },
    [navigate],
  );

  useEffect(() => {
    const collection = activeCollection;
    if (!collection || collection.tracks !== undefined) {
      setIsLoadingCollectionTracks(false);
      return;
    }
    let cancelled = false;
    const load = async () => {
      setIsLoadingCollectionTracks(true);
      try {
        if (collection.playlistId) {
          const res = await executeQuery({
            query: "GetPlaylistTracks",
            payload: { playlist_id: collection.playlistId },
          });
          const plTracks: Track[] = (res.data as any) || [];
          const items: CollectionTrackItem[] = plTracks.map((t) => {
            const isOnline =
              t.format === "online" ||
              t.file_path?.startsWith("online://") ||
              t.id.startsWith("itunes:") ||
              t.id.startsWith("online:");
            return {
              id: t.id,
              title: t.title,
              artist: t.artist_name || "Unknown Artist",
              album: t.album_title,
              duration_secs: t.duration_secs || 210,
              cover_art_url: t.cover_art_url,
              preview_url: t.preview_url,
              is_downloaded: !isOnline,
              matched_local_track_id: isOnline ? undefined : t.id,
              rawLocalTrack: isOnline ? undefined : t,
              rawRecommendation: isOnline
                ? {
                    external_track_id: t.id,
                    provider: t.playback_source?.provider || "online",
                    provider_id: t.playback_source?.track_id || t.id,
                    playback_source: t.playback_source,
                    title: t.title,
                    artist: t.artist_name || "Unknown Artist",
                    album: t.album_title,
                    duration_secs: t.duration_secs || 210,
                    cover_art_url: t.cover_art_url,
                    preview_url: t.preview_url,
                    match_status: "NOT_FOUND",
                    recommendation_reason: "",
                    in_wishlist: false,
                  }
                : undefined,
            };
          });
          setActiveCollection((prev) =>
            !cancelled && prev === collection
              ? { ...prev, tracks: items }
              : prev,
          );
        } else if (collection.type === "chart" || collection.searchQuery) {
          const res = await executeQuery(
            collection.type === "chart"
              ? {
                  query: "GetChartSongs",
                  payload: { chart_id: collection.id, limit: 50 },
                }
              : {
                  query: "SearchOnlineMusic",
                  payload: { query: collection.searchQuery!, limit: 50 },
                },
          );
          const onlineRecs: DiscoveryRecommendation[] = (res.data as any) || [];
          const items: CollectionTrackItem[] = onlineRecs.map((r) => {
            const validLocalId =
              r.matched_local_track_id &&
              !r.matched_local_track_id.startsWith("online:") &&
              !r.matched_local_track_id.startsWith("itunes:")
                ? r.matched_local_track_id
                : undefined;
            return {
              id: r.external_track_id,
              title: r.title,
              artist: r.artist,
              album: r.album,
              duration_secs: r.duration_secs || 210,
              cover_art_url: r.cover_art_url,
              preview_url: r.preview_url,
              is_downloaded: !!validLocalId,
              matched_local_track_id: validLocalId,
              rawRecommendation: r,
            };
          });
          setActiveCollection((prev) =>
            !cancelled && prev === collection
              ? { ...prev, tracks: items }
              : prev,
          );
        }
      } catch (err) {
        console.error("Failed to load collection tracks:", err);
      } finally {
        if (!cancelled) setIsLoadingCollectionTracks(false);
      }
    };
    void load();
    return () => {
      cancelled = true;
    };
  }, [activeCollection, setActiveCollection]);

  const handlePlayDiscoveredCollection = useCallback(
    async (collection: CollectionData) => {
      if (collection.type !== "chart" && !collection.searchQuery) return;
      if (typeof navigator !== "undefined" && !navigator.onLine) {
        addAppNotification(
          "warning",
          "No Internet Connection",
          `Cannot load ${collection.title}. Connect to the internet to stream this collection.`,
        );
        return;
      }

      try {
        const response = await executeQuery(
          collection.type === "chart"
            ? {
                query: "GetChartSongs",
                payload: { chart_id: collection.id, limit: 50 },
              }
            : {
                query: "SearchOnlineMusic",
                payload: { query: collection.searchQuery!, limit: 50 },
              },
        );
        const results = (
          (response?.data as DiscoveryRecommendation[]) || []
        ).filter(
          (track, index, all) =>
            all.findIndex(
              (candidate) =>
                candidate.external_track_id === track.external_track_id,
            ) === index,
        );
        if (results.length === 0) {
          addAppNotification(
            "warning",
            "No Songs Found",
            `${collection.title} did not return any playable songs.`,
          );
          return;
        }

        const localIdFor = (track: DiscoveryRecommendation) => {
          const localId = track.matched_local_track_id;
          return localId &&
            !localId.startsWith("online:") &&
            !localId.startsWith("itunes:")
            ? localId
            : null;
        };
        const resolvedId = (track: DiscoveryRecommendation) =>
          localIdFor(track) || track.external_track_id;
        const isLocal = (track: DiscoveryRecommendation) =>
          localIdFor(track) !== null;
        const collectionIds = new Set(results.map(resolvedId));
        const preservedQueue = Array.from(queuedTrackIds).filter(
          (trackId) => !collectionIds.has(trackId),
        );
        const first = results[0];
        const firstId = resolvedId(first);

        setPlayingOrigin({
          type: collection.type === "chart" ? "chart" : "mix",
          id: collection.id,
          name: collection.title,
          collectionData: {
            ...collection,
            tracks: results.map((track) => ({
              id: track.external_track_id,
              title: track.title,
              artist: track.artist,
              album: track.album,
              duration_secs: track.duration_secs || 210,
              cover_art_url: track.cover_art_url,
              preview_url: track.preview_url,
              is_downloaded: isLocal(track),
              matched_local_track_id: isLocal(track)
                ? resolvedId(track)
                : undefined,
              rawRecommendation: track,
            })),
          },
        });

        if (!isLocal(first)) {
          try {
            await dispatchCommand({
              command: "EnqueueOnlineTrack",
              payload: {
                track_id: firstId,
                title: first.title,
                artist: first.artist,
                album: first.album,
                duration_secs: first.duration_secs,
                cover_art_url: first.cover_art_url,
                preview_url: first.preview_url,
                play_next: false,
              },
            });
          } catch (error: any) {
            if (
              !String(error?.message || error)
                .toLowerCase()
                .includes("already in the queue")
            ) {
              throw error;
            }
          }
        }

        await dispatchCommand({
          command: "PlayTrack",
          payload: { track_id: firstId, source: "collection" },
        });

        for (const track of results.slice(1)) {
          const trackId = resolvedId(track);
          try {
            if (isLocal(track)) {
              await dispatchCommand({
                command: "EnqueueTrack",
                payload: { track_id: trackId, play_next: false },
              });
            } else {
              await dispatchCommand({
                command: "EnqueueOnlineTrack",
                payload: {
                  track_id: trackId,
                  title: track.title,
                  artist: track.artist,
                  album: track.album,
                  duration_secs: track.duration_secs,
                  cover_art_url: track.cover_art_url,
                  preview_url: track.preview_url,
                  play_next: false,
                },
              });
            }
          } catch (error) {
            console.warn("Could not enqueue chart track:", error);
          }
        }

        for (const trackId of preservedQueue) {
          try {
            await dispatchCommand({
              command: "EnqueueTrack",
              payload: { track_id: trackId, play_next: false },
            });
          } catch (error) {
            console.warn("Could not preserve queued track after chart:", error);
          }
        }

        fetchPlaybackState();
      } catch (error: any) {
        console.error("Failed to play discovered collection:", error);
        addAppNotification(
          "error",
          "Collection Playback Failed",
          error?.message || `Could not play ${collection.title}.`,
        );
      }
    },
    [
      queuedTrackIds,
      addAppNotification,
      handleStopOnlineAudio,
      fetchPlaybackState,
    ],
  );

  const handleOpenPlayingOrigin = useCallback(() => {
    if (!playingOrigin) return;
    if (playingOrigin.type === "flow") {
      navigate(viewDestination("home"));
      return;
    }
    if (playingOrigin.collectionData) {
      handleOpenCollection(playingOrigin.collectionData);
    } else if (playingOrigin.type === "playlist" && playingOrigin.id) {
      const pl = playlists.find((p) => p.id === playingOrigin.id);
      if (pl) {
        handleOpenCollection({
          id: pl.id,
          title: pl.name,
          type: "playlist",
          tag: "PLAYLIST",
          playlistId: pl.id,
          subtitle: `${pl.track_count || 0} tracks`,
          bgGradient: "linear-gradient(135deg, #4f46e5 0%, #1e1b4b 100%)",
          accentColor: "#6366f1",
        });
      }
    }
  }, [playingOrigin, playlists, handleOpenCollection, navigate]);

  const handlePlayCollectionTrack = useCallback(
    async (item: CollectionTrackItem) => {
      if (activeCollection) {
        setPlayingOrigin({
          type: activeCollection.type,
          id: activeCollection.id,
          name: activeCollection.title,
          collectionData: activeCollection,
        });
      }

      const hasRealLocalMatch =
        item.matched_local_track_id &&
        !item.matched_local_track_id.startsWith("online:") &&
        !item.matched_local_track_id.startsWith("itunes:");

      const isCurrentlyPlaying =
        (playbackState.is_playing || playbackState.is_paused) &&
        playbackState.current_track &&
        ((hasRealLocalMatch &&
          playbackState.current_track.id === item.matched_local_track_id) ||
          playbackState.current_track.id === item.id);

      if (isCurrentlyPlaying) {
        handleUnifiedPlayPause();
        return;
      }

      const enqueueCollectionRemainder = async () => {
        const collectionTracks = activeCollection?.tracks || [];
        const itemIdx = collectionTracks.findIndex(
          (track) => track.id === item.id,
        );
        const collectionIds = new Set(
          collectionTracks.map(
            (track) => track.matched_local_track_id || track.id,
          ),
        );
        for (let i = itemIdx + 1; i < collectionTracks.length; i++) {
          const queuedTrack = collectionTracks[i];
          const queuedId = queuedTrack.matched_local_track_id || queuedTrack.id;
          const isOnlineQueued =
            !queuedTrack.matched_local_track_id ||
            queuedTrack.matched_local_track_id.startsWith("online:") ||
            queuedTrack.matched_local_track_id.startsWith("itunes:");
          if (isOnlineQueued) {
            try {
              await dispatchCommand({
                command: "EnqueueOnlineTrack",
                payload: {
                  track_id: queuedId,
                  title: queuedTrack.title,
                  artist: queuedTrack.artist,
                  album: queuedTrack.album,
                  duration_secs: queuedTrack.duration_secs,
                  cover_art_url: queuedTrack.cover_art_url,
                  preview_url: queuedTrack.preview_url,
                  play_next: false,
                },
              });
            } catch (err) {
              console.warn("Could not enqueue online collection track:", err);
            }
          } else {
            try {
              await dispatchCommand({
                command: "EnqueueTrack",
                payload: { track_id: queuedId, play_next: false },
              });
            } catch (err) {
              console.warn("Could not enqueue collection track:", err);
            }
          }
        }
        for (const queuedId of queuedTrackIds) {
          if (collectionIds.has(queuedId)) continue;
          try {
            await dispatchCommand({
              command: "EnqueueTrack",
              payload: { track_id: queuedId, play_next: false },
            });
          } catch (err) {
            console.warn(
              "Could not preserve queued track after collection:",
              err,
            );
          }
        }
      };

      if (hasRealLocalMatch) {
        const requestId = beginPlaybackLoading(item.matched_local_track_id!);
        try {
          await dispatchCommand({
            command: "PlayTrack",
            payload: {
              track_id: item.matched_local_track_id!,
              source: "collection",
            },
          });
          await fetchPlaybackState();
        } finally {
          endPlaybackLoading(requestId);
        }
        await enqueueCollectionRemainder();
        return;
      }

      // Offline check: Cannot stream online audio without an internet connection
      if (!item.id.startsWith("jiosaavn:") && typeof navigator !== "undefined" && !navigator.onLine) {
        addAppNotification(
          "warning",
          "No Internet Connection",
          `Cannot play "${item.title}". Connect to the internet to stream online songs.`,
        );
        return;
      }

      if (item.rawRecommendation) {
        await handlePlayOnlineTrack(
          item.rawRecommendation,
          false,
          "collection",
        );
        await enqueueCollectionRemainder();
        return;
      }

      const rec: DiscoveryRecommendation = {
        external_track_id: item.id,
        provider: "online",
        provider_id: item.id,
        title: item.title,
        artist: item.artist,
        album: item.album,
        duration_secs: item.duration_secs,
        cover_art_url: item.cover_art_url,
        preview_url: item.preview_url,
        match_status: hasRealLocalMatch ? "EXACT_MATCH" : "NOT_FOUND",
        matched_local_track_id: hasRealLocalMatch
          ? item.matched_local_track_id
          : undefined,
        recommendation_reason: "From collection",
        in_wishlist: false,
      };
      await handlePlayOnlineTrack(rec, false, "collection");
      await enqueueCollectionRemainder();
    },
    [
      playbackState.is_playing,
      playbackState.is_paused,
      playbackState.current_track,
      activeCollection,
      queuedTrackIds,
      handleUnifiedPlayPause,
      handlePlayOnlineTrack,
      fetchPlaybackState,
      addAppNotification,
      beginPlaybackLoading,
      endPlaybackLoading,
    ],
  );

  const isPlayingThisCollection = useMemo(() => {
    if (
      !activeCollection ||
      !activeCollection.tracks ||
      activeCollection.tracks.length === 0
    ) {
      return false;
    }
    const currentId = playbackState.is_playing
      ? playbackState.current_track?.id
      : null;
    if (!currentId) return false;

    return activeCollection.tracks.some(
      (t) => t.id === currentId || t.matched_local_track_id === currentId,
    );
  }, [
    activeCollection,
    playbackState.is_playing,
    playbackState.current_track?.id,
  ]);

  const isCollectionSaved = useMemo(() => {
    if (!activeCollection) return false;
    if (activeCollection.type === "playlist" || activeCollection.playlistId)
      return true;
    return playlists.some(
      (p) =>
        p.is_smart_mix !== 1 &&
        (p.id === activeCollection.id ||
          p.id === activeCollection.playlistId ||
          p.name === activeCollection.title.trim()),
    );
  }, [activeCollection, playlists]);

  const handlePlayAllCollection = useCallback(async () => {
    if (
      !activeCollection ||
      !activeCollection.tracks ||
      activeCollection.tracks.length === 0
    ) {
      return;
    }

    if (isPlayingThisCollection) {
      handleUnifiedPlayPause();
      return;
    }

    await handlePlayCollectionTrack(activeCollection.tracks[0]);
  }, [
    activeCollection,
    isPlayingThisCollection,
    handleUnifiedPlayPause,
    handlePlayCollectionTrack,
  ]);

  const handleShuffleCollection = useCallback(async () => {
    if (
      !activeCollection ||
      !activeCollection.tracks ||
      activeCollection.tracks.length === 0
    ) {
      return;
    }

    // If shuffle is currently active, clicking it deactivates shuffle and continues with normal queue
    if (playbackState.is_shuffled) {
      await handleToggleShuffle();
      return;
    }

    // If this collection is already playing, simply activate shuffle
    if (isPlayingThisCollection) {
      await handleToggleShuffle();
      return;
    }

    // Otherwise, start playing this collection in shuffle mode
    const tracks = activeCollection.tracks;
    const localTracks = tracks.filter(
      (t) =>
        !!t.matched_local_track_id &&
        !t.matched_local_track_id.startsWith("online:") &&
        !t.matched_local_track_id.startsWith("itunes:"),
    );

    if (localTracks.length > 0) {
      handleStopOnlineAudio();
      // First enqueue tracks in their natural sequential order into the queue
      await dispatchCommand({ command: "ClearQueue" });
      await dispatchCommand({
        command: "PlayTrack",
        payload: {
          track_id: localTracks[0].matched_local_track_id!,
          source: "collection",
        },
      });
      for (let i = 1; i < localTracks.length; i++) {
        const tid = localTracks[i].matched_local_track_id;
        if (tid && !tid.startsWith("online:") && !tid.startsWith("itunes:")) {
          await dispatchCommand({
            command: "EnqueueTrack",
            payload: { track_id: tid, play_next: false },
          });
        }
      }

      // Pick a random track index to start with
      const randIdx = Math.floor(Math.random() * localTracks.length);
      if (randIdx !== 0) {
        await dispatchCommand({
          command: "PlayQueueIndex",
          payload: { index: randIdx },
        });
      }

      // Activate shuffle mode
      await dispatchCommand({
        command: "SetShuffle",
        payload: { enabled: true },
      });
      setPlaybackState((prev) => ({ ...prev, is_shuffled: true }));
      fetchPlaybackState();
    } else {
      // Online collection: pick a random track to start and activate shuffle
      const randIdx = Math.floor(Math.random() * tracks.length);
      const chosenTrack = tracks[randIdx];
      await handlePlayCollectionTrack(chosenTrack);
      await dispatchCommand({
        command: "SetShuffle",
        payload: { enabled: true },
      });
      setPlaybackState((prev) => ({ ...prev, is_shuffled: true }));
    }
  }, [
    activeCollection,
    playbackState.is_shuffled,
    isPlayingThisCollection,
    handleToggleShuffle,
    handleStopOnlineAudio,
    handlePlayCollectionTrack,
    fetchPlaybackState,
  ]);

  const handleInspectJioSaavnPlaylist = async (
    urlOrId: string,
  ): Promise<JioSaavnPlaylistImport | null> => {
    const res = await executeQuery({
      query: "ImportJioSaavnPlaylist",
      payload: { url_or_id: urlOrId },
    });
    return (res.data as JioSaavnPlaylistImport) || null;
  };

  const handleSaveImportedPlaylist = useCallback(
    async (
      name: string,
      trackIds: string[],
      tracksInfo?: Array<{
        id: string;
        title?: string;
        artist?: string;
        album?: string;
        duration_secs?: number;
        cover_art_url?: string;
        preview_url?: string;
      }>,
    ) => {
      let targetName = name.trim();
      while (
        playlists.some(
          (p) =>
            p.is_smart_mix !== 1 &&
            p.name.toLowerCase() === targetName.toLowerCase(),
        )
      ) {
        const prompted = await requestPrompt(
          "Playlist Name Already Taken",
          `A playlist named "${targetName}" already exists in your library. Please enter a new name for this playlist:`,
          `${targetName} (1)`,
        );
        if (prompted === null) {
          return;
        }
        const trimmed = prompted.trim();
        if (!trimmed) {
          addAppNotification(
            "warning",
            "Name Required",
            "Playlist name cannot be empty.",
          );
          continue;
        }
        targetName = trimmed;
      }

      const plRes = await dispatchCommand({
        command: "CreatePlaylist",
        payload: { name: targetName, description: "Saved Collection" },
      });
      const playlistId = (plRes as any)?.data;
      if (playlistId) {
        for (let i = 0; i < trackIds.length; i++) {
          const tid = trackIds[i];
          const info = tracksInfo?.[i];
          await dispatchCommand({
            command: "AddTrackToPlaylist",
            payload: {
              playlist_id: playlistId,
              track_id: tid,
              title: info?.title,
              artist: info?.artist,
              album: info?.album,
              duration_secs: info?.duration_secs,
              cover_art_url: info?.cover_art_url,
              preview_url: info?.preview_url,
            },
          });
        }
        fetchPlaylists();
      }
    },
    [playlists, fetchPlaylists],
  );

  const handleSaveCollectionToPlaylists = useCallback(
    async (collection: CollectionData) => {
      try {
        let targetName = collection.title.trim();
        while (
          playlists.some(
            (p) =>
              p.is_smart_mix !== 1 &&
              p.name.toLowerCase() === targetName.toLowerCase(),
          )
        ) {
          const prompted = await requestPrompt(
            "Playlist Name Already Taken",
            `A playlist named "${targetName}" already exists in your library. Please enter a new name for this playlist:`,
            `${targetName} (1)`,
          );
          if (prompted === null) {
            return;
          }
          const trimmed = prompted.trim();
          if (!trimmed) {
            addAppNotification(
              "warning",
              "Name Required",
              "Playlist name cannot be empty.",
            );
            continue;
          }
          targetName = trimmed;
        }

        const allTracks = collection.tracks || [];
        const trackIds = allTracks.map((t) =>
          t.matched_local_track_id &&
          !t.matched_local_track_id.startsWith("online:") &&
          !t.matched_local_track_id.startsWith("itunes:")
            ? t.matched_local_track_id
            : t.id,
        );

        if (trackIds.length > 0) {
          await handleSaveImportedPlaylist(targetName, trackIds, allTracks);
        } else {
          await handleCreatePlaylist(targetName, collection.subtitle);
          addAppNotification(
            "info",
            "Playlist Created",
            `Playlist "${targetName}" created.`,
          );
        }
      } catch (err) {
        console.error("Failed to save collection to playlists:", err);
      }
    },
    [playlists, handleSaveImportedPlaylist, handleCreatePlaylist],
  );

  const handleAddMissingToWishlist = async (missingTracks: any[]) => {
    await dispatchCommand({
      command: "AddMissingToWishlist",
      payload: { tracks: missingTracks },
    });
    fetchWishlist();
  };

  // Wishlist
  const handleAddToWishlist = async (
    title: string,
    artist: string,
    album?: string,
  ) => {
    await dispatchCommand({
      command: "AddToWishlist",
      payload: { title, artist, album },
    });
    fetchWishlist();
  };

  // Downloads
  const handleSearchDownloads = async (
    artist: string,
    title: string,
    album?: string,
  ): Promise<DownloadSearchResult[]> => {
    const res = await dispatchCommand({
      command: "SearchDownloads",
      payload: { artist, title, album },
    });
    const results = (res as any)?.data || (res as any)?.results;
    if (Array.isArray(results)) {
      return filterAndRankDownloadResults(results, artist, title);
    }
    return [];
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
    playbackState.duration_secs ||
    playbackState.current_track?.duration_secs ||
    0;
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
    (view: import("./components/Sidebar").ViewType) => {
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
  const runQueueCommand = useCallback(
    async (command: Parameters<typeof dispatchCommand>[0]) => {
      try {
        await dispatchCommand(command);
      } catch (error) {
        addAppNotification("error", "Queue action failed", String(error));
      }
    },
    [addAppNotification],
  );
  const handlePlayQueueIndex = useCallback(
    (index: number) => {
      void runQueueCommand({ command: "PlayQueueIndex", payload: { index } });
    },
    [runQueueCommand],
  );
  const handleDequeueTrack = useCallback(
    (track_id: string) => {
      void runQueueCommand({ command: "DequeueTrack", payload: { track_id } });
    },
    [runQueueCommand],
  );
  const handleClearQueue = useCallback(() => {
    void runQueueCommand({ command: "ClearQueue" });
  }, [runQueueCommand]);
  const handleOpenQueue = useCallback(() => {
    handleToggleFullscreen(false);
    navigate(viewDestination("queue"));
  }, [handleToggleFullscreen, navigate]);
  const handleOpenNowPlaying = useCallback(
    () => navigate(viewDestination("now_playing")),
    [navigate],
  );
  const handleStartDownload = async (
    search_result_id: string,
    wishlist_id?: string,
  ) => {
    await dispatchCommand({
      command: "StartDownload",
      payload: { search_result_id, wishlist_id },
    });
    await fetchDownloads();
  };
  const handleCancelDownload = async (task_id: string) => {
    await dispatchCommand({ command: "CancelDownload", payload: { task_id } });
    await fetchDownloads();
  };
  const handleUpdateWishlistStatus = async (
    wishlist_id: string,
    status: "want" | "ignore" | "already_own" | "downloaded",
  ) => {
    await dispatchCommand({
      command: "UpdateWishlistStatus",
      payload: { wishlist_id, status },
    });
    await fetchWishlist();
  };

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
      (!activeCollection && ["library", "albums", "playlists"].includes(currentView)));

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
        <div className="app-body">
          {/* Left Sidebar Navigation */}
          {interfacePreference.style === "spatial" && <Sidebar
            currentView={currentView}
            currentUser={currentUser}
            onSelectView={handleSelectView}
            unreadNotificationsCount={unreadNotificationsCount}
            playlists={playlists}
            activePlaylistId={
              activeCollection?.type === "playlist"
                ? activeCollection.playlistId || activeCollection.id
                : null
            }
            onSelectPlaylist={handleSelectSidebarPlaylist}
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
                  (currentView === "discovery" ? "Discover" : ["library", "albums", "playlists"].includes(currentView) ? "Library" : currentView === "stats" ? "Profile" : currentView.replace(/_/g, " ")))}
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
            {interfacePreference.style === "spatial" && !activeCollection && !isLyricsActive && ["library", "albums", "playlists"].includes(currentView) && <SpatialLibraryHeader view={currentView} albums={albums.length} onNavigate={handleSelectView} onLiked={likedSongs ? handleHomeLiked : undefined} />}

            {interfacePreference.style === "brutalist" && isArchiveSurface && <BrutalistLibraryHeader view={currentView} count={albums.length} likedSelected={isArchiveLiked} onNavigate={handleSelectView} onLiked={likedSongs ? handleHomeLiked : undefined} />}

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
                  onDownload={(artist, title) =>
                    handleInitiateDirectDownloadSearch(artist, title)
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
                  downloads={downloads}
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
                      activeTrackId={playbackState.current_track?.id}
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
                      currentTrackId={playbackState.current_track?.id}
                      isPlaying={playbackState.is_playing}
                      queuedTrackIds={queuedTrackIds}
                      onPlayTrack={handlePlayTrack}
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
                      onSelectPlaylist={() => {}}
                      onPlayPlaylist={handlePlayPlaylist}
                      onCreatePlaylist={handleCreatePlaylist}
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
                      onSelectPlaylist={() => {}}
                      onPlayPlaylist={handlePlayPlaylist}
                      onCreatePlaylist={handleCreatePlaylist}
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

        {recommendationNotice && <div className="recommendation-feedback-toast" role="status">
          <span>Won’t recommend “{recommendationNotice.title}” again.</span>
          <button onClick={() => { void dispatchCommand({ command: "RemoveTrackFeedback", payload: { track_id: recommendationNotice.id } }).catch(error => addAppNotification("error", "Couldn’t update recommendations", String(error))); }}>Allow again</button>
          <button aria-label="Dismiss recommendation message" onClick={() => setRecommendationNotice(null)}>×</button>
        </div>}

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
