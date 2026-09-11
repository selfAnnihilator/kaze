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

export const App: React.FC = () => {
  // Navigation
  const [currentView, setCurrentView] = useState<ViewType>("library");

  // Onboarding
  const [onboardingStatus, setOnboardingStatus] = useState<OnboardingStatus | null>(null);
  const [showOnboarding, setShowOnboarding] = useState(false);

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

  const fetchDiscovery = useCallback(async () => {
    try {
      const res = await executeQuery({
        query: "GetDiscoveryRecommendations",
        payload: { limit: 25 },
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
          break;

        case "LibraryScanFailed":
          setIsScanning(false);
          break;

        case "DownloadProgress":
          setDownloads((prev) =>
            prev.map((t) =>
              t.id === event.payload?.task_id
                ? {
                    ...t,
                    bytes_downloaded: event.payload.bytes_downloaded,
                    file_size: event.payload.file_size || t.file_size,
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

  // --- Actions & Commands ---

  const handlePlayTrack = async (trackId: string) => {
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

  const handleSaveImportedPlaylist = async (name: string, trackIds: string[]) => {
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
  };

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
  const handleLaunchSoulseek = async (query?: string) => {
    if (query) {
      try {
        await navigator.clipboard.writeText(query);
      } catch (e) {}
    }
    await dispatchCommand({
      command: "LaunchSoulseek",
      payload: { search_query: query },
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
    return (res as any)?.results || [];
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

  // Navigation shortcut to search Soulseek from other views
  const handleInitiateSoulseekSearch = async (artist: string, title: string, album?: string) => {
    const q = `${artist} ${title}`.trim();
    setSoulseekSearch({ artist, title, album });
    await handleLaunchSoulseek(q);
    setCurrentView("downloads");
  };

  return (
    <div className="app-container">
      <div className="app-body">
        {/* Left Sidebar Navigation */}
        <Sidebar currentView={currentView} onSelectView={setCurrentView} />

        {/* Main Content Area */}
        <main className="main-content">
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

        {(currentView === "playlists" || currentView === "smart_mixes") && (
          <PlaylistsView
            playlists={playlists}
            onSelectPlaylist={() => {}}
            onPlayPlaylist={handlePlayPlaylist}
            onCreatePlaylist={handleCreatePlaylist}
            onInspectSpotifyPlaylist={handleInspectSpotifyPlaylist}
            onSaveImportedPlaylist={handleSaveImportedPlaylist}
            onAddMissingToWishlist={handleAddMissingToWishlist}
            onLaunchSoulseek={handleLaunchSoulseek}
            onFetchPlaylistTracks={handleFetchPlaylistTracks}
            onPlayTrack={handlePlayTrack}
            queuedTrackIds={queuedTrackIds}
            onEnqueueTrack={handleEnqueueTrack}
            onDequeueTrack={handleDequeueTrack}
          />
        )}

        {currentView === "discovery" && (
          <DiscoveryView
            recommendations={discoveryRecs}
            onAddToWishlist={(rec) =>
              handleAddToWishlist(rec.title, rec.artist, rec.album)
            }
            onSearchSoulseek={(artist, title) =>
              handleInitiateSoulseekSearch(artist, title)
            }
          />
        )}

        {currentView === "wishlist" && (
          <WishlistView
            wishlist={wishlist}
            onAddToWishlist={handleAddToWishlist}
            onUpdateStatus={handleUpdateWishlistStatus}
            onSearchSoulseek={handleInitiateSoulseekSearch}
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
      </main>
      </div>

      {/* Bottom Sticky Player Bar */}
      <NowPlayingBar
        playbackState={playbackState}
        currentTrack={playbackState.current_track}
        onPlayPause={handlePlayPause}
        onNext={handleNextTrack}
        onPrevious={handlePreviousTrack}
        onSeek={handleSeek}
        onVolumeChange={handleVolumeChange}
        onToggleMute={handleToggleMute}
        onToggleRepeat={handleToggleRepeat}
        onToggleShuffle={handleToggleShuffle}
        onLike={handleLike}
        onDislike={handleDislike}
        onRemoveFeedback={handleRemoveFeedback}
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
