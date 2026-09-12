import React, { useState, useEffect, useCallback, useRef } from "react";
import {
  Track,
  Artist,
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
} from "./types";
import { dispatchCommand, executeQuery, subscribeBackendEvents } from "./services/api";
import { Sidebar, ViewType } from "./components/Sidebar";
import { NowPlayingBar } from "./components/NowPlayingBar";
import { OnboardingModal } from "./components/OnboardingModal";
import { LibraryView } from "./components/views/LibraryView";
import { ArtistsView } from "./components/views/ArtistsView";
import { AlbumsView } from "./components/views/AlbumsView";
import { PlaylistsView } from "./components/views/PlaylistsView";
import { DiscoveryView } from "./components/views/DiscoveryView";
import { WishlistView } from "./components/views/WishlistView";
import { DownloadsView } from "./components/views/DownloadsView";
import { SettingsView } from "./components/views/SettingsView";
import {
  CollectionDetailView,
  CollectionData,
  CollectionTrackItem,
} from "./components/views/CollectionDetailView";

export const App: React.FC = () => {
  // Navigation
  const [currentView, setCurrentView] = useState<ViewType>("library");
  const [activeCollection, setActiveCollection] = useState<CollectionData | null>(null);
  const [isLoadingCollectionTracks, setIsLoadingCollectionTracks] = useState<boolean>(false);

  // Onboarding
  const [onboardingStatus, setOnboardingStatus] = useState<OnboardingStatus | null>(null);
  const [showOnboarding, setShowOnboarding] = useState(false);

  // Online Preview / Stream Playback (Integrated into bottom NowPlayingBar)
  const [onlineTrack, setOnlineTrack] = useState<OnlinePlayingTrack | null>(null);
  const onlineAudioRef = useRef<HTMLAudioElement | null>(null);
  const activeOnlinePlayIdRef = useRef<number>(0);

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
  const [artists, setArtists] = useState<Artist[]>([]);
  const [albums, setAlbums] = useState<Album[]>([]);
  const [playlists, setPlaylists] = useState<Playlist[]>([]);
  const [wishlist, setWishlist] = useState<WishlistItem[]>([]);
  const [downloads, setDownloads] = useState<DownloadTask[]>([]);
  const [discoveryRecs, setDiscoveryRecs] = useState<DiscoveryRecommendation[]>([]);
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [isScanning, setIsScanning] = useState(false);

  // Cross-view state (e.g. initiating download search from discovery/wishlist)
  const [soulseekSearch, setSoulseekSearch] = useState<{
    artist: string;
    title: string;
    album?: string;
  } | null>(null);

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

  const fetchArtists = useCallback(async () => {
    try {
      const res = await executeQuery({
        query: "GetArtists",
        payload: { offset: 0, limit: 200 },
      });
      if (Array.isArray(res.data)) {
        setArtists(res.data);
      }
    } catch (err) {
      console.error("Failed to fetch artists:", err);
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
    fetchArtists();
    fetchAlbums();
    fetchPlaylists();
    fetchWishlist();
    fetchDownloads();
    fetchDiscovery();
    fetchSettings();
  }, [
    fetchOnboardingStatus,
    fetchPlaybackState,
    fetchTracks,
    fetchArtists,
    fetchAlbums,
    fetchPlaylists,
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
          fetchTracks();
          fetchArtists();
          fetchAlbums();
          fetchOnboardingStatus();
          fetchDiscovery();
          break;

        case "LibraryScanFailed":
          setIsScanning(false);
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

        case "DownloadCompleted":
        case "DownloadFailed":
          fetchDownloads();
          fetchWishlist();
          fetchTracks();
          fetchArtists();
          fetchAlbums();
          fetchDiscovery();
          break;

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
  }, [fetchTracks, fetchArtists, fetchAlbums, fetchOnboardingStatus, fetchDownloads, fetchWishlist]);

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
        handleStopOnlineAudio();
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
            handleStopOnlineAudio();
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
    [playbackState.is_playing, playbackState.current_track, playbackState.volume, playbackState.is_muted, onlineTrack, handleStopOnlineAudio, handlePlayTrack]
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
    await dispatchCommand({ command: "NextTrack" });
  };

  const handlePreviousTrack = async () => {
    await dispatchCommand({ command: "PreviousTrack" });
  };

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
  const handleCreatePlaylist = async (name: string, description?: string) => {
    await dispatchCommand({
      command: "CreatePlaylist",
      payload: { name, description },
    });
    fetchPlaylists();
  };

  const handlePlayPlaylist = async (playlistId: string) => {
    handleStopOnlineAudio();
    try {
      const res = await executeQuery({
        query: "GetPlaylistTracks",
        payload: { playlist_id: playlistId },
      });
      const plTracks: Track[] = (res.data as any) || [];
      if (plTracks.length > 0) {
        await dispatchCommand({ command: "ClearQueue" });
        await dispatchCommand({
          command: "PlayTrack",
          payload: { track_id: plTracks[0].id, source: "playlist" },
        });
        for (let i = 1; i < plTracks.length; i++) {
          await dispatchCommand({
            command: "EnqueueTrack",
            payload: { track_id: plTracks[i].id, play_next: false },
          });
        }
        fetchPlaybackState();
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
        const items: CollectionTrackItem[] = plTracks.map((t) => ({
          id: t.id,
          title: t.title,
          artist: t.artist_name || "Unknown Artist",
          album: t.album_title,
          duration_secs: t.duration_secs,
          is_downloaded: true,
          matched_local_track_id: t.id,
          rawLocalTrack: t,
        }));
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
        await handlePlayTrack(item.matched_local_track_id);
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
    [playbackState.is_playing, playbackState.current_track, onlineTrack, handleUnifiedPlayPause, handleStopOnlineAudio, handlePlayTrack, handlePlayOnlineTrack]
  );

  const handlePlayAllCollection = useCallback(async () => {
    if (!activeCollection || !activeCollection.tracks || activeCollection.tracks.length === 0) {
      return;
    }

    const isCurrentlyPlaying =
      playbackState.is_playing || (!!onlineTrack && onlineTrack.isPlaying);
    if (isCurrentlyPlaying) {
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
      handlePlayCollectionTrack(first);
    }
  }, [activeCollection, playbackState.is_playing, onlineTrack, handleUnifiedPlayPause, handleStopOnlineAudio, handlePlayCollectionTrack, fetchPlaybackState]);

  const handleShuffleCollection = useCallback(async () => {
    if (!activeCollection || !activeCollection.tracks || activeCollection.tracks.length === 0) {
      return;
    }
    const shuffled = [...activeCollection.tracks].sort(() => Math.random() - 0.5);
    const first = shuffled[0];
    if (first.matched_local_track_id) {
      handleStopOnlineAudio();
      await dispatchCommand({ command: "ClearQueue" });
      await dispatchCommand({
        command: "PlayTrack",
        payload: { track_id: first.matched_local_track_id, source: "collection_shuffle" },
      });
      for (let i = 1; i < shuffled.length; i++) {
        const tid = shuffled[i].matched_local_track_id;
        if (tid) {
          await dispatchCommand({
            command: "EnqueueTrack",
            payload: { track_id: tid, play_next: false },
          });
        }
      }
      fetchPlaybackState();
    } else {
      handlePlayCollectionTrack(first);
    }
  }, [activeCollection, handleStopOnlineAudio, handlePlayCollectionTrack, fetchPlaybackState]);

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
    async (name: string, trackIds: string[]) => {
      const plRes = await dispatchCommand({
        command: "CreatePlaylist",
        payload: { name, description: "Imported from Spotify" },
      });
      const playlistId = (plRes as any)?.data;
      if (playlistId) {
        for (const tid of trackIds) {
          await dispatchCommand({
            command: "AddTrackToPlaylist",
            payload: { playlist_id: playlistId, track_id: tid },
          });
        }
        fetchPlaylists();
      }
    },
    [fetchPlaylists]
  );

  const handleSaveCollectionToPlaylists = useCallback(
    async (collection: CollectionData) => {
      try {
        const localTrackIds = (collection.tracks || [])
          .filter((t) => t.matched_local_track_id)
          .map((t) => t.matched_local_track_id as string);

        if (localTrackIds.length > 0) {
          await handleSaveImportedPlaylist(collection.title, localTrackIds);
        } else {
          await handleCreatePlaylist(collection.title, collection.subtitle);
          alert(`Playlist "${collection.title}" created. Tracks can be added as you download them.`);
        }
      } catch (err) {
        console.error("Failed to save collection to playlists:", err);
      }
    },
    [handleSaveImportedPlaylist, handleCreatePlaylist]
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

  const handleStartDownload = async (searchResultId: string, wishlistId?: string) => {
    await dispatchCommand({
      command: "StartDownload",
      payload: { search_result_id: searchResultId, wishlist_id: wishlistId },
    });
    fetchDownloads();
  };

  const handleCancelDownload = async (taskId: string) => {
    await dispatchCommand({
      command: "CancelDownload",
      payload: { task_id: taskId },
    });
    fetchDownloads();
  };

  // Navigation shortcut to search & download directly in-app from other views
  const handleInitiateDirectDownloadSearch = (artist: string, title: string, album?: string) => {
    setSoulseekSearch({ artist, title, album });
    setCurrentView("downloads");
  };

  const activeDownloadsCount = downloads.filter(
    (d) => d.status === "DOWNLOADING" || d.status === "QUEUED"
  ).length;

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
          activeDownloadsCount={activeDownloadsCount}
        />

        {/* Main Content Area */}
        <main className="main-content">
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
            isPlaying={
              playbackState.is_playing || (!!onlineTrack && onlineTrack.isPlaying)
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
              />
            )}

            {currentView === "artists" && (
              <ArtistsView
                artists={artists}
                onSelectArtist={(artistId) => {
                  const artist = artists.find((a) => a.id === artistId);
                  if (artist) {
                    handleSearchLibrary(artist.name);
                    setCurrentView("library");
                  }
                }}
              />
            )}

            {currentView === "albums" && (
              <AlbumsView
                albums={albums}
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

            {currentView === "downloads" && (
              <DownloadsView
                downloads={downloads}
                onSearchSoulseek={handleSearchSoulseek}
                onStartDownload={handleStartDownload}
                onCancelDownload={handleCancelDownload}
                onRefreshDownloads={fetchDownloads}
                onLaunchSoulseek={handleLaunchSoulseek}
                onImportSoulseek={handleImportSoulseekDownloads}
                initialSearch={soulseekSearch}
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
