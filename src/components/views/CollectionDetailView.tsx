import React, { useState, useMemo } from "react";
import {
  Play,
  Pause,
  Shuffle,
  Plus,
  Clock,
  ArrowLeft,
  Music2,
  DownloadCloud,
  Bookmark,
  BookmarkCheck,
  CheckCircle2,
  RefreshCw,
  MoreVertical,
  Pencil,
  Trash2,
  Heart,
  ListPlus,
  Check,
  Search,
  X,
} from "lucide-react";
import { Track, DiscoveryRecommendation, DownloadTask } from "../../types";
import { filterPlaylistEntries } from "../../playlistSearch";

export interface CollectionTrackItem {
  id: string;
  title: string;
  artist: string;
  album?: string;
  duration_secs: number;
  cover_art_url?: string;
  preview_url?: string;
  is_downloaded?: boolean;
  matched_local_track_id?: string;
  rawLocalTrack?: Track;
  rawRecommendation?: DiscoveryRecommendation;
}

export interface CollectionData {
  id: string;
  type: "playlist" | "mix" | "chart";
  title: string;
  subtitle: string;
  tag: string;
  bgGradient?: string;
  accentColor?: string;
  searchQuery?: string;
  playlistId?: string;
  curator?: string;
  tracks?: CollectionTrackItem[];
  cover_art_url?: string;
}

interface CollectionDetailViewProps {
  collection: CollectionData;
  isLoadingTracks?: boolean;
  loadingTrackId?: string | null;
  onBack: () => void;
  onPlayTrack: (track: CollectionTrackItem) => void;
  onPlayAll: () => void;
  onShuffleAll?: () => void;
  onAddToWishlist?: (track: CollectionTrackItem) => void;
  onDownload?: (artist: string, title: string) => void;
  onSaveToPlaylists?: (collection: CollectionData) => void;
  isSaved?: boolean;
  currentPlayingTrackId?: string | null;
  isPlaying?: boolean;
  isShuffled?: boolean;
  downloads?: DownloadTask[];
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
  isLoadingTracks = false,
  loadingTrackId = null,
  onBack,
  onPlayTrack,
  onPlayAll,
  onShuffleAll,
  onAddToWishlist,
  onDownload,
  onSaveToPlaylists,
  isSaved = false,
  currentPlayingTrackId,
  isPlaying = false,
  isShuffled = false,
  downloads = [],
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
  const visibleTrackEntries = useMemo(() => filterPlaylistEntries(rawTracks, searchQuery), [rawTracks, searchQuery]);
  const tracks = visibleTrackEntries.map(({ track }) => track);

  const activeDownloadMap = useMemo(() => {
    const map = new Map<string, DownloadTask>();
    if (!downloads || downloads.length === 0) return map;
    for (const d of downloads) {
      if (d.status === "FAILED" || d.status === "CANCELLED") continue;
      const t = d.title.toLowerCase().trim();
      const a = (d.artist || "").toLowerCase().trim();
      map.set(`${a}:::${t}`, d);
      map.set(`t:::${t}`, d);
    }
    return map;
  }, [downloads]);

  const totalDurationSecs = tracks.reduce((acc, t) => acc + (t.duration_secs || 0), 0);

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

  // 3 hero circular images
  const heroImage1 = collection.cover_art_url || tracks[0]?.cover_art_url;
  const heroImage2 = tracks[1]?.cover_art_url || heroImage1;
  const heroImage3 = tracks[2]?.cover_art_url || tracks[1]?.cover_art_url || heroImage1;

  const bgGradient =
    collection.bgGradient ||
    "linear-gradient(180deg, rgba(180, 130, 40, 0.85) 0%, rgba(18, 20, 28, 0.98) 100%)";

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "0px",
        paddingBottom: "40px",
        minHeight: "100%",
      }}
    >
      {/* 1. Hero Header Banner */}
      <div
        style={{
          position: "relative",
          borderRadius: "14px",
          overflow: "hidden",
          background: bgGradient,
          padding: "28px 32px 36px 32px",
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          gap: "24px",
          boxShadow: "0 8px 32px rgba(0,0,0,0.4)",
          minHeight: "260px",
        }}
      >
        {/* Back Button on top left */}
        <button
          type="button"
          onClick={onBack}
          style={{
            position: "absolute",
            top: "16px",
            left: "16px",
            display: "inline-flex",
            alignItems: "center",
            gap: "6px",
            backgroundColor: "rgba(0,0,0,0.65)",
            border: "1px solid rgba(255,255,255,0.15)",
            color: "#e8d8c9",
            borderRadius: "20px",
            padding: "6px 14px",
            fontSize: "0.82rem",
            fontWeight: 600,
            cursor: "pointer",
            zIndex: 10,
            transition: "all 0.15s ease",
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.backgroundColor = "rgba(0,0,0,0.7)";
            e.currentTarget.style.borderColor = "rgba(255,255,255,0.3)";
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.backgroundColor = "rgba(0,0,0,0.45)";
            e.currentTarget.style.borderColor = "rgba(255,255,255,0.15)";
          }}
        >
          <ArrowLeft size={16} />
          <span>Back</span>
        </button>

        {/* Left: Text Metadata */}
        <div style={{ flex: 1, marginTop: "24px", zIndex: 2 }}>
          <div
            style={{
              fontSize: "0.8rem",
              fontWeight: 700,
              letterSpacing: "1.2px",
              color: "rgba(255, 255, 255, 0.85)",
              textTransform: "uppercase",
              marginBottom: "8px",
            }}
          >
            {collection.tag || "PUBLIC PLAYLIST"}
          </div>

          <h1
            style={{
              fontSize: "clamp(2rem, 4vw, 3.2rem)",
              fontWeight: 900,
              color: "#e8d8c9",
              lineHeight: 1.1,
              letterSpacing: "-0.5px",
              marginBottom: "12px",
              textShadow: "0 3px 12px rgba(0,0,0,0.4)",
            }}
          >
            {collection.title}
          </h1>

          <p
            style={{
              fontSize: "0.95rem",
              color: "rgba(255, 255, 255, 0.9)",
              maxWidth: "600px",
              lineHeight: 1.4,
              marginBottom: "14px",
              textShadow: "0 1px 4px rgba(0,0,0,0.4)",
            }}
          >
            {collection.subtitle}
          </p>

          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: "8px",
              fontSize: "0.85rem",
              color: "rgba(255, 255, 255, 0.85)",
              flexWrap: "wrap",
            }}
          >
            <span style={{ fontWeight: 700, color: "#e8d8c9" }}>Kaze</span>
            <span>•</span>
            <span>
              {searchQuery.trim()
                ? `${tracks.length} of ${rawTracks.length} ${rawTracks.length === 1 ? "song" : "songs"}`
                : `${rawTracks.length} ${rawTracks.length === 1 ? "song" : "songs"}`}
            </span>
            {totalDurationSecs > 0 && (
              <>
                <span>•</span>
                <span>{formatTotalDuration(totalDurationSecs)}</span>
              </>
            )}
          </div>
        </div>

        {/* Right: 3 Overlapping Circular Hero Portraits (Spotify cutout style) */}
        <div
          style={{
            position: "relative",
            width: "320px",
            height: "220px",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            flexShrink: 0,
            zIndex: 2,
          }}
        >
          {/* Left circle */}
          <div
            style={{
              position: "absolute",
              left: "10px",
              width: "135px",
              height: "135px",
              borderRadius: "50%",
              overflow: "hidden",
              border: "3px solid rgba(255, 255, 255, 0.2)",
              boxShadow: "0 8px 24px rgba(0,0,0,0.5)",
              backgroundColor: "rgba(0,0,0,0.3)",
              zIndex: 1,
            }}
          >
            {heroImage1 ? (
              <img src={heroImage1} alt="Cover 1" style={{ width: "100%", height: "100%", objectFit: "cover" }} />
            ) : (
              <div style={{ width: "100%", height: "100%", display: "flex", alignItems: "center", justifyContent: "center" }}>
                <Music2 size={40} color="rgba(255,255,255,0.4)" />
              </div>
            )}
          </div>

          {/* Right circle */}
          <div
            style={{
              position: "absolute",
              right: "10px",
              width: "135px",
              height: "135px",
              borderRadius: "50%",
              overflow: "hidden",
              border: "3px solid rgba(255, 255, 255, 0.2)",
              boxShadow: "0 8px 24px rgba(0,0,0,0.5)",
              backgroundColor: "rgba(0,0,0,0.3)",
              zIndex: 1,
            }}
          >
            {heroImage3 ? (
              <img src={heroImage3} alt="Cover 3" style={{ width: "100%", height: "100%", objectFit: "cover" }} />
            ) : (
              <div style={{ width: "100%", height: "100%", display: "flex", alignItems: "center", justifyContent: "center" }}>
                <Music2 size={40} color="rgba(255,255,255,0.4)" />
              </div>
            )}
          </div>

          {/* Center circle (Largest, on top) */}
          <div
            style={{
              position: "relative",
              width: "165px",
              height: "165px",
              borderRadius: "50%",
              overflow: "hidden",
              border: "4px solid rgba(255, 255, 255, 0.3)",
              boxShadow: "0 12px 32px rgba(0,0,0,0.6)",
              backgroundColor: "rgba(0,0,0,0.4)",
              zIndex: 2,
            }}
          >
            {heroImage2 ? (
              <img src={heroImage2} alt="Cover 2" style={{ width: "100%", height: "100%", objectFit: "cover" }} />
            ) : (
              <div style={{ width: "100%", height: "100%", display: "flex", alignItems: "center", justifyContent: "center" }}>
                <Music2 size={50} color="rgba(255,255,255,0.5)" />
              </div>
            )}
          </div>
        </div>
      </div>

      {/* 2. Action Toolbar */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          padding: "20px 8px 16px 8px",
          flexWrap: "wrap",
          gap: "14px",
        }}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "18px" }}>
          {/* Big Green Play / Pause Button */}
          <button
            type="button"
            onClick={onPlayAll}
            style={{
              width: "54px",
              height: "54px",
              borderRadius: "50%",
              backgroundColor: "var(--accent-secondary)",
              border: "none",
              color: "#e8d8c9",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              cursor: "pointer",
              boxShadow: "0 6px 20px rgba(139, 124, 246, 0.4)",
              transition: "transform 0.15s ease, background-color 0.15s ease",
            }}
            onMouseEnter={(e) => (e.currentTarget.style.transform = "scale(1.05)")}
            onMouseLeave={(e) => (e.currentTarget.style.transform = "scale(1)")}
            title={isPlaying ? "Pause Collection" : "Play Collection"}
          >
            {isPlaying ? (
              <Pause size={24} fill="#e8d8c9" />
            ) : (
              <Play size={24} fill="#e8d8c9" style={{ marginLeft: "3px" }} />
            )}
          </button>

          {/* Shuffle Button */}
          {onShuffleAll && (
            <button
              type="button"
              onClick={onShuffleAll}
              style={{
                background: isShuffled ? "rgba(139, 124, 246, 0.15)" : "none",
                border: isShuffled ? "1px solid rgba(139, 124, 246, 0.4)" : "1px solid transparent",
                borderRadius: "50%",
                width: "40px",
                height: "40px",
                color: isShuffled ? "var(--accent-secondary)" : "var(--text-muted)",
                cursor: "pointer",
                padding: "8px",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                transition: "all 0.15s ease",
                position: "relative",
              }}
              onMouseEnter={(e) => {
                if (!isShuffled) e.currentTarget.style.color = "#e8d8c9";
              }}
              onMouseLeave={(e) => {
                if (!isShuffled) e.currentTarget.style.color = "var(--text-muted)";
              }}
              title={isShuffled ? "Shuffle is active (Click to deactivate)" : "Shuffle collection"}
            >
              <Shuffle size={20} color={isShuffled ? "var(--accent-secondary)" : undefined} />
              {isShuffled && (
                <span
                  style={{
                    position: "absolute",
                    bottom: "3px",
                    width: "4px",
                    height: "4px",
                    borderRadius: "50%",
                    backgroundColor: "var(--accent-secondary)",
                  }}
                />
              )}
            </button>
          )}

          {/* Already in Collection Bookmarked Icon (non-clickable) OR Save Button */}
          {isInCollection ? (
            <div
              style={{
                border: "1px solid rgba(139, 124, 246, 0.45)",
                backgroundColor: "rgba(139, 124, 246, 0.12)",
                borderRadius: "50%",
                width: "40px",
                height: "40px",
                color: "var(--accent-secondary)",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                cursor: "default",
                userSelect: "none",
              }}
              title="Already in collection"
            >
              <BookmarkCheck size={20} />
            </div>
          ) : onSaveToPlaylists ? (
            <button
              type="button"
              onClick={() => onSaveToPlaylists(collection)}
              style={{
                background: "none",
                border: "1px solid rgba(255, 255, 255, 0.2)",
                borderRadius: "50%",
                width: "40px",
                height: "40px",
                color: "var(--text-muted)",
                cursor: "pointer",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                transition: "all 0.15s ease",
              }}
              title="Save collection to your library"
            >
              <Plus size={18} />
            </button>
          ) : null}

          {collection.type === "playlist" && collection.playlistId && (
            <details style={{ position: "relative" }}>
              <summary
                aria-label={`Playlist options for ${collection.title}`}
                style={{
                  listStyle: "none",
                  border: "1px solid rgba(255, 255, 255, 0.2)",
                  borderRadius: "50%",
                  width: "40px",
                  height: "40px",
                  color: "var(--text-muted)",
                  cursor: "pointer",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                }}
              >
                <MoreVertical size={20} />
              </summary>
              <div style={{ position: "absolute", top: "46px", left: 0, zIndex: 20, minWidth: "160px", padding: "6px", borderRadius: "8px", border: "1px solid var(--border)", background: "var(--bg-sidebar)", boxShadow: "0 12px 30px rgba(0,0,0,0.45)" }}>
                {isLikedSongs ? (
                  <div style={{ padding: "8px 10px", color: "var(--text-dim)", fontSize: "0.78rem" }}>Default playlist</div>
                ) : (
                  <>
                    <button className="btn btn-secondary" style={{ width: "100%", justifyContent: "flex-start", border: 0 }} onClick={handleRenamePlaylist}>
                      <Pencil size={14} /> Rename
                    </button>
                    <button className="btn btn-secondary" style={{ width: "100%", justifyContent: "flex-start", border: 0, color: "var(--danger, #ef4444)" }} onClick={handleDeletePlaylist}>
                      <Trash2 size={14} /> Delete
                    </button>
                  </>
                )}
              </div>
            </details>
          )}
        </div>

        {/* Search inside Playlist */}
        {rawTracks.length > 0 && (
          <div style={{ display: "flex", alignItems: "center", position: "relative" }}>
            <Search
              size={15}
              color="var(--text-muted)"
              style={{ position: "absolute", left: "12px", pointerEvents: "none" }}
            />
            <input
              type="text"
              placeholder="Search in playlist..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              style={{
                padding: "8px 30px 8px 34px",
                borderRadius: "20px",
                border: "1px solid var(--border)",
                backgroundColor: "rgba(255, 255, 255, 0.05)",
                color: "#e8d8c9",
                fontSize: "0.82rem",
                outline: "none",
                width: "220px",
                transition: "all 0.2s ease",
              }}
            />
            {searchQuery && (
              <button
                type="button"
                onClick={() => setSearchQuery("")}
                style={{
                  position: "absolute",
                  right: "10px",
                  background: "none",
                  border: "none",
                  color: "var(--text-muted)",
                  cursor: "pointer",
                  padding: "2px",
                  display: "flex",
                  alignItems: "center",
                }}
                title="Clear search"
              >
                <X size={13} />
              </button>
            )}
          </div>
        )}
      </div>

      {/* 3. Track Table */}
      {isLoadingTracks ? (
        <div key="collection-search-loading" style={{ textAlign: "center", padding: "60px 20px", color: "var(--text-dim)" }}>
          <RefreshCw size={36} className="animate-spin" color="var(--accent-light)" style={{ margin: "0 auto 12px" }} />
          <p style={{ fontSize: "1rem", fontWeight: 600, color: "var(--text-main)" }}>Loading collection songs...</p>
        </div>
      ) : tracks.length === 0 ? (
        rawTracks.length > 0 ? (
          <div
            key="collection-search-empty"
            className="content-card"
            style={{ textAlign: "center", padding: "50px 20px", color: "var(--text-dim)", borderStyle: "dashed" }}
          >
            <Search size={36} color="var(--accent-light)" style={{ margin: "0 auto 12px" }} />
            <p style={{ fontSize: "1.05rem", fontWeight: 600, color: "var(--text-main)", marginBottom: "6px" }}>
              No songs found matching "{searchQuery}"
            </p>
            <p style={{ fontSize: "0.84rem", color: "var(--text-muted)", marginBottom: "16px" }}>
              Check your spelling or try searching for a different track title or artist.
            </p>
            <button
              type="button"
              className="btn btn-secondary"
              onClick={() => setSearchQuery("")}
              style={{ fontSize: "0.82rem" }}
            >
              Clear Search
            </button>
          </div>
        ) : (
          <div
            key="collection-empty"
            className="content-card"
            style={{ textAlign: "center", padding: "50px 20px", color: "var(--text-dim)", borderStyle: "dashed" }}
          >
            <Music2 size={36} color="var(--accent-light)" style={{ marginBottom: "12px" }} />
            <p style={{ fontSize: "1.05rem", fontWeight: 600, color: "var(--text-main)", marginBottom: "6px" }}>
              No songs in this collection yet
            </p>
          </div>
        )
      ) : (
        <div
          key="collection-search-results"
          style={{
            backgroundColor: "var(--bg-card)",
            borderRadius: "12px",
            border: "1px solid var(--border)",
            overflow: "hidden",
          }}
        >
          {/* Table Header */}
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "48px 1fr 1fr 70px 150px",
              padding: "12px 18px",
              borderBottom: "1px solid var(--border)",
              fontSize: "0.78rem",
              fontWeight: 600,
              letterSpacing: "0.6px",
              color: "var(--text-dim)",
              textTransform: "uppercase",
            }}
          >
            <div>#</div>
            <div>Title</div>
            <div>Album</div>
            <div style={{ textAlign: "right" }}>
              <Clock size={14} style={{ display: "inline-block" }} />
            </div>
            <div style={{ textAlign: "right" }}>Action</div>
          </div>

          {/* Table Rows */}
          {tracks.map((track, idx) => {
            const isHovered = hoveredRowId === track.id;
            const isPlayingThis =
              isPlaying &&
              (currentPlayingTrackId === track.id ||
                currentPlayingTrackId === track.matched_local_track_id);
            const isLoadingThis = loadingTrackId === track.id ||
              (!!track.matched_local_track_id && loadingTrackId === track.matched_local_track_id);

            const activeDownload =
              activeDownloadMap.get(`${(track.artist || "").toLowerCase().trim()}:::${track.title.toLowerCase().trim()}`) ||
              activeDownloadMap.get(`t:::${track.title.toLowerCase().trim()}`);

            const isDownloaded = track.is_downloaded || activeDownload?.status === "COMPLETED";
            const isDownloading =
              activeDownload &&
              (activeDownload.status === "DOWNLOADING" || activeDownload.status === "QUEUED");
            const downloadPercent =
              activeDownload?.file_size && activeDownload.file_size > 0
                ? Math.min(100, Math.round((activeDownload.bytes_downloaded / activeDownload.file_size) * 100))
                : activeDownload?.status === "DOWNLOADING"
                ? 50
                : 0;
            const effectiveTrackId = track.matched_local_track_id || track.id;
            const isLiked = likedTrackIds?.has(effectiveTrackId) || isLikedSongs;
            const isQueued = queuedTrackIds?.has(effectiveTrackId) || false;

            return (
              <div
                key={visibleTrackEntries[idx].rowKey}
                onMouseEnter={() => setHoveredRowId(track.id)}
                onMouseLeave={() => setHoveredRowId(null)}
                onClick={() => onPlayTrack(track)}
                style={{
                  display: "grid",
                  gridTemplateColumns: "48px 1fr 1fr 70px 150px",
                  padding: "10px 18px",
                  alignItems: "center",
                  fontSize: "0.88rem",
                  color: isPlayingThis || isLoadingThis ? "var(--accent-secondary)" : "var(--text-main)",
                  backgroundColor: isPlayingThis || isLoadingThis
                    ? "rgba(139, 124, 246, 0.08)"
                    : isHovered
                    ? "rgba(255, 255, 255, 0.04)"
                    : "transparent",
                  borderBottom: "1px solid rgba(255, 255, 255, 0.03)",
                  cursor: "pointer",
                  transition: "background-color 0.15s ease",
                }}
              >
                {/* Index / Play icon */}
                <div style={{ color: isPlayingThis || isLoadingThis ? "var(--accent-secondary)" : "var(--text-dim)", fontSize: "0.85rem", fontWeight: 500 }}>
                  {isLoadingThis ? (
                    <RefreshCw size={15} className="spin-animation" aria-label="Loading song" />
                  ) : isPlayingThis ? (
                    isHovered ? (
                      <Pause size={15} fill="var(--accent-secondary)" />
                    ) : (
                      <div className="discovery-eq-container" style={{ height: "13px" }}>
                        <div className="discovery-eq-bar" style={{ width: "2.5px", backgroundColor: "var(--accent-secondary)" }} />
                        <div className="discovery-eq-bar" style={{ width: "2.5px", backgroundColor: "var(--accent-secondary)" }} />
                        <div className="discovery-eq-bar" style={{ width: "2.5px", backgroundColor: "var(--accent-secondary)" }} />
                      </div>
                    )
                  ) : isHovered ? (
                    <Play size={15} fill="#e8d8c9" />
                  ) : (
                    idx + 1
                  )}
                </div>

                {/* Title & Artist & Artwork */}
                <div style={{ display: "flex", alignItems: "center", gap: "12px", minWidth: 0 }}>
                  <div
                    style={{
                      width: "42px",
                      height: "42px",
                      borderRadius: "6px",
                      overflow: "hidden",
                      backgroundColor: "rgba(255, 255, 255, 0.05)",
                      flexShrink: 0,
                    }}
                  >
                    {track.cover_art_url ? (
                      <img
                        src={track.cover_art_url}
                        alt={track.title}
                        style={{ width: "100%", height: "100%", objectFit: "cover" }}
                      />
                    ) : (
                      <div style={{ width: "100%", height: "100%", display: "flex", alignItems: "center", justifyContent: "center" }}>
                        <Music2 size={20} color="var(--text-dim)" />
                      </div>
                    )}
                  </div>

                  <div style={{ overflow: "hidden", minWidth: 0 }}>
                    <div
                      style={{
                        fontWeight: 600,
                        fontSize: "0.92rem",
                        whiteSpace: "nowrap",
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        color: isPlayingThis ? "var(--accent-secondary)" : "#e8d8c9",
                      }}
                    >
                      {track.title}
                    </div>
                    <div
                      style={{
                        fontSize: "0.78rem",
                        color: "var(--text-muted)",
                        whiteSpace: "nowrap",
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                        marginTop: "2px",
                      }}
                    >
                      {track.artist}
                    </div>
                  </div>
                </div>

                {/* Album */}
                <div
                  style={{
                    color: "var(--text-muted)",
                    fontSize: "0.84rem",
                    whiteSpace: "nowrap",
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    paddingRight: "16px",
                  }}
                >
                  {track.album || "—"}
                </div>

                {/* Duration */}
                <div style={{ textAlign: "right", color: "var(--text-dim)", fontSize: "0.82rem" }}>
                  {formatDuration(track.duration_secs)}
                </div>

                {/* Action Buttons */}
                <div
                  style={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "flex-end",
                    gap: "6px",
                  }}
                  onClick={(e) => e.stopPropagation()}
                >
                  {isDownloaded ? (
                    <span title="Downloaded in local library" style={{ color: "var(--accent-secondary)", display: "inline-flex" }}>
                      <CheckCircle2 size={16} />
                    </span>
                  ) : isDownloading ? (
                    <div
                      style={{
                        display: "inline-flex",
                        alignItems: "center",
                        gap: "4px",
                        padding: "2px 7px",
                        borderRadius: "12px",
                        background: "rgba(243, 112, 30, 0.15)",
                        border: "1px solid rgba(243, 112, 30, 0.35)",
                        fontSize: "0.72rem",
                        fontWeight: 600,
                        color: "#8b7cf6",
                      }}
                      title={activeDownload.status === "QUEUED" ? "Download queued..." : `Downloading ${downloadPercent}%`}
                    >
                      <RefreshCw size={11} className="spin-animation" />
                      <span>{activeDownload.status === "QUEUED" ? "Queued" : `${downloadPercent}%`}</span>
                    </div>
                  ) : onDownload ? (
                    <button
                      type="button"
                      onClick={() => onDownload(track.artist, track.title)}
                      style={{
                        background: "none",
                        border: "none",
                        color: "var(--text-dim)",
                        cursor: "pointer",
                        padding: "4px",
                        display: "flex",
                      }}
                      title="Download track"
                    >
                      <DownloadCloud size={15} />
                    </button>
                  ) : null}

                  <button
                    type="button"
                    onClick={() => onEnqueueTrack?.(track)}
                    style={{ background: isQueued ? "rgba(139,124,246,0.16)" : "none", border: isQueued ? "1px solid rgba(139,124,246,0.4)" : "1px solid transparent", borderRadius: "50%", color: isQueued ? "var(--accent-secondary)" : "var(--text-dim)", cursor: "pointer", padding: "4px", display: "flex" }}
                    title={isQueued ? "Already in queue" : "Add to queue"}
                  >
                    {isQueued ? <Check size={15} /> : <ListPlus size={15} />}
                  </button>

                  {isLikedSongs ? (
                    <span title="Liked song" style={{ color: "#ec4899", padding: "4px", display: "flex" }}>
                      <Heart size={15} fill="#ec4899" />
                    </span>
                  ) : (
                    <button
                      type="button"
                      onClick={() => onToggleLike?.(track, !!isLiked)}
                      style={{ background: "none", border: "none", color: isLiked ? "#ec4899" : "var(--text-dim)", cursor: "pointer", padding: "4px", display: "flex" }}
                      title={isLiked ? "Unlike track" : "Like track"}
                    >
                      <Heart size={15} fill={isLiked ? "#ec4899" : "none"} />
                    </button>
                  )}

                  {(() => {
                    const trackId = track.matched_local_track_id || track.id;
                    const allPlaylistIds = (trackId && trackPlaylistMap?.[trackId]) || [];
                    // Only count user-created playlists (exclude smart mixes and the current collection itself)
                    const playlistIds = userPlaylistIds
                      ? allPlaylistIds.filter((pid) => userPlaylistIds.has(pid))
                      : allPlaylistIds;
                    const isInPlaylist = playlistIds.length > 0;

                    return (
                      <button
                        type="button"
                        onClick={() => {
                          if (onAddToPlaylist) {
                            onAddToPlaylist(track);
                          } else if (onAddToWishlist) {
                            onAddToWishlist(track);
                          }
                        }}
                        style={{
                          background: "none",
                          border: "none",
                          color: isInPlaylist ? "var(--accent-secondary)" : "var(--text-dim)",
                          cursor: "pointer",
                          padding: "4px",
                          display: "flex",
                        }}
                        title={isInPlaylist ? "In playlist (click to manage)" : "Add to playlist"}
                      >
                        {isInPlaylist ? (
                          <BookmarkCheck size={15} color="var(--accent-secondary)" />
                        ) : (
                          <Bookmark size={15} />
                        )}
                      </button>
                    );
                  })()}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};
