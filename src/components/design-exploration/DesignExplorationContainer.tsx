import React, { useState, useMemo, useEffect } from "react";
import {
  DesignModelId,
  AppViewId,
  ExplorationTrack,
  ExplorationPlaylist,
  ExplorationAlbum,
  ExplorationAppContext,
} from "./types";
import {
  MOCK_TRACKS,
  MOCK_PLAYLISTS,
  MOCK_ALBUMS,
} from "./mockData";
import { ModelAApp as DriftApp } from "./models/ModelA/ModelAApp";
import { ModelBApp as BroadsheetApp } from "./models/ModelB/ModelBApp";
import { ModelCApp as GalleryApp } from "./models/ModelC/ModelCApp";
import { DesignSwitcher } from "./DesignSwitcher";
import { Track, Playlist, PlaybackState, HomeOverview } from "../../types";

interface DesignExplorationContainerProps {
  model: DesignModelId;
  onSelectModel: (model: DesignModelId) => void;
  realTracks?: Track[];
  playlists?: Playlist[];
  playbackState?: PlaybackState;
  homeOverview?: HomeOverview | null;
  onPlayRealTrack?: (trackId: string) => void;
  onTogglePlayPause?: () => void;
  onNextTrack?: () => void;
  onPrevTrack?: () => void;
  onSeek?: (seconds: number) => void;
  onVolumeChange?: (vol: number) => void;
  onLike?: (trackId: string) => void;
  searchQuery?: string;
  setSearchQuery?: (q: string) => void;
}

export const DesignExplorationContainer: React.FC<DesignExplorationContainerProps> = ({
  model,
  onSelectModel,
  realTracks = [],
  playlists: realPlaylists = [],
  playbackState,
  onPlayRealTrack,
  onTogglePlayPause,
  onNextTrack,
  onPrevTrack,
  onSeek,
  onVolumeChange,
  onLike,
  searchQuery: externalSearchQuery,
  setSearchQuery: setExternalSearchQuery,
}) => {
  const [currentView, setCurrentView] = useState<AppViewId>("home");
  const [localSearchQuery, setLocalSearchQuery] = useState("");
  const [internalPlaying, setInternalPlaying] = useState(true);
  const [internalPos, setInternalPos] = useState(48);
  const [internalTrackIndex, setInternalTrackIndex] = useState(0);
  const [isLikedMap, setIsLikedMap] = useState<Record<string, boolean>>({});

  const searchQuery = externalSearchQuery !== undefined ? externalSearchQuery : localSearchQuery;
  const setSearchQuery = setExternalSearchQuery || setLocalSearchQuery;

  // Convert real library tracks or use rich mock tracks
  const combinedTracks: ExplorationTrack[] = useMemo(() => {
    if (realTracks.length >= 3) {
      return realTracks.slice(0, 24).map((t, idx) => ({
        id: t.id,
        title: t.title,
        artist: t.artist_name || "Unknown Artist",
        album: t.album_title || "Unknown Album",
        year: t.year || 2026,
        durationSecs: t.duration_secs || 210,
        genre: t.genre_name || "Music",
        bpm: 100 + (idx * 7) % 40,
        key: ["C min", "F# min", "A min", "Eb maj", "G min"][idx % 5],
        coverArt: t.cover_art_url || MOCK_TRACKS[idx % MOCK_TRACKS.length].coverArt,
        accentColor: MOCK_TRACKS[idx % MOCK_TRACKS.length].accentColor,
        gradient: MOCK_TRACKS[idx % MOCK_TRACKS.length].gradient,
        lyricsSnippet: MOCK_TRACKS[idx % MOCK_TRACKS.length].lyricsSnippet,
        linerNotes: `Lossless master: ${t.format.toUpperCase()} format. Sample rate: ${t.sample_rate || 44100}Hz.`,
        isLiked: t.manual_like === 1 || !!isLikedMap[t.id],
      }));
    }
    return MOCK_TRACKS.map((t) => ({
      ...t,
      isLiked: isLikedMap[t.id] !== undefined ? isLikedMap[t.id] : t.isLiked,
    }));
  }, [realTracks, isLikedMap]);

  // Convert real playlists or use rich mock playlists
  const combinedPlaylists: ExplorationPlaylist[] = useMemo(() => {
    if (realPlaylists.length >= 2) {
      return realPlaylists.map((pl, idx) => ({
        id: pl.id,
        name: pl.name,
        description: pl.description || "Curated playlist collection.",
        trackCount: pl.track_count || 12,
        coverArt: pl.cover_art_url || MOCK_PLAYLISTS[idx % MOCK_PLAYLISTS.length].coverArt,
        isSmartMix: pl.is_smart_mix === 1,
        accentColor: MOCK_PLAYLISTS[idx % MOCK_PLAYLISTS.length].accentColor,
        tracks: combinedTracks.slice(0, 4),
      }));
    }
    return MOCK_PLAYLISTS.map((pl) => ({
      ...pl,
      tracks: combinedTracks.slice(0, 4),
    }));
  }, [realPlaylists, combinedTracks]);

  const combinedAlbums: ExplorationAlbum[] = useMemo(() => {
    return MOCK_ALBUMS.map((alb) => ({
      ...alb,
      tracks: combinedTracks.slice(0, 3),
    }));
  }, [combinedTracks]);

  const currentTrack = combinedTracks[internalTrackIndex] || combinedTracks[0];

  // Sync real Kaze playback if active
  const isPlaying = playbackState ? playbackState.is_playing : internalPlaying;
  const playbackSecs = playbackState ? playbackState.position_secs : internalPos;

  // Leaf timer simulation for prototype state
  useEffect(() => {
    if (!isPlaying) return;
    const interval = setInterval(() => {
      setInternalPos((prev) => (prev + 1) % (currentTrack.durationSecs || 240));
    }, 1000);
    return () => clearInterval(interval);
  }, [isPlaying, currentTrack.durationSecs]);

  const handlePlayTrack = (track: ExplorationTrack) => {
    const idx = combinedTracks.findIndex((t) => t.id === track.id);
    if (idx !== -1) setInternalTrackIndex(idx);
    setInternalPos(0);
    setInternalPlaying(true);

    if (onPlayRealTrack) {
      onPlayRealTrack(track.id);
    }
  };

  const handleTogglePlayPause = () => {
    if (onTogglePlayPause) {
      onTogglePlayPause();
    } else {
      setInternalPlaying(!internalPlaying);
    }
  };

  const handleNextTrack = () => {
    if (onNextTrack) {
      onNextTrack();
    } else {
      setInternalTrackIndex((prev) => (prev + 1) % combinedTracks.length);
      setInternalPos(0);
    }
  };

  const handlePrevTrack = () => {
    if (onPrevTrack) {
      onPrevTrack();
    } else {
      setInternalTrackIndex((prev) => (prev > 0 ? prev - 1 : combinedTracks.length - 1));
      setInternalPos(0);
    }
  };

  const handleSeek = (secs: number) => {
    if (onSeek) {
      onSeek(secs);
    } else {
      setInternalPos(secs);
    }
  };

  const handleToggleLike = (trackId: string) => {
    setIsLikedMap((prev) => ({ ...prev, [trackId]: !prev[trackId] }));
    if (onLike) onLike(trackId);
  };

  const appContext: ExplorationAppContext = {
    currentView,
    setCurrentView,
    tracks: combinedTracks,
    playlists: combinedPlaylists,
    albums: combinedAlbums,
    currentTrack,
    isPlaying,
    playbackSecs,
    volume: playbackState ? playbackState.volume : 0.85,
    isShuffled: playbackState ? playbackState.is_shuffled : false,
    repeatMode: playbackState ? playbackState.repeat_mode : "off",
    isLiked: !!currentTrack.isLiked,
    searchQuery,
    setSearchQuery,
    onPlayTrack: handlePlayTrack,
    onTogglePlayPause: handleTogglePlayPause,
    onNextTrack: handleNextTrack,
    onPrevTrack: handlePrevTrack,
    onSeek: handleSeek,
    onVolumeChange: onVolumeChange || (() => {}),
    onToggleShuffle: () => {},
    onToggleRepeat: () => {},
    onToggleLike: handleToggleLike,
  };

  return (
    <div className="design-exploration-root">
      {/* Active Model App Experience */}
      <div className="design-exploration-viewport">
        {model === "drift" && <DriftApp {...appContext} />}
        {model === "broadsheet" && <BroadsheetApp {...appContext} />}
        {model === "gallery" && <GalleryApp {...appContext} />}
      </div>

      {/* Floating Design Model Switcher Dock */}
      <DesignSwitcher currentModel={model} onSelectModel={onSelectModel} />
    </div>
  );
};
