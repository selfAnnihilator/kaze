import { useInterfaceStyle } from "../../presentation/InterfaceContext";
import { useDebouncedValue } from "../../presentation/useDebouncedValue";
import React, { useState, useMemo } from "react";
import { filterPlaylistEntries } from "../../playlistSearch";
import type { CollectionDetailViewContext } from "./collection/context";
import { CollectionHero } from "./collection/CollectionHero";
import { CollectionToolbar } from "./collection/CollectionToolbar";
import { CollectionTracks } from "./collection/CollectionTracks";

export type { CollectionData, CollectionTrackItem } from "./collection/types";
import type { CollectionData, CollectionTrackItem } from "./collection/types";

interface CollectionDetailViewProps {
  collection: CollectionData;
  withinArchive?: boolean;
  isLoadingTracks?: boolean;
  loadingTrackId?: string | null;
  onBack: () => void;
  onPlayTrack: (track: CollectionTrackItem) => void;
  onPlayAll: () => void;
  onShuffleAll?: () => void;
  onAddToWishlist?: (track: CollectionTrackItem) => void;
  onSaveToPlaylists?: (collection: CollectionData) => void;
  isSaved?: boolean;
  currentPlayingTrackId?: string | null;
  isPlaying?: boolean;
  isShuffled?: boolean;
  trackPlaylistMap?: Record<string, string[]>;
  onAddToPlaylist?: (track: CollectionTrackItem) => void;
  /** IDs of user-created (non-smart-mix) playlists — used to filter bookmark state */
  userPlaylistIds?: Set<string>;
  onRenamePlaylist?: (playlistId: string, name: string) => Promise<void>;
  onDeletePlaylist?: (playlistId: string) => Promise<void>;
  likedTrackIds?: Set<string>;
  onToggleLike?: (track: CollectionTrackItem, isLiked: boolean) => void;
  queuedTrackIds?: Set<string>;
  onEnqueueTrack?: (track: CollectionTrackItem) => void;
}


export const CollectionDetailView: React.FC<CollectionDetailViewProps> = ({
  collection,
  withinArchive = false,
  isLoadingTracks = false,
  loadingTrackId = null,
  onBack,
  onPlayTrack,
  onPlayAll,
  onShuffleAll,
  onAddToWishlist,
  onSaveToPlaylists,
  isSaved = false,
  currentPlayingTrackId,
  isPlaying = false,
  isShuffled = false,
  trackPlaylistMap,
  onAddToPlaylist,
  userPlaylistIds,
  onRenamePlaylist,
  onDeletePlaylist,
  likedTrackIds,
  onToggleLike,
  queuedTrackIds,
  onEnqueueTrack,
}) => {
  const [hoveredRowId, setHoveredRowId] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");

  const filteredQuery = useDebouncedValue(searchQuery);

  const isInCollection = isSaved || collection.type === "playlist";
  const isLikedSongs = collection.type === "playlist" && collection.title === "Liked Songs";

  const handleRenamePlaylist = async () => {
    if (!collection.playlistId || !onRenamePlaylist || isLikedSongs) return;
    const name = window.prompt("Rename playlist", collection.title)?.trim();
    if (!name || name === collection.title) return;
    try {
      await onRenamePlaylist(collection.playlistId, name);
    } catch (err: any) {
      console.warn("Could not rename playlist:", err);
    }
  };

  const handleDeletePlaylist = async () => {
    if (!collection.playlistId || !onDeletePlaylist || isLikedSongs) return;
    try {
      await onDeletePlaylist(collection.playlistId);
    } catch (err: any) {
      console.warn("Could not delete playlist:", err);
    }
  };

  const rawTracks = collection.tracks || [];
  const visibleTrackEntries = useMemo(() => filterPlaylistEntries(rawTracks, filteredQuery),
    [rawTracks, filteredQuery],
  );
  const tracks = visibleTrackEntries.map(({ track }) => track);

  const totalDurationSecs = tracks.reduce(
    (acc, t) => acc + (t.duration_secs || 0),
    0,
  );

  const formatTotalDuration = (totalSecs: number) => {
    if (totalSecs <= 0) return "";
    const hours = Math.floor(totalSecs / 3600);
    const mins = Math.floor((totalSecs % 3600) / 60);
    if (hours > 0) {
      return `about ${hours} hr ${mins} min`;
    }
    return `${mins} min`;
  };

  const formatDuration = (secs: number) => {
    if (!secs || isNaN(secs)) return "0:00";
    const m = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    return `${m}:${s < 10 ? "0" : ""}${s}`;
  };

  // Artwork occupies each presentation's record region.
  const interfaceStyle = useInterfaceStyle();
  const heroImage1 = collection.cover_art_url || tracks[0]?.cover_art_url;
  const bgGradient =
    collection.bgGradient ||
    "linear-gradient(180deg, var(--palette-orange-550-a85) 0%, var(--palette-slate-900-a95) 100%)";

  const c: CollectionDetailViewContext = {
    withinArchive, collection, isLoadingTracks, rawTracks, bgGradient, onBack, searchQuery, tracks,
    totalDurationSecs, formatTotalDuration, interfaceStyle, heroImage1, onPlayAll, isPlaying,
    onShuffleAll, isShuffled, isInCollection, onSaveToPlaylists, isLikedSongs, handleRenamePlaylist,
    handleDeletePlaylist, setSearchQuery, hoveredRowId, currentPlayingTrackId, loadingTrackId,
    likedTrackIds, queuedTrackIds, visibleTrackEntries, setHoveredRowId, onPlayTrack, formatDuration,
    onEnqueueTrack, onToggleLike, trackPlaylistMap, userPlaylistIds, onAddToPlaylist, onAddToWishlist,
  };
  return (
    <div
      className={`collection-view${isLikedSongs ? " collection-view--liked" : ""}${withinArchive ? " collection-view--archive" : ""}`}
      role={withinArchive ? "tabpanel" : undefined}
      aria-label={withinArchive ? collection.title : undefined}
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "0px",
        paddingBottom: "var(--space-40)",
        minHeight: "100%",
      }}
    >
      {/* 1. Hero Header Banner */}
      <CollectionHero c={c} />

      {/* 2. Action Toolbar */}
      <CollectionToolbar c={c} />

      {/* 3. Track Table */}
      <CollectionTracks c={c} />
    </div>
  );
};
