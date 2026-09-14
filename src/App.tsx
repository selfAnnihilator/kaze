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
} from "./types";
import { dispatchCommand, executeQuery, subscribeBackendEvents } from "./services/api";
import { Sidebar, ViewType } from "./components/Sidebar";
import { NowPlayingBar } from "./components/NowPlayingBar";
import { OnboardingModal } from "./components/OnboardingModal";
import { LibraryView } from "./components/views/LibraryView";
import { AlbumsView } from "./components/views/AlbumsView";
import { PlaylistsView } from "./components/views/PlaylistsView";
import { DiscoveryView } from "./components/views/DiscoveryView";
import { WishlistView } from "./components/views/WishlistView";
import { NotificationsView } from "./components/views/NotificationsView";
import { SettingsView } from "./components/views/SettingsView";
import { GlobalTopSearchBar } from "./components/layout/GlobalTopSearchBar";
import { ToastContainer } from "./components/notifications/ToastContainer";
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
  const handlePlayCollectionTrackRef = useRef<(track: CollectionTrackItem) => void>(() => {});

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
  const [wishlist, setWishlist] = useState<WishlistItem[]>([]);
  const [downloads, setDownloads] = useState<DownloadTask[]>([]);
  const [discoveryRecs, setDiscoveryRecs] = useState<DiscoveryRecommendation[]>([]);
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [isScanning, setIsScanning] = useState(false);

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
        payload: { limit: 25, force_refresh: forceRefresh },
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
  ]);

  // Backend Event Subscriptions
  useEffect(() => {
    let unlistenFn: (() => void) | null = null;

    subscribeBackendEvents((event: any) => {
      if (!event || !event.event) return;
      console.log("[Backend Event]", event.event, event.payload);

      switch (event.event) {
        case "PlaybackStarted": {
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

        default:
          break;
      }
    }).then((fn) => {
      unlistenFn = fn;
    });

    return () => {
      if (unlistenFn) unlistenFn();
    };
  }, [fetchTracks, fetchAlbums, fetchOnboardingStatus, fetchDownloads, fetchWishlist, addAppNotification]);

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
      onlineAudioRef.current.pause();
      onlineAudioRef.current.removeAttribute("src");
      onlineAudioRef.current.load();
      onlineAudioRef.current = null;
    }
    setOnlineTrack(null);
  }, []);

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
      // Immediately fetch updated track/state
      fetchPlaybackState();
    },
    [tracks, handleStopOnlineAudio, fetchPlaybackState]
  );

  const handlePlayOnlineTrack = useCallback(
    async (rec: DiscoveryRecommendation) => {
      // 1. Check if this track is currently playing locally through Rodio
      const isCurrentlyPlayingLocal =
        playbackState.is_playing &&
        playbackState.current_track &&
        ((rec.matched_local_track_id && playbackState.current_track.id === rec.matched_local_track_id) ||
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
      if (rec.matched_local_track_id) {
        handleStopOnlineAudio();
        await handlePlayTrack(rec.matched_local_track_id);
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
        cover_art_url: rec.cover_art_url,
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
    if (onlineTrack && activeCollection && activeCollection.tracks && activeCollection.tracks.length > 0) {
      const currIdx = activeCollection.tracks.findIndex(
        (t) => t.id === onlineTrack.id || (t.matched_local_track_id && t.matched_local_track_id === onlineTrack.id)
      );
      if (currIdx !== -1) {
        let nextIdx = (currIdx + 1) % activeCollection.tracks.length;
        if (playbackState.is_shuffled && activeCollection.tracks.length > 1) {
          do {
            nextIdx = Math.floor(Math.random() * activeCollection.tracks.length);
          } while (nextIdx === currIdx && activeCollection.tracks.length > 1);
        }
        handlePlayCollectionTrackRef.current(activeCollection.tracks[nextIdx]);
        return;
      }
    }
    await dispatchCommand({ command: "NextTrack" });
  };

  const handlePreviousTrack = async () => {
    if (onlineTrack && activeCollection && activeCollection.tracks && activeCollection.tracks.length > 0) {
      const currIdx = activeCollection.tracks.findIndex(
        (t) => t.id === onlineTrack.id || (t.matched_local_track_id && t.matched_local_track_id === onlineTrack.id)
      );
      if (currIdx !== -1) {
        let prevIdx = (currIdx - 1 + activeCollection.tracks.length) % activeCollection.tracks.length;
        if (playbackState.is_shuffled && activeCollection.tracks.length > 1) {
          do {
            prevIdx = Math.floor(Math.random() * activeCollection.tracks.length);
          } while (prevIdx === currIdx && activeCollection.tracks.length > 1);
        }
        handlePlayCollectionTrackRef.current(activeCollection.tracks[prevIdx]);
        return;
      }
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
    const nextMode =
      playbackState.repeat_mode === "off"
        ? "all"
        : playbackState.repeat_mode === "all"
        ? "one"
        : "off";

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
  };

  const handleDislike = async (trackId: string) => {
    setTracks((prev) =>
      prev.map((t) => (t.id === trackId ? { ...t, manual_like: -1 } : t))
    );
    setPlaybackState((prev) =>
      prev.current_track?.id === trackId
        ? { ...prev, current_track: { ...prev.current_track, manual_like: -1 } }
        : prev
    );
    await dispatchCommand({
      command: "DislikeTrack",
      payload: { track_id: trackId },
    });
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
  };

  const handleEnqueueTrack = async (trackId: string) => {
    setQueuedTrackIds((prev) => new Set(prev).add(trackId));
    await dispatchCommand({
      command: "EnqueueTrack",
      payload: { track_id: trackId, play_next: false },
    });
  };

  const handleDequeueTrack = async (trackId: string) => {
    setQueuedTrackIds((prev) => {
      const next = new Set(prev);
      next.delete(trackId);
      return next;
    });
    await dispatchCommand({
      command: "DequeueTrack",
      payload: { track_id: trackId },
    });
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
  const handleCreatePlaylist = async (name: string, description?: string): Promise<string | null> => {
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
    try {
      const res = await executeQuery({
        query: "GetPlaylistTracks",
        payload: { playlist_id: playlistId },
      });
      const plTracks: Track[] = (res.data as any) || [];
      if (plTracks.length > 0) {
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
          await handlePlayOnlineTrack(rec);
        } else {
          handleStopOnlineAudio();
          await dispatchCommand({ command: "ClearQueue" });
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
            if (!nextIsOnline) {
              await dispatchCommand({
                command: "EnqueueTrack",
                payload: { track_id: nextTrack.id, play_next: false },
              });
            }
          }
          fetchPlaybackState();
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
        const items: CollectionTrackItem[] = onlineRecs.map((r) => ({
          id: r.external_track_id,
          title: r.title,
          artist: r.artist,
          album: r.album,
          duration_secs: r.duration_secs || 210,
          cover_art_url: r.cover_art_url,
          preview_url: r.preview_url,
          is_downloaded: !!r.matched_local_track_id,
          matched_local_track_id: r.matched_local_track_id,
          rawRecommendation: r,
        }));
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

  const handlePlayCollectionTrack = useCallback(
    async (item: CollectionTrackItem) => {
      const isCurrentlyPlaying =
        (item.matched_local_track_id &&
          playbackState.is_playing &&
          playbackState.current_track?.id === item.matched_local_track_id) ||
        (onlineTrack &&
          onlineTrack.isPlaying &&
          (onlineTrack.id === item.id || (item.matched_local_track_id && onlineTrack.id === item.matched_local_track_id)));

      if (isCurrentlyPlaying) {
        handleUnifiedPlayPause();
        return;
      }

      if (item.matched_local_track_id) {
        handleStopOnlineAudio();
        await dispatchCommand({ command: "ClearQueue" });
        await dispatchCommand({
          command: "PlayTrack",
          payload: { track_id: item.matched_local_track_id, source: "collection" },
        });
        if (activeCollection && activeCollection.tracks) {
          const itemIdx = activeCollection.tracks.findIndex((t) => t.id === item.id);
          for (let i = itemIdx + 1; i < activeCollection.tracks.length; i++) {
            const tid = activeCollection.tracks[i].matched_local_track_id;
            if (tid) {
              await dispatchCommand({
                command: "EnqueueTrack",
                payload: { track_id: tid, play_next: false },
              });
            }
          }
          for (let i = 0; i < itemIdx; i++) {
            const tid = activeCollection.tracks[i].matched_local_track_id;
            if (tid) {
              await dispatchCommand({
                command: "EnqueueTrack",
                payload: { track_id: tid, play_next: false },
              });
            }
          }
        }
        fetchPlaybackState();
        return;
      }

      if (typeof navigator !== "undefined" && !navigator.onLine) {
        addAppNotification(
          "warning",
          "No Internet Connection",
          `Cannot play "${item.title}". Connect to the internet to stream online songs.`
        );
        return;
      }

      if (item.rawRecommendation) {
        await handlePlayOnlineTrack(item.rawRecommendation);
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
        match_status: item.matched_local_track_id ? "EXACT_MATCH" : "NOT_FOUND",
        matched_local_track_id: item.matched_local_track_id,
        recommendation_reason: "From collection",
        in_wishlist: false,
      };
      await handlePlayOnlineTrack(rec);
    },
    [playbackState.is_playing, playbackState.current_track, onlineTrack, activeCollection, handleUnifiedPlayPause, handleStopOnlineAudio, handlePlayOnlineTrack, fetchPlaybackState, addAppNotification]
  );

  handlePlayCollectionTrackRef.current = handlePlayCollectionTrack;

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

    const tracks = activeCollection.tracks;
    const first = tracks[0];
    if (first.matched_local_track_id) {
      handleStopOnlineAudio();
      await dispatchCommand({ command: "ClearQueue" });
      await dispatchCommand({
        command: "PlayTrack",
        payload: { track_id: first.matched_local_track_id, source: "collection" },
      });
      for (let i = 1; i < tracks.length; i++) {
        const tid = tracks[i].matched_local_track_id;
        if (tid) {
          await dispatchCommand({
            command: "EnqueueTrack",
            payload: { track_id: tid, play_next: false },
          });
        }
      }
      fetchPlaybackState();
    } else {
      if (typeof navigator !== "undefined" && !navigator.onLine) {
        addAppNotification(
          "warning",
          "No Internet Connection",
          `Cannot play "${first.title}". Connect to the internet to stream online songs.`
        );
        return;
      }
      handlePlayCollectionTrack(first);
    }
  }, [activeCollection, isPlayingThisCollection, handleUnifiedPlayPause, handleStopOnlineAudio, handlePlayCollectionTrack, fetchPlaybackState, addAppNotification]);

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
    const localTracks = tracks.filter((t) => !!t.matched_local_track_id);

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
        if (tid) {
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
    try {
      const res = await executeQuery({
        query: "ImportSpotifyPlaylist",
        payload: { url_or_id: urlOrId },
      });
      return (res.data as SpotifyPlaylistImport) || null;
    } catch (err) {
      console.error("Failed to inspect Spotify playlist:", err);
      return null;
    }
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
      while (playlists.some((p) => p.is_smart_mix !== 1 && p.name === targetName)) {
        const prompted = window.prompt(
          `A playlist named "${targetName}" already exists.\nPlease enter a new name for this playlist:`,
          `${targetName} (1)`
        );
        if (prompted === null) {
          return;
        }
        const trimmed = prompted.trim();
        if (!trimmed) {
          alert("Playlist name cannot be empty.");
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
        while (playlists.some((p) => p.is_smart_mix !== 1 && p.name === targetName)) {
          const prompted = window.prompt(
            `A playlist named "${targetName}" already exists.\nPlease enter a new name for this playlist:`,
            `${targetName} (1)`
          );
          if (prompted === null) {
            return;
          }
          const trimmed = prompted.trim();
          if (!trimmed) {
            alert("Playlist name cannot be empty.");
            continue;
          }
          targetName = trimmed;
        }

        const allTracks = collection.tracks || [];
        const trackIds = allTracks.map((t) => t.matched_local_track_id || t.id);

        if (trackIds.length > 0) {
          await handleSaveImportedPlaylist(targetName, trackIds, allTracks);
        } else {
          await handleCreatePlaylist(targetName, collection.subtitle);
          alert(`Playlist "${targetName}" created.`);
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

  const handleUpdateWishlistStatus = async (
    wishlistId: string,
    status: "want" | "ignore" | "already_own" | "downloaded"
  ) => {
    await dispatchCommand({
      command: "UpdateWishlistStatus",
      payload: { wishlist_id: wishlistId, status },
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
      await handleAddToWishlist(track.title, track.artist, track.album);
      addAppNotification(
        "info",
        "Saved to Wishlist",
        `"${track.title}" by ${track.artist} added to wishlist for automated background download.`
      );
      fetchWishlist();
    } catch (err: any) {
      console.warn("Direct download fallback to wishlist:", err);
      addAppNotification(
        "error",
        "Wishlist Error",
        `Could not queue "${track.title}" to wishlist.`
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
      const qLower = q.toLowerCase();
      const seenKeys = new Set(
        incoming.map((r) => `${r.artist.toLowerCase().trim()}:${r.title.toLowerCase().trim()}`)
      );
      for (const rec of discoveryRecs) {
        const key = `${rec.artist.toLowerCase().trim()}:${rec.title.toLowerCase().trim()}`;
        if (!seenKeys.has(key)) {
          if (
            rec.title.toLowerCase().includes(qLower) ||
            rec.artist.toLowerCase().includes(qLower) ||
            (rec.album && rec.album.toLowerCase().includes(qLower)) ||
            (rec.genre && rec.genre.toLowerCase().includes(qLower))
          ) {
            incoming.push(rec);
            seenKeys.add(key);
          }
        }
      }
      setGlobalSearchResults(incoming);
    } catch (err) {
      console.error("Global online music search failed:", err);
      const qLower = q.toLowerCase();
      const fallbackMatches = discoveryRecs.filter(
        (rec) =>
          rec.title.toLowerCase().includes(qLower) ||
          rec.artist.toLowerCase().includes(qLower) ||
          (rec.album && rec.album.toLowerCase().includes(qLower))
      );
      setGlobalSearchResults(fallbackMatches);
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

  return (
    <div className="app-container">
      <div className="app-body">
        {/* Left Sidebar Navigation */}
        <Sidebar
          currentView={currentView}
          onSelectView={(view) => {
            setActiveCollection(null);
            setCurrentView(view);
          }}
          unreadNotificationsCount={unreadNotificationsCount}
        />

        {/* Main Content Area */}
        <main className="main-content">
          {/* Persistent Global Top Search Bar (hidden on discovery view when offline) */}
          {(!activeCollection && currentView === "discovery" && !isOnline) ? null : (
            <GlobalTopSearchBar
              searchQuery={globalSearchQuery}
              setSearchQuery={setGlobalSearchQuery}
              onSearch={handleGlobalOnlineSearch}
              onRefresh={handleGlobalRefresh}
              isSearching={isGlobalSearching}
              isRefreshing={isGlobalRefreshing}
            />
          )}

          {activeCollection ? (
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
              trackPlaylistMap={trackPlaylistMap}
              onAddToPlaylist={(track) =>
                handleOpenAddToPlaylistModal({
                  id: track.matched_local_track_id || track.id,
                  title: track.title,
                  artist: track.artist,
                  album: track.album,
                  cover_art_url: track.cover_art_url,
                })
              }
            />
          ) : (
            <>
              {currentView === "library" && (
                <LibraryView
                  tracks={tracks}
                  queuedTrackIds={queuedTrackIds}
                  onPlayTrack={handlePlayTrack}
                  onEnqueueTrack={handleEnqueueTrack}
                  onDequeueTrack={handleDequeueTrack}
                  onLikeTrack={handleLike}
                  onDislikeTrack={handleDislike}
                  onRemoveFeedback={handleRemoveFeedback}
                  onRescan={handleRescanLibrary}
                  onSearch={handleSearchLibrary}
                  trackPlaylistMap={trackPlaylistMap}
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
                  tracks={tracks}
                  onSelectAlbum={(albumId) => {
                    const album = albums.find((al) => al.id === albumId);
                    if (album) {
                      handleSearchLibrary(album.title);
                      setCurrentView("library");
                    }
                  }}
                />
              )}

              {currentView === "playlists" && (
                <PlaylistsView
                  viewMode="playlists"
                  playlists={playlists}
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
                  onDequeueTrack={handleDequeueTrack}
                  onOpenCollection={handleOpenCollection}
                />
              )}

              {currentView === "smart_mixes" && (
                <PlaylistsView
                  viewMode="smart_mixes"
                  playlists={playlists}
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
                  onDequeueTrack={handleDequeueTrack}
                  onOpenCollection={handleOpenCollection}
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
                  downloads={downloads}
                  searchQuery={globalSearchQuery}
                  setSearchQuery={setGlobalSearchQuery}
                  searchResults={globalSearchResults}
                  setSearchResults={setGlobalSearchResults}
                  isSearchingOnline={isGlobalSearching}
                  onSearchOnline={handleGlobalOnlineSearch}
                  onClearSearch={handleGlobalClearSearch}
                  trackPlaylistMap={trackPlaylistMap}
                  onAddToPlaylist={(rec) =>
                    handleOpenAddToPlaylistModal({
                      id: rec.matched_local_track_id || rec.external_track_id,
                      title: rec.title,
                      artist: rec.artist,
                      album: rec.album,
                      cover_art_url: rec.cover_art_url,
                    })
                  }
                  onGoToLibrary={() => setCurrentView("library")}
                />
              )}

              {currentView === "wishlist" && (
                <WishlistView
                  wishlist={wishlist}
                  onAddToWishlist={handleAddToWishlist}
                  onUpdateStatus={handleUpdateWishlistStatus}
                  onSearchDirect={handleInitiateDirectDownloadSearch}
                  onLaunchSoulseek={handleLaunchSoulseek}
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

              {currentView === "settings" && (
                <SettingsView
                  settings={settings}
                  configuredFolders={onboardingStatus?.configured_folders || []}
                  onAddFolder={handleAddFolder}
                  onRemoveFolder={handleRemoveFolder}
                  onRescanLibrary={handleRescanLibrary}
                  onRerunOnboarding={handleRerunOnboarding}
                  onLaunchSoulseek={handleLaunchSoulseek}
                  onImportSoulseek={handleImportSoulseekDownloads}
                  isScanning={isScanning}
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
        onPlayPause={handleUnifiedPlayPause}
        onNext={handleNextTrack}
        onPrevious={handlePreviousTrack}
        onSeek={handleUnifiedSeek}
        onVolumeChange={handleVolumeChange}
        onToggleMute={handleToggleMute}
        onToggleRepeat={handleToggleRepeat}
        onToggleShuffle={handleToggleShuffle}
        onLike={handleLike}
        onDislike={handleDislike}
        onRemoveFeedback={handleRemoveFeedback}
        onDownloadOnlineTrack={handleInitiateDirectDownloadSearch}
      />

      {/* Top-Right Floating Toast Notifications */}
      <ToastContainer toasts={toasts} onDismiss={dismissToast} />

      {/* Center Modal Popup for Playlist Management */}
      <AddToPlaylistModal
        isOpen={isPlaylistModalOpen}
        onClose={() => {
          setIsPlaylistModalOpen(false);
          setPlaylistModalTrack(null);
        }}
        track={playlistModalTrack}
        playlists={playlists.filter((p) => p.is_smart_mix === 0)}
        trackPlaylistIds={
          playlistModalTrack && trackPlaylistMap[playlistModalTrack.id]
            ? trackPlaylistMap[playlistModalTrack.id]
            : []
        }
        onTogglePlaylist={handleToggleTrackInPlaylist}
        onCreatePlaylist={handleCreatePlaylist}
      />

      {/* Center Modal Popup for Track Download */}
      <DownloadOptionsModal
        isOpen={!!downloadModalTrack}
        track={downloadModalTrack}
        onClose={() => setDownloadModalTrack(null)}
        onSearchSoulseek={handleSearchSoulseek}
        onStartDownload={handleModalStartDownload}
        onDirectAudioDownload={handleDirectAudioDownload}
        onAddToWishlist={(title, artist, album) => handleAddToWishlist(title, artist, album)}
      />

      {/* Onboarding Modal */}
      {showOnboarding && onboardingStatus && (
        <OnboardingModal
          defaultMusicDir={onboardingStatus.default_music_dir}
          onComplete={handleCompleteOnboarding}
        />
      )}
    </div>
  );
};
