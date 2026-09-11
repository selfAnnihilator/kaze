import React, { useState, useEffect, useCallback } from "react";
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
        setPlaybackState((prev) => ({ ...prev, ...res.data }));
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
        case "PlaybackPositionChanged":
          setPlaybackState((prev) => ({
            ...prev,
            position_secs: event.payload?.position_secs ?? prev.position_secs,
            duration_secs: event.payload?.duration_secs ?? prev.duration_secs,
          }));
          break;

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

        case "VolumeChanged":
          setPlaybackState((prev) => ({
            ...prev,
            volume: event.payload?.volume ?? prev.volume,
            is_muted: event.payload?.is_muted ?? prev.is_muted,
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
    await dispatchCommand({
      command: "LikeTrack",
      payload: { track_id: trackId },
    });
  };

  const handleDislike = async (trackId: string) => {
    await dispatchCommand({
      command: "DislikeTrack",
      payload: { track_id: trackId },
    });
  };

  const handleEnqueueTrack = async (trackId: string) => {
    await dispatchCommand({
      command: "EnqueueTrack",
      payload: { track_id: trackId, play_next: false },
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

  // Playlists & Smart Mixes
  const handleCreatePlaylist = async (name: string, description?: string) => {
    await dispatchCommand({
      command: "CreatePlaylist",
      payload: { name, description },
    });
    fetchPlaylists();
  };

  const handleGenerateSmartMix = async (mixType: string) => {
    await dispatchCommand({
      command: "GenerateSmartMix",
      payload: { mix_type: mixType },
    });
    fetchPlaylists();
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
  const handleInitiateSoulseekSearch = (artist: string, title: string, album?: string) => {
    setSoulseekSearch({ artist, title, album });
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
            onPlayTrack={handlePlayTrack}
            onEnqueueTrack={handleEnqueueTrack}
            onLikeTrack={handleLike}
            onDislikeTrack={handleDislike}
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
            onCreatePlaylist={handleCreatePlaylist}
            onGenerateSmartMix={handleGenerateSmartMix}
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
