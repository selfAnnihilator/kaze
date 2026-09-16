import React, { useState, useEffect, useCallback, useRef, useMemo } from "react";
import {
  Track,
  Album,
  Playlist,
  WishlistItem,
  DownloadTask,
  DownloadSearchResult,
  DiscoveryRecommendation,
  OnlinePlayingTrack,
  PlaybackState,
  AppSettings,
  OnboardingStatus,
  SpotifyPlaylistImport,
  AppNotification,
  CloudSyncStatus,
} from "./types";
import { dispatchCommand, executeQuery, subscribeBackendEvents } from "./services/api";
import { Sidebar, ViewType } from "./components/Sidebar";
import { NowPlayingBar } from "./components/NowPlayingBar";
import { OnboardingModal } from "./components/OnboardingModal";
import { LibraryView } from "./components/views/LibraryView";
import { AlbumsView } from "./components/views/AlbumsView";
import { PlaylistsView } from "./components/views/PlaylistsView";
import { DiscoveryView } from "./components/views/DiscoveryView";
import { NotificationsView } from "./components/views/NotificationsView";
import { SettingsView } from "./components/views/SettingsView";
import { GlobalTopSearchBar } from "./components/layout/GlobalTopSearchBar";
import { ToastContainer } from "./components/notifications/ToastContainer";
import { UpdateBanner } from "./components/UpdateBanner";
import {
  DownloadOptionsModal,
  DownloadModalTrack,
} from "./components/modals/DownloadOptionsModal";
import {
  AddToPlaylistModal,
  AddToPlaylistModalTrack,
} from "./components/modals/AddToPlaylistModal";
import {
  CollectionDetailView,
  CollectionData,
  CollectionTrackItem,
} from "./components/views/CollectionDetailView";
import { LyricsView } from "./components/views/LyricsView";
import { FullScreenPlayerView } from "./components/views/FullScreenPlayerView";
import { StatsView } from "./components/views/StatsView";
import { AuthModal } from "./components/modals/AuthModal";
import { StatsOverview, UserProfile } from "./types";
import { X, Sparkles } from "lucide-react";

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
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "16px" }}>
          <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
            <Sparkles size={20} color="var(--accent-secondary, #f3701e)" />
            <h3 style={{ fontSize: "1.15rem", fontWeight: 700, margin: 0, color: "#e8d8c9" }}>
              {title}
            </h3>
          </div>
          <button
            onClick={onCancel}
            style={{ background: "none", border: "none", color: "var(--text-muted, #a89f91)", cursor: "pointer", padding: "4px" }}
          >
            <X size={18} />
          </button>
        </div>

        <p style={{ fontSize: "0.88rem", color: "var(--text-dim, #8c8273)", margin: "0 0 16px 0", lineHeight: 1.5 }}>
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

export const App: React.FC = () => {
  // Navigation
  const [currentView, setCurrentView] = useState<ViewType>("discovery");
  const [activeCollection, setActiveCollection] = useState<CollectionData | null>(null);
  const [isLoadingCollectionTracks, setIsLoadingCollectionTracks] = useState<boolean>(false);

  // Playlist Management Modal
  const [isPlaylistModalOpen, setIsPlaylistModalOpen] = useState(false);
  const [playlistModalTrack, setPlaylistModalTrack] = useState<AddToPlaylistModalTrack | null>(null);
  const [trackPlaylistMap, setTrackPlaylistMap] = useState<Record<string, string[]>>({});

  // Onboarding
  const [onboardingStatus, setOnboardingStatus] = useState<OnboardingStatus | null>(null);
  const [showOnboarding, setShowOnboarding] = useState(false);

  // Online Preview / Stream Playback (Integrated into bottom NowPlayingBar)
  const [onlineTrack, setOnlineTrack] = useState<OnlinePlayingTrack | null>(null);
  const onlineAudioRef = useRef<HTMLAudioElement | null>(null);
  const activeOnlinePlayIdRef = useRef<number>(0);
  const handleNextTrackRef = useRef<() => void>(() => {});
  const handleQueuedOnlineTrackRef = useRef<(track: DiscoveryRecommendation) => void>(() => {});
  const handleStopOnlineAudioRef = useRef<() => void>(() => {});
  const onlineQueueMetadataRef = useRef<Map<string, DiscoveryRecommendation>>(new Map());
  const playingOnlineRecRef = useRef<DiscoveryRecommendation | null>(null);
  const repeatModeRef = useRef<PlaybackState["repeat_mode"]>("off");

  // User Authentication & Profile
  const [currentUser, setCurrentUser] = useState<UserProfile | null>(null);
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

  // Library & Content Data
  const [tracks, setTracks] = useState<Track[]>([]);
  const [queuedTrackIds, setQueuedTrackIds] = useState<Set<string>>(new Set());
  const tracksRef = useRef<Track[]>([]);
  tracksRef.current = tracks;
  const [albums, setAlbums] = useState<Album[]>([]);
  const [playlists, setPlaylists] = useState<Playlist[]>([]);
  const [playingPlaylistId, setPlayingPlaylistId] = useState<string | null>(null);
  const [, setWishlist] = useState<WishlistItem[]>([]);
  const [downloads, setDownloads] = useState<DownloadTask[]>([]);
  const [discoveryRecs, setDiscoveryRecs] = useState<DiscoveryRecommendation[]>([]);
  const discoveryRecsRef = useRef<DiscoveryRecommendation[]>([]);
  discoveryRecsRef.current = discoveryRecs;
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [isScanning, setIsScanning] = useState(false);
  const [cloudSyncStatus, setCloudSyncStatus] = useState<CloudSyncStatus | null>(null);
  const [isSyncingCloud, setIsSyncingCloud] = useState(false);

  // Notifications & Toasts System
  const [notifications, setNotifications] = useState<AppNotification[]>([]);
  const [toasts, setToasts] = useState<AppNotification[]>([]);

  // Middle Modal Popup for Track Download
  const [downloadModalTrack, setDownloadModalTrack] = useState<DownloadModalTrack | null>(null);

  // Global Top Search Bar State
  const [globalSearchQuery, setGlobalSearchQuery] = useState("");
  const [isGlobalSearching, setIsGlobalSearching] = useState(false);
  const [isGlobalRefreshing, setIsGlobalRefreshing] = useState(false);
  const [globalSearchResults, setGlobalSearchResults] = useState<DiscoveryRecommendation[] | null>(null);

  // Online / Offline Connectivity State
  const [isOnline, setIsOnline] = useState<boolean>(
    typeof navigator !== "undefined" ? navigator.onLine : true
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
  const [isLyricsActive, setIsLyricsActive] = useState(false);

  // Playing Origin state: identifies where the current track is playing from
  const [playingOrigin, setPlayingOrigin] = useState<{
    type: "playlist" | "mix" | "album" | "discovery";
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

  const requestPrompt = useCallback((title: string, message: string, defaultValue: string = ""): Promise<string | null> => {
    return new Promise((resolve) => {
      setPromptDialog({
        isOpen: true,
        title,
        message,
        defaultValue,
        resolve,
      });
    });
  }, []);

  // Local track cover art cache
  const [currentTrackCoverUrl, setCurrentTrackCoverUrl] = useState<string | null>(null);

  useEffect(() => {
    if (playbackState.current_track?.id) {
      if (playbackState.current_track.cover_art_url) {
        setCurrentTrackCoverUrl(playbackState.current_track.cover_art_url);
      }
      let isMounted = true;
      executeQuery({
        query: "GetTrackCoverArt",
        payload: { track_id: playbackState.current_track.id },
      })
        .then((res) => {
          if (isMounted && res.data) {
            setCurrentTrackCoverUrl(res.data);
          }
        })
        .catch(() => {});
      return () => {
        isMounted = false;
      };
    } else {
      setCurrentTrackCoverUrl(null);
    }
  }, [playbackState.current_track?.id, playbackState.current_track?.cover_art_url]);

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
    [isFullscreen]
  );

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.key === "Escape" && isFullscreen) || e.key === "F11") {
        e.preventDefault();
        handleToggleFullscreen(e.key === "Escape" ? false : undefined);
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
    (type: "success" | "error" | "info" | "warning", title: string, message: string) => {
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
    []
  );

  const dismissToast = useCallback((id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  const markNotificationAsRead = useCallback((id: string) => {
    setNotifications((prev) =>
      prev.map((n) => (n.id === id ? { ...n, read: true } : n))
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
    try {
      const res = await executeQuery({ query: "GetPlaybackState" });
      if (res.data) {
        setPlaybackState((prev) => {
          const updated = { ...prev, ...res.data };
          if (res.data.current_track) {
            updated.current_track = res.data.current_track;
          } else if (res.data.current_track_id) {
            const found = tracksRef.current.find((t) => t.id === res.data.current_track_id);
            if (found) updated.current_track = found;
          }
          return updated;
        });
        if (Array.isArray(res.data.queue_track_ids)) {
          setQueuedTrackIds(new Set(res.data.queue_track_ids));
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
        payload: { limit: 50 },
      });
      if (Array.isArray(res.data)) {
        setDownloads(res.data);
      }
    } catch (err) {
      console.error("Failed to fetch downloads:", err);
    }
  }, []);

  const fetchDiscovery = useCallback(async (forceRefresh: boolean = false) => {
    try {
      const res = await executeQuery({
        query: "GetDiscoveryRecommendations",
        payload: { limit: 100, force_refresh: forceRefresh },
      });
      if (Array.isArray(res.data)) {
        setDiscoveryRecs(res.data);
      }
    } catch (err) {
      console.error("Failed to fetch discovery recommendations:", err);
    }
  }, []);

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

  const fetchCloudSyncStatus = useCallback(async () => {
    try {
      const res = await executeQuery({ query: "GetCloudSyncStatus" });
      if (res && res.type === "CloudSyncStatus" && res.data) {
        setCloudSyncStatus(res.data);
      }
    } catch (err) {
      console.warn("Failed to fetch cloud sync status:", err);
    }
  }, []);

  const handleSyncCloud = async () => {
    setIsSyncingCloud(true);
    try {
      await dispatchCommand({ command: "SyncCloudData" });
      addAppNotification("success", "Cloud Synchronized", "Your playlists, songs, and stats have been synchronized with Cloudflare D1.");
      await fetchCloudSyncStatus();
      await fetchPlaylists();
      await fetchTracks();
    } catch (err: any) {
      addAppNotification("error", "Sync Failed", err?.message || "Failed to synchronize with cloud.");
    } finally {
      setIsSyncingCloud(false);
    }
  };

  const handleSetCloudUrl = async (url: string) => {
    try {
      await dispatchCommand({ command: "SetCloudServerUrl", payload: { url } });
      addAppNotification("success", "Cloud Endpoint Updated", `Configured worker URL: ${url}`);
      await fetchCloudSyncStatus();
    } catch (err: any) {
      addAppNotification("error", "Failed to update URL", err?.message || "Could not save cloud server URL.");
    }
  };

  const handleLogin = async (username: string, password: string): Promise<UserProfile | null> => {
    const res = await dispatchCommand({
      command: "Login",
      payload: { username, password },
    });
    if (res && res.status === "UserProfile" && res.data) {
      setCurrentUser(res.data);
      addAppNotification("success", "Logged In", `Welcome back, ${res.data.username}!`);
      await fetchPlaylists();
      await fetchCloudSyncStatus();
      return res.data;
    }
    return null;
  };

  const handleSignUp = async (username: string, password: string): Promise<UserProfile | null> => {
    const res = await dispatchCommand({
      command: "SignUp",
      payload: { username, password },
    });
    if (res && res.status === "UserProfile" && res.data) {
      setCurrentUser(res.data);
      addAppNotification("success", "Account Created", `Welcome to Kaze, ${res.data.username}!`);
      await fetchPlaylists();
      await fetchCloudSyncStatus();
      return res.data;
    }
    return null;
  };

  const handleLogout = async () => {
    try {
      await dispatchCommand({ command: "Logout" });
    } catch (err) {
      console.error("Logout failed:", err);
    } finally {
      window.location.reload();
    }
  };

  const handleLogoutAll = async () => {
    try {
      await dispatchCommand({ command: "LogoutAll" });
    } catch (err) {
      console.error("Logout all failed:", err);
    } finally {
      window.location.reload();
    }
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
    []
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
    fetchSettings();
    fetchCurrentUser();
    fetchCloudSyncStatus();
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
    fetchSettings,
    fetchCurrentUser,
    fetchCloudSyncStatus,
  ]);

  // Backend Event Subscriptions
  useEffect(() => {
    let unlistenFn: (() => void) | null = null;

    subscribeBackendEvents((event: any) => {
      if (!event || !event.event) return;
      console.log("[Backend Event]", event.event, event.payload);

      switch (event.event) {
        case "PlaybackStarted": {
          handleStopOnlineAudioRef.current();
          const trackId = event.payload?.track_id;
          const found = tracksRef.current.find((t) => t.id === trackId);
          const track: Track = found || {
            id: trackId,
            file_path: "",
            title: event.payload?.title || "Unknown Track",
            artist_name: event.payload?.artist || "Unknown Artist",
            duration_secs: event.payload?.duration_secs || 0,
            format: "mp3",
            has_cover_art: 0,
          };
          setPlaybackState((prev) => ({
            ...prev,
            current_track: track,
            is_playing: true,
            position_secs: 0,
            duration_secs: event.payload?.duration_secs || track.duration_secs || prev.duration_secs,
          }));
          break;
        }

        case "PlaybackPaused":
          setPlaybackState((prev) => ({
            ...prev,
            is_playing: false,
            position_secs: event.payload?.position_secs ?? prev.position_secs,
          }));
          break;

        case "PlaybackResumed":
          setPlaybackState((prev) => ({
            ...prev,
            is_playing: true,
            position_secs: event.payload?.position_secs ?? prev.position_secs,
          }));
          break;

        case "PlaybackStopped":
          setPlaybackState((prev) => ({
            ...prev,
            is_playing: false,
            position_secs: 0,
          }));
          break;

        case "OnlinePlaybackRequested": {
          const trackId = event.payload?.track_id;
          const known = onlineQueueMetadataRef.current.get(trackId) ||
            discoveryRecsRef.current.find((rec) => rec.external_track_id === trackId);
          const rec: DiscoveryRecommendation = known || {
            external_track_id: trackId,
            provider: "online",
            provider_id: trackId,
            title: event.payload?.title || "Unknown Track",
            artist: event.payload?.artist || "Unknown Artist",
            album: event.payload?.album,
            duration_secs: event.payload?.duration_secs || 210,
            match_status: "NOT_FOUND",
            recommendation_reason: "Queued track",
            in_wishlist: false,
          };
          handleQueuedOnlineTrackRef.current(rec);
          break;
        }

        case "PlaybackSeeked":
          setPlaybackState((prev) => ({
            ...prev,
            position_secs: event.payload?.position_secs ?? prev.position_secs,
          }));
          break;

        case "PlaybackPositionChanged":
          setPlaybackState((prev) => ({
            ...prev,
            position_secs: event.payload?.position_secs ?? prev.position_secs,
            duration_secs: event.payload?.duration_secs ?? prev.duration_secs,
          }));
          break;

        case "PlaybackVolumeChanged":
        case "VolumeChanged":
          setPlaybackState((prev) => ({
            ...prev,
            volume: event.payload?.volume ?? prev.volume,
            is_muted: event.payload?.is_muted ?? prev.is_muted,
          }));
          break;

        case "QueueUpdated": {
          if (Array.isArray(event.payload?.queue_track_ids)) {
            setQueuedTrackIds(new Set(event.payload.queue_track_ids));
          } else {
            const items: any[] = event.payload?.items || [];
            const curr = event.payload?.current_index;
            const upcoming = (curr !== null && curr !== undefined) ? items.slice(curr + 1) : items;
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

        case "LibraryScanCompleted":
        case "ScanCompleted":
          setIsScanning(false);
          addAppNotification("info", "Library Scan Complete", "Your local music library has been updated with the latest tracks.");
          fetchTracks();
          fetchAlbums();
          fetchOnboardingStatus();
          fetchDiscovery();
          break;

        case "LibraryScanFailed":
          setIsScanning(false);
          addAppNotification("error", "Library Scan Failed", "An error occurred while scanning your music folders.");
          break;

        case "DownloadQueued":
          fetchDownloads();
          break;

        case "DownloadProgress":
        case "DownloadProgressChanged":
          setDownloads((prev) =>
            prev.map((t) =>
              t.id === event.payload?.task_id
                ? {
                    ...t,
                    bytes_downloaded: event.payload.bytes_downloaded,
                    file_size: event.payload.total_bytes || event.payload.file_size || t.file_size,
                  }
                : t
            )
          );
          break;

        case "DownloadCompleted": {
          const trackTitle = event.payload?.title || "Track";
          const artist = event.payload?.artist ? ` by ${event.payload.artist}` : "";
          addAppNotification("success", "Download Complete", `"${trackTitle}"${artist} has been downloaded and added to your library.`);
          fetchDownloads();
          fetchWishlist();
          fetchTracks();
          fetchAlbums();
          fetchDiscovery();
          break;
        }

        case "DownloadFailed": {
          const trackTitle = event.payload?.title || "Track";
          const errReason = event.payload?.error || event.payload?.message || "Connection timed out or peer unavailable.";
          addAppNotification("error", "Download Failed", `Failed downloading "${trackTitle}": ${errReason}`);
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

        case "SessionChanged": {
          const user = event.payload?.user;
          if (user) {
            setCurrentUser(user);
            fetchPlaylists();
            fetchTrackPlaylistMemberships();
            fetchCloudSyncStatus();
          } else {
            setCurrentUser(null);
            setPlaylists((prev) => prev.filter((p) => p.is_smart_mix === 1));
            setTrackPlaylistMap({});
            fetchCloudSyncStatus();
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
      unlistenFn = fn;
    });

    return () => {
      if (unlistenFn) unlistenFn();
    };
  }, [fetchTracks, fetchAlbums, fetchOnboardingStatus, fetchDownloads, fetchWishlist, addAppNotification, fetchPlaylists, fetchTrackPlaylistMemberships, fetchCloudSyncStatus]);

  // Smooth local playback progression ticker while playing
  useEffect(() => {
    if (!playbackState.is_playing) return;

    const interval = setInterval(() => {
      setPlaybackState((prev) => {
        if (!prev.is_playing) return prev;
        const maxDur = prev.duration_secs || prev.current_track?.duration_secs || 0;
        const nextPos = prev.position_secs + 0.25;
        if (maxDur > 0 && nextPos >= maxDur) {
          fetchPlaybackState();
          return { ...prev, position_secs: maxDur };
        }
        return { ...prev, position_secs: nextPos };
      });
    }, 250);

    return () => clearInterval(interval);
  }, [playbackState.is_playing, fetchPlaybackState]);

  // Periodic state reconciliation with backend while playing
  useEffect(() => {
    if (!playbackState.is_playing) return;

    const interval = setInterval(() => {
      fetchPlaybackState();
    }, 2000);

    return () => clearInterval(interval);
  }, [playbackState.is_playing, fetchPlaybackState]);

  // --- Actions & Commands ---

  const handleStopOnlineAudio = useCallback(() => {
    activeOnlinePlayIdRef.current++;
    if (onlineAudioRef.current) {
      const listened = onlineAudioRef.current.currentTime;
      const rec = playingOnlineRecRef.current;
      if (rec && listened > 5) {
        const dur = onlineAudioRef.current.duration || rec.duration_secs || 210;
        const completed = listened >= dur - 3;
        dispatchCommand({
          command: "RecordPlaybackSession",
          payload: {
            track_id: rec.external_track_id,
            title: rec.title,
            artist: rec.artist,
            album: rec.album,
            duration_secs: dur,
            seconds_listened: listened,
            completed,
            skipped: !completed && listened < 30,
            source: "online",
          },
        }).catch(console.warn);
      }
      onlineAudioRef.current.pause();
      onlineAudioRef.current.removeAttribute("src");
      onlineAudioRef.current.load();
      onlineAudioRef.current = null;
    }
    playingOnlineRecRef.current = null;
    setOnlineTrack(null);
  }, []);

  handleStopOnlineAudioRef.current = handleStopOnlineAudio;
  repeatModeRef.current = playbackState.repeat_mode;

  const handleStopTrack = useCallback(
    async (rec?: DiscoveryRecommendation) => {
      activeOnlinePlayIdRef.current++;
      if (onlineTrack && (!rec || onlineTrack.id === rec.external_track_id)) {
        if (onlineAudioRef.current) {
          onlineAudioRef.current.pause();
          onlineAudioRef.current.removeAttribute("src");
          onlineAudioRef.current.load();
          onlineAudioRef.current = null;
        }
        setOnlineTrack(null);
        return;
      }

      if (playbackState.is_playing) {
        await dispatchCommand({ command: "Pause" });
        setPlaybackState((prev) => ({ ...prev, is_playing: false }));
        return;
      }

      if (onlineAudioRef.current) {
        onlineAudioRef.current.pause();
        onlineAudioRef.current.removeAttribute("src");
        onlineAudioRef.current.load();
        onlineAudioRef.current = null;
        setOnlineTrack(null);
      }
    },
    [onlineTrack, playbackState.is_playing]
  );

  const handlePlayTrack = useCallback(
    async (trackId: string) => {
      const preservedQueue = Array.from(queuedTrackIds).filter((queuedId) => queuedId !== trackId);
      handleStopOnlineAudio();
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
      for (const queuedId of preservedQueue) {
        try {
          await dispatchCommand({
            command: "EnqueueTrack",
            payload: { track_id: queuedId, play_next: false },
          });
        } catch (err) {
          console.warn("Could not restore queued track after direct playback:", err);
        }
      }
      // Immediately fetch updated track/state
      fetchPlaybackState();
    },
    [tracks, queuedTrackIds, handleStopOnlineAudio, fetchPlaybackState]
  );

  const handlePlayOnlineTrack = useCallback(
    async (
      rec: DiscoveryRecommendation,
      fromBackendQueue = false,
      playbackSource: "online" | "collection" | "playlist" = "online"
    ) => {
      const hasRealLocalMatch =
        !!rec.matched_local_track_id &&
        !rec.matched_local_track_id.startsWith("itunes:") &&
        !rec.matched_local_track_id.startsWith("online:");

      // 1. Check if this track is currently playing locally through Rodio
      const isCurrentlyPlayingLocal =
        playbackState.is_playing &&
        playbackState.current_track &&
        playbackState.current_track.format !== "online" &&
        !playbackState.current_track.file_path.startsWith("online://") &&
        ((hasRealLocalMatch && playbackState.current_track.id === rec.matched_local_track_id) ||
          rec.external_track_id === playbackState.current_track.id ||
          (rec.title.toLowerCase().trim() === playbackState.current_track.title.toLowerCase().trim() &&
            playbackState.current_track.artist_name &&
            rec.artist.toLowerCase().trim() === playbackState.current_track.artist_name.toLowerCase().trim()));

      if (isCurrentlyPlayingLocal) {
        await dispatchCommand({ command: "Pause" });
        setPlaybackState((prev) => ({ ...prev, is_playing: false }));
        return;
      }

      // 2. If this track is in local library, play it locally through Rodio!
      if (hasRealLocalMatch) {
        handleStopOnlineAudio();
        await handlePlayTrack(rec.matched_local_track_id!);
        return;
      }

      // Offline check: Cannot stream online audio without an internet connection
      if (typeof navigator !== "undefined" && !navigator.onLine) {
        addAppNotification(
          "warning",
          "No Internet Connection",
          `Cannot play "${rec.title}". Connect to the internet to stream online songs.`
        );
        return;
      }

      // 3. If it's already the active online track, toggle play/pause
      if (onlineAudioRef.current && onlineTrack?.id === rec.external_track_id) {
        if (onlineTrack.isPlaying) {
          onlineAudioRef.current.pause();
          setOnlineTrack((prev) => (prev ? { ...prev, isPlaying: false } : null));
        } else {
          onlineAudioRef.current.play().catch(console.warn);
          setOnlineTrack((prev) => (prev ? { ...prev, isPlaying: true } : null));
        }
        return;
      }

      if (!fromBackendQueue) {
        onlineQueueMetadataRef.current.set(rec.external_track_id, rec);
        try {
          await dispatchCommand({
            command: "PlayTrack",
            payload: { track_id: rec.external_track_id, source: playbackSource },
          });
          return;
        } catch (err) {
          console.warn("Could not register online track with the shared queue; playing directly:", err);
        }
      }

      // 4. Stop local playback if active
      if (playbackState.is_playing) {
        await dispatchCommand({ command: "Pause" });
        setPlaybackState((prev) => ({ ...prev, is_playing: false }));
      }

      // 5. Stop existing online audio & acquire new playId token
      handleStopOnlineAudio();
      const playId = ++activeOnlinePlayIdRef.current;

      // Initial state on player bar
      setOnlineTrack({
        id: rec.external_track_id,
        title: rec.title,
        artist: rec.artist,
        album: rec.album,
        cover_art_url: rec.cover_art_url,
        preview_url: rec.preview_url,
        duration: rec.duration_secs || 210,
        currentTime: 0,
        isPlaying: true,
        isLoading: true,
      });

      // 6. Resolve full song stream via backend
      let streamUrl = rec.preview_url || "";
      let duration = rec.duration_secs || 210;

      try {
        const res = await executeQuery({
          query: "ResolveFullTrackAudio",
          payload: { artist: rec.artist, title: rec.title },
        });
        if (activeOnlinePlayIdRef.current !== playId) return;
        if (res && res.type === "FullTrackAudio" && res.data && res.data.stream_url) {
          streamUrl = res.data.stream_url;
          if (res.data.duration_secs) {
            duration = res.data.duration_secs;
          }
        }
      } catch (err) {
        if (activeOnlinePlayIdRef.current !== playId) return;
        console.warn("Could not resolve full stream, falling back to preview URL:", err);
      }

      if (activeOnlinePlayIdRef.current !== playId) {
        return;
      }

      if (!streamUrl) {
        setOnlineTrack(null);
        return;
      }

      const audio = new Audio(streamUrl);
      audio.volume = playbackState.is_muted ? 0 : playbackState.volume;
      onlineAudioRef.current = audio;
      playingOnlineRecRef.current = rec;

      audio.ontimeupdate = () => {
        if (activeOnlinePlayIdRef.current !== playId) return;
        const dur =
          audio.duration && !isNaN(audio.duration) && audio.duration > 0
            ? audio.duration
            : duration;
        setOnlineTrack((prev) =>
          prev && prev.id === rec.external_track_id
            ? { ...prev, currentTime: audio.currentTime, duration: dur, isLoading: false }
            : prev
        );
      };

      audio.onplay = () => {
        if (activeOnlinePlayIdRef.current !== playId) return;
        setOnlineTrack((prev) =>
          prev && prev.id === rec.external_track_id
            ? { ...prev, isPlaying: true, isLoading: false }
            : prev
        );
      };

      audio.onpause = () => {
        if (activeOnlinePlayIdRef.current !== playId) return;
        setOnlineTrack((prev) =>
          prev && prev.id === rec.external_track_id
            ? { ...prev, isPlaying: false }
            : prev
        );
      };

      audio.onended = () => {
        if (activeOnlinePlayIdRef.current !== playId) return;
        const listened = audio.currentTime;
        dispatchCommand({
          command: "RecordPlaybackSession",
          payload: {
            track_id: rec.external_track_id,
            title: rec.title,
            artist: rec.artist,
            album: rec.album,
            duration_secs: duration,
            seconds_listened: listened,
            completed: true,
            skipped: false,
            source: "online",
          },
        }).catch(console.warn);
        if (repeatModeRef.current === "one") {
          audio.currentTime = 0;
          audio.play().catch(console.warn);
          return;
        }

        playingOnlineRecRef.current = null;

        if (handleNextTrackRef.current) {
          handleNextTrackRef.current();
        } else {
          handleStopOnlineAudio();
        }
      };

      audio.onerror = () => {
        if (activeOnlinePlayIdRef.current !== playId) return;
        if (rec.preview_url && streamUrl !== rec.preview_url) {
          const fallback = new Audio(rec.preview_url);
          fallback.volume = playbackState.is_muted ? 0 : playbackState.volume;
          onlineAudioRef.current = fallback;
          fallback.ontimeupdate = () => {
            if (activeOnlinePlayIdRef.current !== playId) return;
            setOnlineTrack((prev) =>
              prev && prev.id === rec.external_track_id
                ? { ...prev, currentTime: fallback.currentTime, duration: 30, isLoading: false }
                : prev
            );
          };
          fallback.onplay = () => {
            if (activeOnlinePlayIdRef.current !== playId) return;
            setOnlineTrack((prev) =>
              prev && prev.id === rec.external_track_id
                ? { ...prev, isPlaying: true, isLoading: false }
                : prev
            );
          };
          fallback.onpause = () => {
            if (activeOnlinePlayIdRef.current !== playId) return;
            setOnlineTrack((prev) =>
              prev && prev.id === rec.external_track_id
                ? { ...prev, isPlaying: false }
                : prev
            );
          };
          fallback.onended = () => {
            if (activeOnlinePlayIdRef.current !== playId) return;
            if (repeatModeRef.current === "one") {
              fallback.currentTime = 0;
              fallback.play().catch(console.warn);
              return;
            }
            if (handleNextTrackRef.current) {
              handleNextTrackRef.current();
            } else {
              handleStopOnlineAudio();
            }
          };
          fallback.play().catch(console.warn);
        } else {
          handleStopOnlineAudio();
        }
      };

      audio.play().catch((e) => {
        if (activeOnlinePlayIdRef.current !== playId) return;
        console.warn("Online audio play prevented:", e);
        setOnlineTrack((prev) => (prev ? { ...prev, isPlaying: false, isLoading: false } : null));
      });
    },
    [playbackState.is_playing, playbackState.current_track, playbackState.volume, playbackState.is_muted, onlineTrack, handleStopOnlineAudio, handlePlayTrack, addAppNotification]
  );

  handleQueuedOnlineTrackRef.current = (rec) => {
    void handlePlayOnlineTrack(rec, true);
  };

  const handlePlayPause = async () => {
    if (playbackState.is_playing) {
      await dispatchCommand({ command: "Pause" });
      setPlaybackState((prev) => ({ ...prev, is_playing: false }));
    } else {
      await dispatchCommand({ command: "Resume" });
      setPlaybackState((prev) => ({ ...prev, is_playing: true }));
    }
  };

  const handleUnifiedPlayPause = async () => {
    if (playbackState.is_playing) {
      handleStopOnlineAudio();
      handlePlayPause();
      return;
    }
    if (onlineTrack && onlineAudioRef.current) {
      if (onlineTrack.isPlaying) {
        onlineAudioRef.current.pause();
        setOnlineTrack((prev) => (prev ? { ...prev, isPlaying: false } : null));
      } else {
        onlineAudioRef.current.play().catch(console.warn);
        setOnlineTrack((prev) => (prev ? { ...prev, isPlaying: true } : null));
      }
      return;
    }
    handlePlayPause();
  };

  const handleNextTrack = async () => {
    if (onlineAudioRef.current) {
      handleStopOnlineAudio();
    }
    await dispatchCommand({ command: "NextTrack" });
  };

  const handlePreviousTrack = async () => {
    if (onlineAudioRef.current) {
      handleStopOnlineAudio();
    }
    await dispatchCommand({ command: "PreviousTrack" });
  };

  handleNextTrackRef.current = handleNextTrack;

  const handleSeek = async (position_secs: number) => {
    await dispatchCommand({
      command: "Seek",
      payload: { position_secs },
    });
    setPlaybackState((prev) => ({ ...prev, position_secs }));
  };

  const handleUnifiedSeek = async (position_secs: number) => {
    if (onlineTrack && onlineAudioRef.current) {
      onlineAudioRef.current.currentTime = position_secs;
      setOnlineTrack((prev) => (prev ? { ...prev, currentTime: position_secs } : null));
      return;
    }
    handleSeek(position_secs);
  };

  const handleVolumeChange = async (volume: number) => {
    if (onlineAudioRef.current) {
      onlineAudioRef.current.volume = volume;
    }
    await dispatchCommand({
      command: "SetVolume",
      payload: { volume },
    });
    setPlaybackState((prev) => ({ ...prev, volume }));
  };

  const handleToggleMute = async () => {
    if (onlineAudioRef.current) {
      onlineAudioRef.current.muted = !playbackState.is_muted;
    }
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
      prev.map((t) => (t.id === trackId ? { ...t, manual_like: 1 } : t))
    );
    setPlaybackState((prev) =>
      prev.current_track?.id === trackId
        ? { ...prev, current_track: { ...prev.current_track, manual_like: 1 } }
        : prev
    );
    await dispatchCommand({
      command: "LikeTrack",
      payload: { track_id: trackId },
    });
    await Promise.all([fetchPlaylists(), fetchTrackPlaylistMemberships()]);
  };

  const handleRemoveFeedback = async (trackId: string) => {
    setTracks((prev) =>
      prev.map((t) => (t.id === trackId ? { ...t, manual_like: 0 } : t))
    );
    setPlaybackState((prev) =>
      prev.current_track?.id === trackId
        ? { ...prev, current_track: { ...prev.current_track, manual_like: 0 } }
        : prev
    );
    await dispatchCommand({
      command: "RemoveTrackFeedback",
      payload: { track_id: trackId },
    });
    await Promise.all([fetchPlaylists(), fetchTrackPlaylistMemberships()]);
  };

  const handleEnqueueTrack = async (trackId: string) => {
    if (queuedTrackIds.has(trackId)) {
      addAppNotification("info", "Already in Queue", "This song is already waiting in the queue.");
      return;
    }
    try {
      await dispatchCommand({
        command: "EnqueueTrack",
        payload: { track_id: trackId, play_next: false },
      });
      setQueuedTrackIds((prev) => new Set(prev).add(trackId));
    } catch (err: any) {
      if (String(err?.message || err).toLowerCase().includes("already in the queue")) {
        setQueuedTrackIds((prev) => new Set(prev).add(trackId));
        addAppNotification("info", "Already in Queue", "This song is already waiting in the queue.");
        return;
      }
      addAppNotification("error", "Queue Failed", err?.message || "Could not add this song to the queue.");
    }
  };

  const handleEnqueueRecommendation = async (rec: DiscoveryRecommendation) => {
    const hasLocalMatch =
      !!rec.matched_local_track_id &&
      !rec.matched_local_track_id.startsWith("online:") &&
      !rec.matched_local_track_id.startsWith("itunes:");
    const trackId = hasLocalMatch ? rec.matched_local_track_id! : rec.external_track_id;
    if (!hasLocalMatch) {
      onlineQueueMetadataRef.current.set(trackId, rec);
      if (queuedTrackIds.has(trackId)) {
        addAppNotification("info", "Already in Queue", "This song is already waiting in the queue.");
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
        if (String(err?.message || err).toLowerCase().includes("already in the queue")) {
          setQueuedTrackIds((prev) => new Set(prev).add(trackId));
          addAppNotification("info", "Already in Queue", "This song is already waiting in the queue.");
        } else {
          addAppNotification("error", "Queue Failed", err?.message || "Could not add this song to the queue.");
        }
      }
      return;
    }
    await handleEnqueueTrack(trackId);
  };

  const handleSearchLibrary = async (queryText: string) => {
    if (!queryText.trim()) {
      fetchTracks();
      return;
    }
    try {
      const res = await executeQuery({
        query: "SearchLibrary",
        payload: { query_text: queryText, limit: 100 },
      });
      if (Array.isArray(res.data)) {
        setTracks(res.data);
      }
    } catch (err) {
      console.error("Search failed:", err);
    }
  };

  const handleSelectAlbum = useCallback(
    (albumId: string) => {
      const album = albums.find((al) => al.id === albumId);
      if (album) {
        handleSearchLibrary(album.title);
        setCurrentView("library");
      }
    },
    [albums]
  );

  // Onboarding completion
  const handleCompleteOnboarding = async (folders: string[], startScan: boolean) => {
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
    await dispatchCommand({
      command: "ScanLibrary",
      payload: { incremental: false },
    });
  };

  const handleRerunOnboarding = async () => {
    await dispatchCommand({ command: "ResetOnboarding" });
    await fetchOnboardingStatus();
    setShowOnboarding(true);
  };

  // Playlists & Smart Mixes
  const handleRenamePlaylist = async (playlistId: string, name: string) => {
    await dispatchCommand({ command: "RenamePlaylist", payload: { playlist_id: playlistId, name } });
    setActiveCollection((current) =>
      current && (current.playlistId === playlistId || current.id === playlistId)
        ? { ...current, title: name.trim() }
        : current
    );
    await fetchPlaylists();
  };

  const handleDeletePlaylist = async (playlistId: string) => {
    await dispatchCommand({ command: "DeletePlaylist", payload: { playlist_id: playlistId } });
    await fetchPlaylists();
    if (activeCollection?.playlistId === playlistId || activeCollection?.id === playlistId) {
      setActiveCollection(null);
    }
  };

  // Online likes are represented by membership in the account's Liked Songs playlist.
  const handleLikeOnline = async (track: { id: string; title: string; artist: string; album?: string; cover_art_url?: string; preview_url?: string; duration_secs?: number }) => {
    const res = await dispatchCommand({ command: "EnsureLikedSongsPlaylist" });
    const likedId = (res as any)?.data;
    if (likedId) {
      const isLiked = (trackPlaylistMap[track.id] || []).includes(likedId);
      if (isLiked) {
        await dispatchCommand({
          command: "RemoveTrackFromPlaylist",
          payload: { playlist_id: likedId, track_id: track.id },
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
      }
      await fetchPlaylists();
      await fetchTrackPlaylistMemberships();
    }
  };

  const handleToggleCollectionLike = async (track: CollectionTrackItem, isLiked: boolean) => {
    const hasLocalTrack =
      !!track.matched_local_track_id &&
      !track.matched_local_track_id.startsWith("online:") &&
      !track.matched_local_track_id.startsWith("itunes:");
    if (hasLocalTrack) {
      if (isLiked) {
        await handleRemoveFeedback(track.matched_local_track_id!);
      } else {
        await handleLike(track.matched_local_track_id!);
      }
      return;
    }
    await handleLikeOnline({
      id: track.id,
      title: track.title,
      artist: track.artist,
      album: track.album,
      cover_art_url: track.cover_art_url,
      preview_url: track.preview_url,
      duration_secs: track.duration_secs,
    });
  };

  const handleCreatePlaylist = async (name: string, description?: string): Promise<string | null> => {
    if (!currentUser) {
      setIsAuthModalOpen(true);
      return null;
    }
    let targetName = name.trim();
    while (playlists.some((p) => p.is_smart_mix !== 1 && p.name === targetName)) {
      const prompted = window.prompt(
        `A playlist named "${targetName}" already exists.\nPlease enter a new name for this playlist:`,
        `${targetName} (1)`
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

  const handleToggleTrackInPlaylist = async (playlistId: string, isCurrentlyMember: boolean) => {
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
          return { ...prev, [trackId]: current.filter((id) => id !== playlistId) };
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
        const preservedQueue = Array.from(queuedTrackIds).filter((trackId) => !playlistTrackIds.has(trackId));
        const first = plTracks[0];
        const isOnline =
          first.format === "online" ||
          first.file_path?.startsWith("online://") ||
          first.id.startsWith("itunes:") ||
          first.id.startsWith("online:");

        if (isOnline) {
          if (typeof navigator !== "undefined" && !navigator.onLine) {
            addAppNotification(
              "warning",
              "No Internet Connection",
              `Cannot play "${first.title}". Connect to the internet to stream online songs.`
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
            if (nextTrack.format === "online" || nextTrack.file_path?.startsWith("online://")) {
              onlineQueueMetadataRef.current.set(nextTrack.id, {
                external_track_id: nextTrack.id,
                provider: "online",
                provider_id: nextTrack.id,
                title: nextTrack.title,
                artist: nextTrack.artist_name || "Unknown Artist",
                album: nextTrack.album_title,
                duration_secs: nextTrack.duration_secs,
                cover_art_url: nextTrack.cover_art_url,
                preview_url: nextTrack.preview_url,
                match_status: "NOT_FOUND",
                recommendation_reason: "From playlist",
                in_wishlist: false,
              });
            }
            await dispatchCommand({
              command: "EnqueueTrack",
              payload: { track_id: nextTrack.id, play_next: false },
            });
          }
        } else {
          handleStopOnlineAudio();
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
              onlineQueueMetadataRef.current.set(nextTrack.id, {
                external_track_id: nextTrack.id,
                provider: "online",
                provider_id: nextTrack.id,
                title: nextTrack.title,
                artist: nextTrack.artist_name || "Unknown Artist",
                album: nextTrack.album_title,
                duration_secs: nextTrack.duration_secs,
                cover_art_url: nextTrack.cover_art_url,
                preview_url: nextTrack.preview_url,
                match_status: "NOT_FOUND",
                recommendation_reason: "From playlist",
                in_wishlist: false,
              });
            }
            await dispatchCommand({
              command: "EnqueueTrack",
              payload: { track_id: nextTrack.id, play_next: false },
            });
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
            console.warn("Could not preserve queued track after playlist:", err);
          }
        }
      }
    } catch (err) {
      console.error("Failed to play playlist:", err);
    }
  };

  const handleFetchPlaylistTracks = async (playlistId: string): Promise<Track[]> => {
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
  const handleOpenCollection = useCallback(async (collection: CollectionData) => {
    setActiveCollection(collection);
    if (collection.playlistId || collection.type === "playlist") {
      setPlayingPlaylistId(collection.playlistId || collection.id);
    }
    if (collection.tracks && collection.tracks.length > 0) {
      return;
    }

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
                  provider: "online",
                  provider_id: t.id,
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
          prev && prev.id === collection.id ? { ...prev, tracks: items } : prev
        );
      } else if (collection.searchQuery) {
        const res = await executeQuery({
          query: "SearchOnlineMusic",
          payload: { query: collection.searchQuery, limit: 50 },
        });
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
          prev && prev.id === collection.id ? { ...prev, tracks: items } : prev
        );
      }
    } catch (err) {
      console.error("Failed to load collection tracks:", err);
    } finally {
      setIsLoadingCollectionTracks(false);
    }
  }, []);

  const handlePlayDiscoveredCollection = useCallback(async (collection: CollectionData) => {
    if (!collection.searchQuery) return;
    if (typeof navigator !== "undefined" && !navigator.onLine) {
      addAppNotification(
        "warning",
        "No Internet Connection",
        `Cannot load ${collection.title}. Connect to the internet to stream this collection.`
      );
      return;
    }

    try {
      const response = await executeQuery({
        query: "SearchOnlineMusic",
        payload: { query: collection.searchQuery, limit: 50 },
      });
      const results = ((response?.data as DiscoveryRecommendation[]) || []).filter(
        (track, index, all) =>
          all.findIndex((candidate) => candidate.external_track_id === track.external_track_id) === index
      );
      if (results.length === 0) {
        addAppNotification("warning", "No Songs Found", `${collection.title} did not return any playable songs.`);
        return;
      }

      const localIdFor = (track: DiscoveryRecommendation) => {
        const localId = track.matched_local_track_id;
        return localId && !localId.startsWith("online:") && !localId.startsWith("itunes:") ? localId : null;
      };
      const resolvedId = (track: DiscoveryRecommendation) => localIdFor(track) || track.external_track_id;
      const isLocal = (track: DiscoveryRecommendation) => localIdFor(track) !== null;
      const collectionIds = new Set(results.map(resolvedId));
      const preservedQueue = Array.from(queuedTrackIds).filter((trackId) => !collectionIds.has(trackId));
      const first = results[0];
      const firstId = resolvedId(first);

      setPlayingOrigin({
        type: "mix",
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
            matched_local_track_id: isLocal(track) ? resolvedId(track) : undefined,
            rawRecommendation: track,
          })),
        },
      });

      handleStopOnlineAudio();
      if (!isLocal(first)) {
        onlineQueueMetadataRef.current.set(firstId, first);
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
          if (!String(error?.message || error).toLowerCase().includes("already in the queue")) {
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
            onlineQueueMetadataRef.current.set(trackId, track);
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
        error?.message || `Could not play ${collection.title}.`
      );
    }
  }, [queuedTrackIds, addAppNotification, handleStopOnlineAudio, fetchPlaybackState]);

  const handleOpenPlayingOrigin = useCallback(() => {
    if (!playingOrigin) return;
    if (playingOrigin.collectionData) {
      setIsLyricsActive(false);
      handleOpenCollection(playingOrigin.collectionData);
    } else if (playingOrigin.type === "playlist" && playingOrigin.id) {
      const pl = playlists.find((p) => p.id === playingOrigin.id);
      if (pl) {
        setIsLyricsActive(false);
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
  }, [playingOrigin, playlists, handleOpenCollection]);

  const handlePlayCollectionTrack = useCallback(
    async (item: CollectionTrackItem) => {
      if (activeCollection) {
        setPlayingOrigin({
          type: activeCollection.type === "playlist" ? "playlist" : "mix",
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
        (hasRealLocalMatch &&
          playbackState.is_playing &&
          playbackState.current_track?.id === item.matched_local_track_id) ||
        (onlineTrack &&
          onlineTrack.isPlaying &&
          (onlineTrack.id === item.id || (item.matched_local_track_id && onlineTrack.id === item.matched_local_track_id)));

      if (isCurrentlyPlaying) {
        handleUnifiedPlayPause();
        return;
      }

      const enqueueCollectionRemainder = async () => {
        const collectionTracks = activeCollection?.tracks || [];
        const itemIdx = collectionTracks.findIndex((track) => track.id === item.id);
        const collectionIds = new Set(collectionTracks.map((track) => track.matched_local_track_id || track.id));
        for (let i = itemIdx + 1; i < collectionTracks.length; i++) {
          const queuedTrack = collectionTracks[i];
          const queuedId = queuedTrack.matched_local_track_id || queuedTrack.id;
          const isOnlineQueued =
            !queuedTrack.matched_local_track_id ||
            queuedTrack.matched_local_track_id.startsWith("online:") ||
            queuedTrack.matched_local_track_id.startsWith("itunes:");
          if (isOnlineQueued) {
            onlineQueueMetadataRef.current.set(queuedId, queuedTrack.rawRecommendation || {
              external_track_id: queuedId,
              provider: "online",
              provider_id: queuedId,
              title: queuedTrack.title,
              artist: queuedTrack.artist,
              album: queuedTrack.album,
              duration_secs: queuedTrack.duration_secs,
              cover_art_url: queuedTrack.cover_art_url,
              preview_url: queuedTrack.preview_url,
              match_status: "NOT_FOUND",
              recommendation_reason: "From collection",
              in_wishlist: false,
            });
          }
          try {
            await dispatchCommand({ command: "EnqueueTrack", payload: { track_id: queuedId, play_next: false } });
          } catch (err) {
            console.warn("Could not enqueue collection track:", err);
          }
        }
        for (const queuedId of queuedTrackIds) {
          if (collectionIds.has(queuedId)) continue;
          try {
            await dispatchCommand({ command: "EnqueueTrack", payload: { track_id: queuedId, play_next: false } });
          } catch (err) {
            console.warn("Could not preserve queued track after collection:", err);
          }
        }
      };

      if (hasRealLocalMatch) {
        handleStopOnlineAudio();
        await dispatchCommand({
          command: "PlayTrack",
          payload: { track_id: item.matched_local_track_id!, source: "collection" },
        });
        await enqueueCollectionRemainder();
        fetchPlaybackState();
        return;
      }

      // Offline check: Cannot stream online audio without an internet connection
      if (typeof navigator !== "undefined" && !navigator.onLine) {
        addAppNotification(
          "warning",
          "No Internet Connection",
          `Cannot play "${item.title}". Connect to the internet to stream online songs.`
        );
        return;
      }

      if (item.rawRecommendation) {
        await handlePlayOnlineTrack(item.rawRecommendation, false, "collection");
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
        matched_local_track_id: hasRealLocalMatch ? item.matched_local_track_id : undefined,
        recommendation_reason: "From collection",
        in_wishlist: false,
      };
      await handlePlayOnlineTrack(rec, false, "collection");
      await enqueueCollectionRemainder();
    },
    [playbackState.is_playing, playbackState.current_track, onlineTrack, activeCollection, queuedTrackIds, handleUnifiedPlayPause, handleStopOnlineAudio, handlePlayOnlineTrack, fetchPlaybackState, addAppNotification]
  );

  const isPlayingThisCollection = useMemo(() => {
    if (!activeCollection || !activeCollection.tracks || activeCollection.tracks.length === 0) {
      return false;
    }
    const currentId = playbackState.is_playing
      ? playbackState.current_track?.id
      : onlineTrack?.isPlaying
      ? onlineTrack?.id
      : null;
    if (!currentId) return false;

    return activeCollection.tracks.some(
      (t) => t.id === currentId || t.matched_local_track_id === currentId
    );
  }, [activeCollection, playbackState.is_playing, playbackState.current_track?.id, onlineTrack]);

  const isCollectionSaved = useMemo(() => {
    if (!activeCollection) return false;
    if (activeCollection.type === "playlist" || activeCollection.playlistId) return true;
    return playlists.some(
      (p) =>
        p.is_smart_mix !== 1 &&
        (p.id === activeCollection.id ||
          p.id === activeCollection.playlistId ||
          p.name === activeCollection.title.trim())
    );
  }, [activeCollection, playlists]);

  const handlePlayAllCollection = useCallback(async () => {
    if (!activeCollection || !activeCollection.tracks || activeCollection.tracks.length === 0) {
      return;
    }

    if (isPlayingThisCollection) {
      handleUnifiedPlayPause();
      return;
    }

    await handlePlayCollectionTrack(activeCollection.tracks[0]);
  }, [activeCollection, isPlayingThisCollection, handleUnifiedPlayPause, handlePlayCollectionTrack]);

  const handleShuffleCollection = useCallback(async () => {
    if (!activeCollection || !activeCollection.tracks || activeCollection.tracks.length === 0) {
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
    const localTracks = tracks.filter((t) =>
      !!t.matched_local_track_id &&
      !t.matched_local_track_id.startsWith("online:") &&
      !t.matched_local_track_id.startsWith("itunes:")
    );

    if (localTracks.length > 0) {
      handleStopOnlineAudio();
      // First enqueue tracks in their natural sequential order into the queue
      await dispatchCommand({ command: "ClearQueue" });
      await dispatchCommand({
        command: "PlayTrack",
        payload: { track_id: localTracks[0].matched_local_track_id!, source: "collection" },
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

  const handleInspectSpotifyPlaylist = async (urlOrId: string): Promise<SpotifyPlaylistImport | null> => {
    const res = await executeQuery({
      query: "ImportSpotifyPlaylist",
      payload: { url_or_id: urlOrId },
    });
    return (res.data as SpotifyPlaylistImport) || null;
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
      }>
    ) => {
      let targetName = name.trim();
      while (playlists.some((p) => p.is_smart_mix !== 1 && p.name.toLowerCase() === targetName.toLowerCase())) {
        const prompted = await requestPrompt(
          "Playlist Name Already Taken",
          `A playlist named "${targetName}" already exists in your library. Please enter a new name for this playlist:`,
          `${targetName} (1)`
        );
        if (prompted === null) {
          return;
        }
        const trimmed = prompted.trim();
        if (!trimmed) {
          addAppNotification("warning", "Name Required", "Playlist name cannot be empty.");
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
    [playlists, fetchPlaylists]
  );

  const handleSaveCollectionToPlaylists = useCallback(
    async (collection: CollectionData) => {
      try {
        let targetName = collection.title.trim();
        while (playlists.some((p) => p.is_smart_mix !== 1 && p.name.toLowerCase() === targetName.toLowerCase())) {
          const prompted = await requestPrompt(
            "Playlist Name Already Taken",
            `A playlist named "${targetName}" already exists in your library. Please enter a new name for this playlist:`,
            `${targetName} (1)`
          );
          if (prompted === null) {
            return;
          }
          const trimmed = prompted.trim();
          if (!trimmed) {
            addAppNotification("warning", "Name Required", "Playlist name cannot be empty.");
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
            : t.id
        );

        if (trackIds.length > 0) {
          await handleSaveImportedPlaylist(targetName, trackIds, allTracks);
        } else {
          await handleCreatePlaylist(targetName, collection.subtitle);
          addAppNotification("info", "Playlist Created", `Playlist "${targetName}" created.`);
        }
      } catch (err) {
        console.error("Failed to save collection to playlists:", err);
      }
    },
    [playlists, handleSaveImportedPlaylist, handleCreatePlaylist]
  );

  const handleAddMissingToWishlist = async (missingTracks: any[]) => {
    await dispatchCommand({
      command: "AddMissingToWishlist",
      payload: { tracks: missingTracks },
    });
    fetchWishlist();
  };

  // Wishlist
  const handleAddToWishlist = async (title: string, artist: string, album?: string) => {
    await dispatchCommand({
      command: "AddToWishlist",
      payload: { title, artist, album },
    });
    fetchWishlist();
  };

  // Soulseek & Downloads
  const handleLaunchSoulseek = async (query?: string, filter?: string) => {
    if (query) {
      try {
        await navigator.clipboard.writeText(query);
      } catch (e) {}
    }
    await dispatchCommand({
      command: "LaunchSoulseek",
      payload: { search_query: query, filter_query: filter },
    });
  };

  const handleImportSoulseekDownloads = async () => {
    setIsScanning(true);
    await dispatchCommand({
      command: "ImportSoulseekDownloads",
    });
    await fetchTracks();
    await fetchPlaylists();
    await fetchWishlist();
    setIsScanning(false);
  };

  const handleSearchSoulseek = async (
    artist: string,
    title: string,
    album?: string
  ): Promise<DownloadSearchResult[]> => {
    const res = await dispatchCommand({
      command: "SearchSoulseek",
      payload: { artist, title, album },
    });
    const results = (res as any)?.data || (res as any)?.results;
    if (Array.isArray(results)) {
      return results;
    }
    return [];
  };

  // Navigation shortcut to search & download directly via centered modal popup
  const handleInitiateDirectDownloadSearch = (artist: string, title: string, album?: string) => {
    setDownloadModalTrack({ artist, title, album });
  };

  const handleModalStartDownload = async (searchResultId: string, track: DownloadModalTrack) => {
    await dispatchCommand({
      command: "StartDownload",
      payload: {
        search_result_id: searchResultId,
      },
    });
    addAppNotification(
      "info",
      "Download Queued",
      `"${track.title}" by ${track.artist} has been queued for download.`
    );
    fetchDownloads();
  };

  const handleDirectAudioDownload = async (track: DownloadModalTrack) => {
    try {
      addAppNotification(
        "info",
        "Direct Download",
        `Locating audio stream for "${track.title}" by ${track.artist}...`
      );
      const searchResults = await handleSearchSoulseek(track.artist, track.title, track.album);
      const directResult =
        searchResults.find((r) => r.id.startsWith("ytdlp_flac_")) ||
        searchResults.find((r) => r.provider === "yt-dlp" || r.id.startsWith("ytdlp_")) ||
        searchResults[0];

      if (directResult) {
        await handleModalStartDownload(directResult.id, track);
      } else {
        addAppNotification(
          "error",
          "Download Error",
          `No streamable audio found for "${track.title}".`
        );
      }
    } catch (err: any) {
      console.warn("Direct download failed:", err);
      addAppNotification(
        "error",
        "Download Error",
        `Could not start direct stream download for "${track.title}".`
      );
    }
  };

  const handleGlobalOnlineSearch = async (queryOverride?: string) => {
    const q = (typeof queryOverride === "string" ? queryOverride : globalSearchQuery).trim();
    if (!q) {
      setGlobalSearchResults(null);
      return;
    }
    setGlobalSearchQuery(q);
    setIsGlobalSearching(true);
    setActiveCollection(null);
    setCurrentView("discovery");

    try {
      const res = await executeQuery({
        query: "SearchOnlineMusic",
        payload: { query: q, limit: 35 },
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
        if (titleLower.includes(qLower) || artistLower.includes(qLower) || albumLower.includes(qLower)) {
          return true;
        }

        // Token match: title or artist contains any significant token
        return qTokens.some((tok) => titleLower.includes(tok) || artistLower.includes(tok));
      });

      setGlobalSearchResults(relevant.length > 0 ? relevant : incoming);
    } catch (err) {
      console.error("Global online music search failed:", err);
      setGlobalSearchResults([]);
    } finally {
      setIsGlobalSearching(false);
    }
  };

  const handleGlobalClearSearch = () => {
    setGlobalSearchQuery("");
    setGlobalSearchResults(null);
  };

  const handleGlobalRefresh = async () => {
    setIsGlobalRefreshing(true);
    try {
      if (globalSearchQuery.trim()) {
        await handleGlobalOnlineSearch(globalSearchQuery.trim());
      }
      await Promise.allSettled([
        fetchDiscovery(true),
        fetchTracks(),
        fetchAlbums(),
      ]);
    } finally {
      setIsGlobalRefreshing(false);
    }
  };

  const unreadNotificationsCount = notifications.filter((n) => !n.read).length;

  const isPlayingOnline = !!onlineTrack && !playbackState.is_playing;
  const activePlayingTitle = isPlayingOnline
    ? onlineTrack.title
    : playbackState.current_track?.title || "No Track Selected";
  const activePlayingArtist = isPlayingOnline
    ? onlineTrack.artist
    : playbackState.current_track?.artist_name || "";
  const activePlayingTrackId = isPlayingOnline ? undefined : playbackState.current_track?.id;
  const activePlayingDuration = isPlayingOnline
    ? onlineTrack.duration
    : playbackState.duration_secs || playbackState.current_track?.duration_secs || 0;
  const activePlayingCurrentTime = isPlayingOnline
    ? onlineTrack.currentTime
    : playbackState.position_secs || 0;
  const activePlayingArtwork = useMemo(() => {
    if (isPlayingOnline && onlineTrack?.cover_art_url) {
      return onlineTrack.cover_art_url;
    }
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
      const matchInLibrary = tracks.find((t) => t.id === playbackState.current_track?.id);
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
          (t.title.toLowerCase().trim() === playbackState.current_track?.title.toLowerCase().trim() &&
            t.artist.toLowerCase().trim() === playbackState.current_track?.artist_name?.toLowerCase().trim())
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
          (r.title.toLowerCase().trim() === playbackState.current_track?.title.toLowerCase().trim() &&
            r.artist.toLowerCase().trim() === playbackState.current_track?.artist_name?.toLowerCase().trim())
      );
      if (matchInDiscovery?.cover_art_url) {
        return matchInDiscovery.cover_art_url;
      }
    }
    // 6. Track's album cover in albums list
    if (playbackState.current_track?.album_title) {
      const matchAlbum = albums.find(
        (a) =>
          a.title.toLowerCase().trim() === playbackState.current_track?.album_title?.toLowerCase().trim()
      );
      if (matchAlbum?.cover_art_path) {
        return matchAlbum.cover_art_path;
      }
    }
    return null;
  }, [
    isPlayingOnline,
    onlineTrack?.cover_art_url,
    currentTrackCoverUrl,
    playbackState.current_track,
    tracks,
    activeCollection?.tracks,
    discoveryRecs,
    albums,
  ]);

  const likedPlaylistId = playlists.find((playlist) => playlist.name === "Liked Songs")?.id;
  const likedTrackIds = useMemo(() => {
    if (!likedPlaylistId) return new Set<string>();
    return new Set(
      Object.entries(trackPlaylistMap)
        .filter(([, playlistIds]) => playlistIds.includes(likedPlaylistId))
        .map(([trackId]) => trackId)
    );
  }, [trackPlaylistMap, likedPlaylistId]);

  const regularPlaylistIds = useMemo(
    () => new Set(
      playlists
        .filter((playlist) => playlist.is_smart_mix !== 1 && playlist.name !== "Liked Songs")
        .map((playlist) => playlist.id)
    ),
    [playlists]
  );

  const regularTrackPlaylistMap = useMemo(() => {
    const filtered: Record<string, string[]> = {};
    for (const [trackId, playlistIds] of Object.entries(trackPlaylistMap)) {
      filtered[trackId] = playlistIds.filter((playlistId) => regularPlaylistIds.has(playlistId));
    }
    return filtered;
  }, [trackPlaylistMap, regularPlaylistIds]);

  const isCurrentTrackInPlaylist = useMemo(() => {
    if (isPlayingOnline && onlineTrack) {
      const id1 = onlineTrack.id;
      const id2 = `online:${onlineTrack.artist}-${onlineTrack.title}`;
      return (
        (id1 && regularTrackPlaylistMap[id1] && regularTrackPlaylistMap[id1].length > 0) ||
        (id2 && regularTrackPlaylistMap[id2] && regularTrackPlaylistMap[id2].length > 0) ||
        false
      );
    }
    if (playbackState.current_track) {
      const tid = playbackState.current_track.id;
      return (regularTrackPlaylistMap[tid] && regularTrackPlaylistMap[tid].length > 0) || false;
    }
    return false;
  }, [isPlayingOnline, onlineTrack, playbackState.current_track, regularTrackPlaylistMap]);

  return (
    <div className="app-container">
      <div className="app-body">
        {/* Left Sidebar Navigation */}
        <Sidebar
          currentView={currentView}
          onSelectView={(view) => {
            setActiveCollection(null);
            setIsLyricsActive(false);
            if (view === "stats") {
              setStatsRefreshTrigger((prev) => prev + 1);
            }
            setCurrentView(view);
          }}
          unreadNotificationsCount={unreadNotificationsCount}
          playlists={playlists}
          activePlaylistId={activeCollection?.type === "playlist" ? (activeCollection.playlistId || activeCollection.id) : null}
          onSelectPlaylist={(playlist) => {
            setCurrentView("playlists");
            setIsLyricsActive(false);
            handleOpenCollection({
              id: playlist.id,
              type: playlist.is_smart_mix === 1 ? "mix" : "playlist",
              title: playlist.name,
              subtitle: playlist.description || (playlist.is_smart_mix === 1 ? "Custom algorithmic smart mix" : "User Playlist"),
              tag: playlist.is_smart_mix === 1 ? "SMART MIX" : "PUBLIC PLAYLIST",
              playlistId: playlist.id,
              cover_art_url: playlist.cover_art_url,
            });
          }}
        />

        {/* Main Content Area */}
        <main className="main-content">
          {/* Persistent Global Top Search Bar (hidden on stats/profile view, lyrics active, or offline discovery) */}
          {((!activeCollection && currentView === "discovery" && !isOnline) || isLyricsActive || currentView === "stats") ? null : (
            <GlobalTopSearchBar
              searchQuery={globalSearchQuery}
              setSearchQuery={setGlobalSearchQuery}
              onSearch={handleGlobalOnlineSearch}
              onRefresh={handleGlobalRefresh}
              isSearching={isGlobalSearching}
              isRefreshing={isGlobalRefreshing}
            />
          )}

          {isLyricsActive ? (
            <div style={{ flex: 1, minHeight: 0, height: "100%", width: "100%", position: "relative" }}>
              <LyricsView
                trackId={activePlayingTrackId}
                artist={activePlayingArtist}
                title={activePlayingTitle}
                durationSecs={activePlayingDuration}
                currentTime={activePlayingCurrentTime}
                onSeek={handleUnifiedSeek}
                isFullScreen={false}
              />
            </div>
          ) : activeCollection ? (
            <CollectionDetailView
              collection={activeCollection}
              isLoadingTracks={isLoadingCollectionTracks}
              onBack={() => setActiveCollection(null)}
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
                  : onlineTrack?.id
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
              onEnqueueTrack={(track) => handleEnqueueRecommendation(track.rawRecommendation || {
                external_track_id: track.id,
                provider: track.matched_local_track_id ? "library" : "online",
                provider_id: track.id,
                title: track.title,
                artist: track.artist,
                album: track.album,
                duration_secs: track.duration_secs,
                cover_art_url: track.cover_art_url,
                preview_url: track.preview_url,
                matched_local_track_id: track.matched_local_track_id,
                match_status: track.matched_local_track_id ? "EXACT_MATCH" : "NOT_FOUND",
                recommendation_reason: "From collection",
                in_wishlist: false,
              })}
            />
          ) : (
            <>
              {currentView === "library" && (
                <LibraryView
                  tracks={tracks}
                  queuedTrackIds={queuedTrackIds}
                  onPlayTrack={handlePlayTrack}
                  onEnqueueTrack={handleEnqueueTrack}
                  onLikeTrack={handleLike}
                  onRemoveFeedback={handleRemoveFeedback}
                  onRescan={handleRescanLibrary}
                  onSearch={handleSearchLibrary}
                  trackPlaylistMap={regularTrackPlaylistMap}
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
                  onInspectSpotifyPlaylist={handleInspectSpotifyPlaylist}
                  onSaveImportedPlaylist={handleSaveImportedPlaylist}
                  onAddMissingToWishlist={handleAddMissingToWishlist}
                  onLaunchSoulseek={handleLaunchSoulseek}
                  onSearchDirect={handleInitiateDirectDownloadSearch}
                  onFetchPlaylistTracks={handleFetchPlaylistTracks}
                  onPlayTrack={handlePlayTrack}
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
                  onInspectSpotifyPlaylist={handleInspectSpotifyPlaylist}
                  onSaveImportedPlaylist={handleSaveImportedPlaylist}
                  onAddMissingToWishlist={handleAddMissingToWishlist}
                  onLaunchSoulseek={handleLaunchSoulseek}
                  onSearchDirect={handleInitiateDirectDownloadSearch}
                  onFetchPlaylistTracks={handleFetchPlaylistTracks}
                  onPlayTrack={handlePlayTrack}
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
                  playlists={playlists}
                  onPlayPlaylist={handlePlayPlaylist}
                  onAddToWishlist={(rec) =>
                    handleAddToWishlist(rec.title, rec.artist, rec.album)
                  }
                  onSearchDirect={(artist, title) =>
                    handleInitiateDirectDownloadSearch(artist, title)
                  }
                  onRefresh={fetchDiscovery}
                  onPlayOnlineTrack={handlePlayOnlineTrack}
                  onStopTrack={handleStopTrack}
                  activeOnlineTrackId={onlineTrack?.id || null}
                  isOnlinePlaying={!!onlineTrack?.isPlaying}
                  isOnlineLoading={!!onlineTrack?.isLoading}
                  currentLocalTrack={playbackState.current_track}
                  isLocalPlaying={playbackState.is_playing}
                  onOpenCollection={handleOpenCollection}
                  onPlayCollection={handlePlayDiscoveredCollection}
                  downloads={downloads}
                  searchQuery={globalSearchQuery}
                  setSearchQuery={setGlobalSearchQuery}
                  searchResults={globalSearchResults}
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
                  onGoToLibrary={() => setCurrentView("library")}
                  queuedTrackIds={queuedTrackIds}
                  onEnqueueTrack={handleEnqueueRecommendation}
                  likedTrackIds={likedTrackIds}
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
                  onLogout={handleLogout}
                  fetchStatsOverview={fetchStatsOverview}
                  refreshTrigger={statsRefreshTrigger}
                  onUpdateUser={(u) => setCurrentUser(u)}
                />
              )}

              {currentView === "settings" && (
                <SettingsView
                  settings={settings}
                  configuredFolders={onboardingStatus?.configured_folders || []}
                  cloudSyncStatus={cloudSyncStatus}
                  onAddFolder={handleAddFolder}
                  onRemoveFolder={handleRemoveFolder}
                  onRescanLibrary={handleRescanLibrary}
                  onRerunOnboarding={handleRerunOnboarding}
                  onLaunchSoulseek={handleLaunchSoulseek}
                  onImportSoulseek={handleImportSoulseekDownloads}
                  onSyncCloud={handleSyncCloud}
                  onSetCloudUrl={handleSetCloudUrl}
                  onLogoutAll={handleLogoutAll}
                  isScanning={isScanning}
                  isSyncingCloud={isSyncingCloud}
                />
              )}
            </>
          )}
        </main>
      </div>

      {/* Bottom Sticky Player Bar (Unified for local music & online streams) */}
      <NowPlayingBar
        playbackState={playbackState}
        currentTrack={playbackState.current_track}
        onlineTrack={onlineTrack}
        coverArtUrl={activePlayingArtwork}
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
        isOnlineLiked={!!onlineTrack && !!playlists.find((p) => p.name === "Liked Songs" && (trackPlaylistMap[onlineTrack.id] || []).includes(p.id))}
        onRemoveFeedback={handleRemoveFeedback}
        onDownloadOnlineTrack={handleInitiateDirectDownloadSearch}
        isInPlaylist={isCurrentTrackInPlaylist}
        onOpenAddToPlaylist={handleOpenAddToPlaylistModal}
        onOpenOrigin={
          playingOrigin?.type === "playlist" || playingOrigin?.type === "mix"
            ? handleOpenPlayingOrigin
            : undefined
        }
        originName={
          playingOrigin?.type === "playlist" || playingOrigin?.type === "mix"
            ? playingOrigin.name
            : undefined
        }
        isLyricsActive={isLyricsActive}
        onToggleLyrics={() => setIsLyricsActive((prev) => !prev)}
        isFullscreen={isFullscreen}
        onToggleFullscreen={() => handleToggleFullscreen()}
      />

      {/* Full Screen Player View (matching user uploaded images) */}
      {isFullscreen && (
        <FullScreenPlayerView
          playbackState={playbackState}
          currentTrack={playbackState.current_track}
          onlineTrack={onlineTrack}
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
          isOnlineLiked={!!onlineTrack && !!playlists.find((p) => p.name === "Liked Songs" && (trackPlaylistMap[onlineTrack.id] || []).includes(p.id))}
          onRemoveFeedback={handleRemoveFeedback}
          onDownloadOnlineTrack={handleInitiateDirectDownloadSearch}
          onOpenOrigin={
            playingOrigin?.type === "playlist" || playingOrigin?.type === "mix"
              ? () => {
                  handleToggleFullscreen(false);
                  handleOpenPlayingOrigin();
                }
              : undefined
          }
          originName={
            playingOrigin?.type === "playlist" || playingOrigin?.type === "mix"
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
        playlists={playlists.filter((p) => p.is_smart_mix === 0 && p.name !== "Liked Songs")}
        trackPlaylistIds={
          playlistModalTrack && regularTrackPlaylistMap[playlistModalTrack.id]
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

      {/* Center Modal Popup for Track Download */}
      <DownloadOptionsModal
        isOpen={!!downloadModalTrack}
        track={downloadModalTrack}
        onClose={() => setDownloadModalTrack(null)}
        onSearchSoulseek={handleSearchSoulseek}
        onStartDownload={handleModalStartDownload}
        onDirectAudioDownload={handleDirectAudioDownload}
      />

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
    </div>
  );
};
