import React, { useState, useRef, useEffect, useLayoutEffect, useMemo } from "react";
import {
  DownloadCloud,
  Sparkles,
  Bookmark,
  BookmarkCheck,
  WifiOff,
  Music,
  RefreshCw,
  Radio,
  Music2,
  Play,
  Pause,
  CheckCircle2,
  ChevronLeft,
  ChevronRight,
  TrendingUp,
  ListPlus,
  Check,
  Heart,
} from "lucide-react";
import { DiscoveryRecommendation, Track, Playlist, DownloadTask } from "../../types";
import { executeQuery } from "../../services/api";
import { CollectionData } from "./CollectionDetailView";
import { OnlineSearchResultsSection } from "./OnlineSearchResultsSection";
import { splitDiscoveryRecommendations } from "../../forYouRecommendations";

interface DiscoveryTrackCardProps {
  rec: DiscoveryRecommendation;
  isPlaying: boolean;
  isLoading: boolean;
  isSelected: boolean;
  onPlay: (rec: DiscoveryRecommendation) => void;
  onStop: (rec: DiscoveryRecommendation) => void;
  onArtistClick: (artist: string) => void;
  onAddToWishlist: (rec: DiscoveryRecommendation) => void;
  onDownload: (artist: string, title: string) => void;
  downloads?: DownloadTask[];
  trackPlaylistMap?: Record<string, string[]>;
  onAddToPlaylist?: (rec: DiscoveryRecommendation) => void;
  isQueued?: boolean;
  onEnqueue?: (rec: DiscoveryRecommendation) => void;
  isLiked?: boolean;
}

const DiscoveryTrackCard: React.FC<DiscoveryTrackCardProps> = ({
  rec,
  isPlaying,
  isLoading,
  isSelected,
  onPlay,
  onStop,
  onArtistClick,
  onAddToWishlist,
  onDownload,
  downloads,
  trackPlaylistMap,
  onAddToPlaylist,
  isQueued = false,
  onEnqueue,
  isLiked = false,
}) => {
  const [isHovered, setIsHovered] = useState(false);

  const trackId = rec.matched_local_track_id || rec.external_track_id;
  const playlistIds = (trackId && trackPlaylistMap?.[trackId]) || [];
  const isInPlaylist = playlistIds.length > 0;

  const activeDownload = downloads?.find((d) => {
    if (d.status === "FAILED" || d.status === "CANCELLED") return false;
    const titleMatch =
      d.title.toLowerCase().trim() === rec.title.toLowerCase().trim() ||
      d.filename.toLowerCase().includes(rec.title.toLowerCase().trim());
    const artistMatch =
      !d.artist ||
      d.artist.toLowerCase().trim() === rec.artist.toLowerCase().trim() ||
      rec.artist.toLowerCase().includes(d.artist.toLowerCase().trim()) ||
      d.filename.toLowerCase().includes(rec.artist.toLowerCase().trim());
    return titleMatch && artistMatch;
  });

  const hasRealLocalMatch =
    rec.match_status === "EXACT_MATCH" &&
    !!rec.matched_local_track_id &&
    !rec.matched_local_track_id.startsWith("itunes:") &&
    !rec.matched_local_track_id.startsWith("online:");

  const isDownloaded =
    (rec.provider === "library" && !rec.external_track_id.startsWith("itunes:") && !rec.external_track_id.startsWith("online:")) ||
    hasRealLocalMatch ||
    activeDownload?.status === "COMPLETED";

  const isDownloading =
    activeDownload &&
    (activeDownload.status === "DOWNLOADING" || activeDownload.status === "QUEUED");

  const downloadPercent =
    activeDownload?.file_size && activeDownload.file_size > 0
      ? Math.min(100, Math.round((activeDownload.bytes_downloaded / activeDownload.file_size) * 100))
      : activeDownload?.status === "DOWNLOADING"
      ? 50
      : 0;

  const handleCardClick = () => {
    if (isLoading) return;
    if (isPlaying) {
      onStop(rec);
    } else {
      onPlay(rec);
    }
  };

  const handlePlayBtnClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (isPlaying) {
      onStop(rec);
    } else {
      onPlay(rec);
    }
  };

  return (
    <div
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      onClick={handleCardClick}
      style={{
        backgroundColor: isPlaying
          ? "rgba(139, 124, 246, 0.09)"
          : isSelected
          ? "rgba(243, 112, 30, 0.12)"
          : "var(--bg-card)",
        border: isPlaying
          ? "1.5px solid var(--accent-secondary)"
          : isSelected
          ? "1px solid var(--accent-light)"
          : isHovered
          ? "1px solid rgba(255, 255, 255, 0.22)"
          : "1px solid var(--border)",
        borderRadius: "12px",
        padding: "10px",
        display: "flex",
        flexDirection: "column",
        cursor: "pointer",
        position: "relative",
        transition: "all 0.2s cubic-bezier(0.4, 0, 0.2, 1)",
        transform: isHovered ? "translateY(-4px)" : "none",
        boxShadow: isPlaying
          ? "0 0 20px rgba(139, 124, 246, 0.35), 0 6px 18px rgba(0, 0, 0, 0.45)"
          : isHovered
          ? "0 10px 24px rgba(0, 0, 0, 0.4)"
          : isSelected
          ? "0 4px 18px rgba(243, 112, 30, 0.2)"
          : "0 2px 8px rgba(0, 0, 0, 0.15)",
      }}
    >
      {/* 1. Thumbnail (Biggest element, passport aspect ratio) */}
      <div
        style={{
          position: "relative",
          width: "100%",
          aspectRatio: "1 / 1",
          borderRadius: "8px",
          overflow: "hidden",
          backgroundColor: "rgba(255, 255, 255, 0.04)",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
        }}
      >
        {rec.cover_art_url ? (
          <img
            src={rec.cover_art_url}
            alt={rec.title}
            style={{ width: "100%", height: "100%", objectFit: "cover", display: "block" }}
            loading="lazy"
          />
        ) : (
          <div
            style={{
              width: "100%",
              height: "100%",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              background: "linear-gradient(135deg, rgba(243, 112, 30, 0.2), rgba(232, 216, 201, 0.15))",
            }}
          >
            <Music2 size={36} color="var(--text-dim)" />
          </div>
        )}

        {/* Downloaded Symbol (tick in circle icon) on top right of thumbnail */}
        {isDownloaded && (
          <div
            style={{
              position: "absolute",
              top: "7px",
              right: "7px",
              backgroundColor: "rgba(0, 0, 0, 0.88)",
              borderRadius: "50%",
              padding: "3px",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              boxShadow: "0 2px 8px rgba(0,0,0,0.6)",
              zIndex: 2,
            }}
            title="Downloaded & in your local library"
          >
            <CheckCircle2 size={16} color="var(--accent-secondary)" />
          </div>
        )}

        {/* Now Playing Animated Equalizer Badge on top left of thumbnail */}
        {isPlaying && (
          <div
            style={{
              position: "absolute",
              top: "7px",
              left: "7px",
              backgroundColor: "rgba(139, 124, 246, 0.95)",
              borderRadius: "20px",
              padding: "2px 8px",
              display: "flex",
              alignItems: "center",
              gap: "5px",
              boxShadow: "0 2px 8px rgba(0,0,0,0.6)",
              zIndex: 2,
            }}
            title="Now Playing"
          >
            <div className="discovery-eq-container" style={{ height: "11px" }}>
              <div className="discovery-eq-bar" style={{ width: "2.5px" }} />
              <div className="discovery-eq-bar" style={{ width: "2.5px" }} />
              <div className="discovery-eq-bar" style={{ width: "2.5px" }} />
            </div>
            <span style={{ fontSize: "0.64rem", fontWeight: 700, color: "#e8d8c9", letterSpacing: "0.4px" }}>
              PLAYING
            </span>
          </div>
        )}

        {/* Hover Play Button (Fades in opposite the queue control) */}
        {/* On hover when playing, play icon becomes pause and clicking stops music */}
        <button
          type="button"
          onClick={handlePlayBtnClick}
          disabled={isLoading}
          style={{
            position: "absolute",
            bottom: "8px",
            right: "8px",
            width: "38px",
            height: "38px",
            borderRadius: "50%",
            backgroundColor: isPlaying ? "var(--accent-secondary)" : "var(--accent)",
            border: "none",
            color: "#e8d8c9",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            boxShadow: "0 4px 14px rgba(0, 0, 0, 0.6)",
            cursor: "pointer",
            opacity: isHovered || isPlaying || isLoading ? 1 : 0,
            transform: isHovered || isPlaying || isLoading ? "scale(1)" : "scale(0.85)",
            transition: "opacity 0.2s ease, transform 0.2s ease, background-color 0.2s ease",
            zIndex: 3,
          }}
          title={isPlaying ? "Pause / Stop playback" : "Play"}
        >
          {isLoading ? (
            <RefreshCw size={16} className="spin-animation" />
          ) : isPlaying ? (
            <Pause size={17} fill="#e8d8c9" />
          ) : (
            <Play size={17} fill="#e8d8c9" style={{ marginLeft: "2px" }} />
          )}
        </button>

        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onEnqueue?.(rec);
          }}
          style={{
            position: "absolute",
            bottom: "8px",
            left: "8px",
            width: "38px",
            height: "38px",
            borderRadius: "50%",
            backgroundColor: isQueued ? "rgba(139,124,246,0.92)" : "rgba(0,0,0,0.72)",
            border: isQueued ? "1px solid rgba(232,216,201,0.45)" : "1px solid rgba(255,255,255,0.16)",
            color: isQueued ? "#e8d8c9" : "var(--text-muted)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            cursor: "pointer",
            opacity: isHovered || isQueued ? 1 : 0,
            transform: isHovered || isQueued ? "scale(1)" : "scale(0.85)",
            transition: "opacity 0.2s ease, transform 0.2s ease, background-color 0.2s ease",
            zIndex: 3,
          }}
          title={isQueued ? "Already in queue" : "Add to queue"}
        >
          {isQueued ? <Check size={17} /> : <ListPlus size={17} />}
        </button>
      </div>

      {/* 2. Name (Below thumbnail, bold, smaller than thumbnail) */}
      <div
        style={{
          marginTop: "10px",
          fontWeight: 700,
          fontSize: "0.88rem",
          color: isPlaying ? "var(--accent-secondary)" : "#e8d8c9",
          whiteSpace: "nowrap",
          overflow: "hidden",
          textOverflow: "ellipsis",
          lineHeight: "1.3",
        }}
        title={rec.title}
      >
        {rec.title}
      </div>

      {/* 3. Artist (Below name, even smaller than title, not bold, clickable to search) */}
      <div
        style={{
          marginTop: "3px",
          fontWeight: 400,
          fontSize: "0.78rem",
          color: "var(--text-muted)",
          whiteSpace: "nowrap",
          overflow: "hidden",
          textOverflow: "ellipsis",
          cursor: "pointer",
          transition: "color 0.15s ease",
        }}
        title={`Search other songs by ${rec.artist}`}
        onClick={(e) => {
          e.stopPropagation();
          onArtistClick(rec.artist);
        }}
        onMouseEnter={(e) => {
          (e.currentTarget as HTMLElement).style.color = "var(--accent-light)";
          (e.currentTarget as HTMLElement).style.textDecoration = "underline";
        }}
        onMouseLeave={(e) => {
          (e.currentTarget as HTMLElement).style.color = "var(--text-muted)";
          (e.currentTarget as HTMLElement).style.textDecoration = "none";
        }}
      >
        {rec.artist}
      </div>

      {/* 4. Footer Actions (Download / Wishlist) */}
      <div
        style={{
          marginTop: "auto",
          paddingTop: "8px",
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          borderTop: "1px solid rgba(255, 255, 255, 0.05)",
        }}
      >
        {isDownloaded ? (
          <span
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: "4px",
              fontSize: "0.72rem",
              color: "var(--accent-secondary)",
              fontWeight: 500,
            }}
          >
            <CheckCircle2 size={13} />
            <span>Downloaded</span>
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
              color: "var(--accent-secondary)",
            }}
            title={activeDownload.status === "QUEUED" ? "Download queued..." : `Downloading ${downloadPercent}%`}
          >
            <RefreshCw size={11} className="spin-animation" />
            <span>{activeDownload.status === "QUEUED" ? "Queued" : `${downloadPercent}%`}</span>
          </div>
        ) : (
          <button
            type="button"
            className="btn btn-secondary"
            style={{
              padding: "3px 8px",
              fontSize: "0.72rem",
              display: "inline-flex",
              alignItems: "center",
              gap: "4px",
            }}
            onClick={(e) => {
              e.stopPropagation();
              onDownload(rec.artist, rec.title);
            }}
            title="Download song directly to library"
          >
            <DownloadCloud size={12} />
            <span>Download</span>
          </button>
        )}

        <div style={{ display: "inline-flex", alignItems: "center", gap: "2px" }}>
          {isLiked && (
            <span
              title="Liked song"
              aria-label="Liked song"
              style={{
                color: "#ff5c8a",
                padding: "4px",
                display: "inline-flex",
                alignItems: "center",
              }}
            >
              <Heart size={16} fill="currentColor" />
            </span>
          )}

          <button
            type="button"
            style={{
              background: "none",
              border: "none",
              cursor: "pointer",
              color: isInPlaylist ? "var(--accent-secondary)" : "var(--text-dim)",
              padding: "4px",
              display: "flex",
              alignItems: "center",
              transition: "color 0.15s ease",
            }}
            onClick={(e) => {
              e.stopPropagation();
              if (onAddToPlaylist) {
                onAddToPlaylist(rec);
              } else {
                onAddToWishlist(rec);
              }
            }}
            title={isInPlaylist ? "In playlist (click to manage)" : "Add to playlist"}
          >
            {isInPlaylist ? (
              <BookmarkCheck size={16} color="var(--accent-secondary)" />
            ) : (
              <Bookmark size={16} />
            )}
          </button>
        </div>
      </div>
    </div>
  );
};

interface MixItem {
  id: string;
  name: string;
  subtitle: string;
  bgGradient: string;
  accentColor: string;
  searchQuery?: string;
  playlistId?: string;
  cover_art_url?: string;
}

const MixCard: React.FC<{ mix: MixItem; onPlay: () => void; onOpen?: () => void }> = ({
  mix,
  onPlay,
  onOpen,
}) => {
  const [isHovered, setIsHovered] = useState(false);

  return (
    <div
      className="mix-card"
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      onClick={onOpen || onPlay}
      style={{
        backgroundColor: "var(--bg-card)",
        border: isHovered ? "1px solid rgba(255, 255, 255, 0.22)" : "1px solid var(--border)",
        borderRadius: "12px",
        padding: "10px",
        display: "flex",
        flexDirection: "column",
        cursor: "pointer",
        position: "relative",
        transition: "all 0.2s cubic-bezier(0.4, 0, 0.2, 1)",
        transform: isHovered ? "translateY(-4px)" : "none",
        boxShadow: isHovered ? "0 10px 24px rgba(0, 0, 0, 0.4)" : "0 2px 8px rgba(0, 0, 0, 0.15)",
        width: "100%",
        height: "100%",
      }}
    >
      <div
        style={{
          position: "relative",
          width: "100%",
          aspectRatio: "1 / 1",
          borderRadius: "8px",
          overflow: "hidden",
          background: "var(--bg-card)",
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          padding: "16px",
          textAlign: "center",
        }}
      >
          <img
            src={mix.cover_art_url || "/kaze-playlist-default.svg"}
            alt={mix.name}
            style={{ position: "absolute", inset: 0, width: "100%", height: "100%", objectFit: "cover" }}
          />

        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onPlay();
          }}
          style={{
            position: "absolute",
            bottom: "8px",
            left: "8px",
            width: "38px",
            height: "38px",
            borderRadius: "50%",
            backgroundColor: "var(--accent)",
            border: "none",
            color: "#e8d8c9",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            boxShadow: "0 4px 14px rgba(0, 0, 0, 0.6)",
            cursor: "pointer",
            opacity: isHovered ? 1 : 0,
            transform: isHovered ? "scale(1)" : "scale(0.85)",
            transition: "opacity 0.2s ease, transform 0.2s ease, background-color 0.2s ease",
            zIndex: 3,
          }}
          title={`Play ${mix.name}`}
        >
          <Play size={17} fill="#e8d8c9" style={{ marginLeft: "2px" }} />
        </button>
      </div>

      <div
        style={{
          marginTop: "10px",
          fontWeight: 700,
          fontSize: "0.88rem",
          color: "#e8d8c9",
          whiteSpace: "nowrap",
          overflow: "hidden",
          textOverflow: "ellipsis",
          lineHeight: "1.3",
        }}
        title={mix.name}
      >
        {mix.name}
      </div>

      <div
        style={{
          marginTop: "3px",
          fontWeight: 400,
          fontSize: "0.76rem",
          color: "var(--text-muted)",
          whiteSpace: "nowrap",
          overflow: "hidden",
          textOverflow: "ellipsis",
          lineHeight: "1.3",
        }}
        title={mix.subtitle}
      >
        {mix.subtitle}
      </div>
    </div>
  );
};

interface ChartItem {
  id: string;
  title: string;
  subtitle: string;
  chartNumber: string;
  region: string;
  bgGradient: string;
  badgeBg: string;
}

const ChartCard: React.FC<{ chart: ChartItem; onPlay: () => void; onOpen?: () => void }> = ({
  chart,
  onPlay,
  onOpen,
}) => {
  const [isHovered, setIsHovered] = useState(false);

  return (
    <div
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      onClick={onOpen || onPlay}
      style={{
        backgroundColor: "var(--bg-card)",
        border: isHovered ? "1px solid rgba(255, 255, 255, 0.22)" : "1px solid var(--border)",
        borderRadius: "12px",
        padding: "10px",
        display: "flex",
        flexDirection: "column",
        cursor: "pointer",
        position: "relative",
        transition: "all 0.2s cubic-bezier(0.4, 0, 0.2, 1)",
        transform: isHovered ? "translateY(-4px)" : "none",
        boxShadow: isHovered ? "0 10px 24px rgba(0, 0, 0, 0.4)" : "0 2px 8px rgba(0, 0, 0, 0.15)",
        width: "100%",
        height: "100%",
      }}
    >
      <div
        style={{
          position: "relative",
          width: "100%",
          aspectRatio: "1 / 1",
          borderRadius: "8px",
          overflow: "hidden",
          background: chart.bgGradient,
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          padding: "12px",
          textAlign: "center",
        }}
      >
        <div
          style={{
            backgroundColor: "rgba(0,0,0,0.6)",
            padding: "3px 8px",
            borderRadius: "6px",
            fontSize: "0.68rem",
            fontWeight: 800,
            letterSpacing: "1.5px",
            color: "#e8d8c9",
            marginBottom: "6px",
          }}
        >
          TOP 50
        </div>
        <div
          style={{
            fontSize: "1.05rem",
            fontWeight: 900,
            letterSpacing: "0.5px",
            color: "#e8d8c9",
            textShadow: "0 2px 8px rgba(0,0,0,0.6)",
          }}
        >
          {chart.chartNumber}
        </div>

        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onPlay();
          }}
          style={{
            position: "absolute",
            bottom: "8px",
            left: "8px",
            width: "38px",
            height: "38px",
            borderRadius: "50%",
            backgroundColor: chart.badgeBg || "var(--accent)",
            border: "none",
            color: "#e8d8c9",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            boxShadow: "0 4px 14px rgba(0, 0, 0, 0.6)",
            cursor: "pointer",
            opacity: isHovered ? 1 : 0,
            transform: isHovered ? "scale(1)" : "scale(0.85)",
            transition: "opacity 0.2s ease, transform 0.2s ease, background-color 0.2s ease",
            zIndex: 3,
          }}
          title={`Play ${chart.title}`}
        >
          <Play size={17} fill="#e8d8c9" style={{ marginLeft: "2px" }} />
        </button>
      </div>

      <div
        style={{
          marginTop: "10px",
          fontWeight: 700,
          fontSize: "0.88rem",
          color: "#e8d8c9",
          whiteSpace: "nowrap",
          overflow: "hidden",
          textOverflow: "ellipsis",
          lineHeight: "1.3",
        }}
        title={chart.title}
      >
        {chart.title}
      </div>

      <div
        style={{
          marginTop: "3px",
          fontWeight: 400,
          fontSize: "0.76rem",
          color: "var(--text-muted)",
          whiteSpace: "nowrap",
          overflow: "hidden",
          textOverflow: "ellipsis",
          lineHeight: "1.3",
        }}
        title={chart.subtitle}
      >
        {chart.subtitle}
      </div>
    </div>
  );
};

interface DiscoveryViewProps {
  recommendations: DiscoveryRecommendation[];
  worldTrending: DiscoveryRecommendation[];
  localTracks: Track[];
  playlists?: Playlist[];
  onPlayPlaylist?: (id: string) => void;
  onAddToWishlist: (rec: DiscoveryRecommendation) => void;
  onSearchDirect: (artist: string, title: string) => void;
  onRefresh?: (force?: boolean) => Promise<void>;
  onPlayOnlineTrack: (rec: DiscoveryRecommendation) => void;
  onStopTrack?: (rec: DiscoveryRecommendation) => void;
  activeOnlineTrackId?: string | null;
  isOnlinePlaying?: boolean;
  loadingTrackId?: string | null;
  currentLocalTrack?: Track | null;
  isLocalPlaying?: boolean;
  onOpenCollection?: (collection: CollectionData) => void;
  onPlayCollection?: (collection: CollectionData) => Promise<void>;
  downloads?: DownloadTask[];
  downloadTargets?: Record<string, { title: string; artist: string }>;
  searchQuery?: string;
  setSearchQuery?: (q: string) => void;
  searchResults?: DiscoveryRecommendation[] | null;
  setSearchResults?: (res: DiscoveryRecommendation[] | null) => void;
  isSearchingOnline?: boolean;
  onSearchOnline?: (queryOverride?: string) => Promise<void>;
  onClearSearch?: () => void;
  trackPlaylistMap?: Record<string, string[]>;
  onAddToPlaylist?: (rec: DiscoveryRecommendation) => void;
  onGoToLibrary?: () => void;
  queuedTrackIds?: Set<string>;
  onEnqueueTrack?: (rec: DiscoveryRecommendation) => void;
  likedTrackIds?: Set<string>;
}

export const DiscoveryView: React.FC<DiscoveryViewProps> = ({
  recommendations,
  worldTrending,
  localTracks,
  playlists = [],
  onPlayPlaylist,
  onAddToWishlist,
  onSearchDirect,
  onRefresh: _onRefresh,
  onPlayOnlineTrack,
  onStopTrack,
  activeOnlineTrackId,
  isOnlinePlaying = false,
  loadingTrackId = null,
  currentLocalTrack,
  isLocalPlaying = false,
  onOpenCollection,
  onPlayCollection,
  downloads = [],
  downloadTargets,
  searchQuery: propSearchQuery,
  setSearchQuery: propSetSearchQuery,
  searchResults: propSearchResults,
  setSearchResults: propSetSearchResults,
  isSearchingOnline: propIsSearchingOnline,
  onSearchOnline: propOnSearchOnline,
  onClearSearch: propOnClearSearch,
  trackPlaylistMap,
  onAddToPlaylist,
  onGoToLibrary,
  queuedTrackIds,
  onEnqueueTrack,
  likedTrackIds,
}) => {
  const [forYouFilter, setForYouFilter] = useState<"ALL" | "GENRE" | "SIMILAR">("ALL");
  const [visibleCount, setVisibleCount] = useState(15);
  const [isRefreshingTrending, setIsRefreshingTrending] = useState(false);
  const [isOnline, setIsOnline] = useState(
    typeof navigator !== "undefined" ? navigator.onLine : true
  );

  const handleRefreshTrending = async () => {
    if (isRefreshingTrending || !_onRefresh) return;
    setIsRefreshingTrending(true);
    try {
      await _onRefresh(true);
      setVisibleCount(15);
      if (songsScrollRef.current) {
        songsScrollRef.current.scrollLeft = 0;
      }
    } catch (err) {
      console.error("Failed to refresh trending:", err);
    } finally {
      setIsRefreshingTrending(false);
    }
  };

  const playlistTrackIds = useMemo(
    () => new Set(Object.entries(trackPlaylistMap || {})
      .filter(([, playlistIds]) => playlistIds.length > 0)
      .map(([trackId]) => trackId)),
    [trackPlaylistMap]
  );
  const { forYou } = useMemo(() => splitDiscoveryRecommendations({
    recommendations,
    playlistTrackIds,
    likedTrackIds: likedTrackIds || new Set<string>(),
    downloads,
    downloadTargets,
    localTracks,
  }), [recommendations, playlistTrackIds, likedTrackIds, downloads, downloadTargets, localTracks]);
  const trending = worldTrending;

  const handleSongsScroll = () => {
    const el = songsScrollRef.current;
    if (!el) return;
    if (el.scrollLeft + el.clientWidth >= el.scrollWidth - 350) {
      setVisibleCount((prev) => Math.min(prev + 10, trending.length));
    }
  };

  useEffect(() => {
    const handleOnline = () => {
      setIsOnline(true);
      if (_onRefresh) _onRefresh();
    };
    const handleOffline = () => {
      setIsOnline(false);
    };
    window.addEventListener("online", handleOnline);
    window.addEventListener("offline", handleOffline);
    return () => {
      window.removeEventListener("online", handleOnline);
      window.removeEventListener("offline", handleOffline);
    };
  }, [_onRefresh]);

  // Online Search State (fallback if not controlled by parent)
  const [internalSearchQuery, setInternalSearchQuery] = useState("");
  const [, setInternalIsSearching] = useState(false);
  const [internalSearchResults, setInternalSearchResults] = useState<DiscoveryRecommendation[] | null>(null);

  const searchQuery = propSearchQuery !== undefined ? propSearchQuery : internalSearchQuery;
  const setSearchQuery = propSetSearchQuery || setInternalSearchQuery;
  const searchResults = propSearchResults !== undefined ? propSearchResults : internalSearchResults;
  const setSearchResults = propSetSearchResults || setInternalSearchResults;

  const handleSearchOnline = async (queryOverride?: string) => {
    if (propOnSearchOnline) {
      await propOnSearchOnline(queryOverride);
      return;
    }
    const q = (typeof queryOverride === "string" ? queryOverride : searchQuery).trim();
    if (!q) {
      setSearchResults(null);
      return;
    }
    if (propIsSearchingOnline === undefined) setInternalIsSearching(true);
    try {
      const res = await executeQuery({
        query: "SearchOnlineMusic",
        payload: { query: q, limit: 35 },
      });
      console.log("[DiscoveryView] SearchOnlineMusic response:", res);

      let incoming: DiscoveryRecommendation[] = [];
      if (res && Array.isArray(res.data)) {
        incoming = res.data;
      }

      // Filter to relevant results matching the search query
      const qLower = q.toLowerCase();
      const qTokens = qLower.split(/\s+/).filter((t) => t.length > 1);

      const relevant = incoming.filter((r) => {
        const titleLower = r.title.toLowerCase();
        const artistLower = r.artist.toLowerCase();
        const albumLower = r.album ? r.album.toLowerCase() : "";
        if (titleLower.includes(qLower) || artistLower.includes(qLower) || albumLower.includes(qLower)) {
          return true;
        }
        return qTokens.some((tok) => titleLower.includes(tok) || artistLower.includes(tok));
      });

      setSearchResults(relevant.length > 0 ? relevant : incoming);
    } catch (err) {
      console.error("Online music search failed:", err);
      setSearchResults([]);
    } finally {
      if (propIsSearchingOnline === undefined) setInternalIsSearching(false);
    }
  };

  const handleClearSearch = () => {
    if (propOnClearSearch) {
      propOnClearSearch();
      return;
    }
    setSearchQuery("");
    setSearchResults(null);
  };

  const forYouGenreCount = useMemo(
    () =>
      forYou.filter((r) => {
        const s = r.recommendation_reason.toLowerCase();
        return s.includes("genre") || s.includes("popular in") || s.includes("trending in");
      }).length,
    [forYou]
  );

  const forYouSimilarCount = useMemo(
    () =>
      forYou.filter((r) => {
        const s = r.recommendation_reason.toLowerCase();
        return s.includes("similar") || s.includes("listening") || s.includes("library artist");
      }).length,
    [forYou]
  );

  const filteredForYou = useMemo(() => {
    if (forYouFilter === "GENRE") {
      return forYou.filter((r) => {
        const s = r.recommendation_reason.toLowerCase();
        return s.includes("genre") || s.includes("popular in") || s.includes("trending in");
      });
    }
    if (forYouFilter === "SIMILAR") {
      return forYou.filter((r) => {
        const s = r.recommendation_reason.toLowerCase();
        return s.includes("similar") || s.includes("listening") || s.includes("library artist");
      });
    }
    return forYou;
  }, [forYou, forYouFilter]);

  const handleForYouFilterChange = (nextFilter: "ALL" | "GENRE" | "SIMILAR") => {
    setForYouFilter(nextFilter);
    if (forYouScrollRef.current) {
      forYouScrollRef.current.scrollLeft = 0;
    }
  };

  const songsScrollRef = useRef<HTMLDivElement>(null);
  const forYouScrollRef = useRef<HTMLDivElement>(null);
  const mixesScrollRef = useRef<HTMLDivElement>(null);
  const chartsScrollRef = useRef<HTMLDivElement>(null);
  const discoveryRef = useRef<HTMLDivElement>(null);

  useLayoutEffect(() => {
    const discovery = discoveryRef.current;
    const scroller = discovery?.closest<HTMLElement>(".main-content");
    if (!discovery || !scroller) return;

    // Pin to the hero's original position, including wrapped search suggestions.
    const updateBackdropPosition = () => {
      const top = discovery.getBoundingClientRect().top
        - scroller.getBoundingClientRect().top + scroller.scrollTop;
      discovery.style.setProperty("--backdrop-top", `${top}px`);
    };
    updateBackdropPosition();
    const observer = new ResizeObserver(updateBackdropPosition);
    observer.observe(scroller);
    const search = scroller.querySelector(".global-search");
    if (search) observer.observe(search);
    return () => observer.disconnect();
  }, [isOnline, searchResults !== null]);

  const handleScroll = (ref: React.RefObject<HTMLDivElement | null>, direction: "left" | "right") => {
    if (ref.current) {
      const amount = direction === "left" ? -500 : 500;
      ref.current.scrollBy({ left: amount, behavior: "smooth" });
    }
  };

  const seenMixNames = new Set<string>();
  const userSmartMixes: MixItem[] = (playlists || [])
    .filter((p) => {
      if (p.is_smart_mix !== 1) return false;
      const key = p.name.trim().toLowerCase();
      if (seenMixNames.has(key)) return false;
      seenMixNames.add(key);
      return true;
    })
    .map((p, idx) => {
      const gradients = [
        "linear-gradient(135deg, #4b607f 0%, #8b7cf6 100%)",
        "linear-gradient(135deg, #2a211b 0%, #f3701e 100%)",
        "linear-gradient(135deg, #1a1714 0%, #4b607f 100%)",
        "linear-gradient(135deg, #2a211b 0%, #8b7cf6 100%)",
        "linear-gradient(135deg, #f3701e 0%, #8b7cf6 100%)",
      ];
      return {
        id: p.id,
        name: p.name,
        subtitle: p.description || p.generation_reason || `${p.track_count || 0} tracks`,
        bgGradient: gradients[idx % gradients.length],
        accentColor: "var(--accent-light)",
        playlistId: p.id,
        cover_art_url: p.cover_art_url,
      };
    });

  // Every mix shown here is generated for the active account from its library
  // and listening profile. Generic search presets do not belong in For You.
  const allMixes = userSmartMixes;

  const topCharts: ChartItem[] = [
    {
      id: "chart_top50_global",
      title: "Top 50 - Worldwide",
      subtitle: "Popular across 12 country charts",
      chartNumber: "WORLD",
      region: "Worldwide",
      bgGradient: "linear-gradient(135deg, #1a1714 0%, #4b607f 55%, #8b7cf6 100%)",
      badgeBg: "#8b7cf6",
    },
    {
      id: "chart_top50_india",
      title: "Top 50 - India",
      subtitle: "Current top songs in the India chart",
      chartNumber: "INDIA",
      region: "India",
      bgGradient: "linear-gradient(135deg, #2a211b 0%, #f3701e 100%)",
      badgeBg: "#f3701e",
    },
    {
      id: "chart_top50_usa",
      title: "Top 50 - USA",
      subtitle: "Current top songs in the US chart",
      chartNumber: "USA",
      region: "United States",
      bgGradient: "linear-gradient(135deg, #1a1714 0%, #4b607f 58%, #8b7cf6 100%)",
      badgeBg: "#4b607f",
    },
    {
      id: "chart_top50_uk",
      title: "Top 50 - UK",
      subtitle: "Current top songs in the UK chart",
      chartNumber: "UK",
      region: "United Kingdom",
      bgGradient: "linear-gradient(135deg, #2a211b 0%, #8b7cf6 62%, #f3701e 100%)",
      badgeBg: "#8b7cf6",
    },
    {
      id: "chart_top50_japan",
      title: "Top 50 - Japan",
      subtitle: "Current top songs in the Japan chart",
      chartNumber: "JAPAN",
      region: "Japan",
      bgGradient: "linear-gradient(135deg, #2a211b 0%, #f3701e 58%, #8b7cf6 100%)",
      badgeBg: "#f3701e",
    },
    {
      id: "chart_top50_brazil",
      title: "Top 50 - Brazil",
      subtitle: "Current top songs in the Brazil chart",
      chartNumber: "BRAZIL",
      region: "Brazil",
      bgGradient: "linear-gradient(135deg, #1a1714 0%, #4b607f 58%, #f3701e 100%)",
      badgeBg: "#8b7cf6",
    },
  ];

  const handleOpenMix = (mix: MixItem) => {
    if (onOpenCollection) {
      onOpenCollection({
        id: mix.id,
        type: "mix",
        title: mix.name,
        subtitle: mix.subtitle,
        tag: "SMART MIX",
        bgGradient: mix.bgGradient,
        accentColor: mix.accentColor,
        searchQuery: mix.searchQuery,
        playlistId: mix.playlistId,
        cover_art_url: mix.cover_art_url,
      });
    } else {
      handlePlayMixItem(mix);
    }
  };

  const chartToCollection = (chart: ChartItem): CollectionData => ({
    id: chart.id,
    type: "chart",
    title: chart.title,
    subtitle: chart.subtitle,
    tag: `TOP CHART • ${chart.region.toUpperCase()}`,
    bgGradient: chart.bgGradient,
    accentColor: chart.badgeBg,
  });

  const handleOpenChart = (chart: ChartItem) => {
    if (onOpenCollection) {
      onOpenCollection(chartToCollection(chart));
    } else {
      handlePlayChartItem(chart);
    }
  };

  const handlePlayMixItem = (mix: MixItem) => {
    if (mix.playlistId && onPlayPlaylist) {
      onPlayPlaylist(mix.playlistId);
    } else if (mix.searchQuery) {
      setSearchQuery(mix.searchQuery);
      handleSearchOnline(mix.searchQuery);
    }
  };

  const handlePlayChartItem = (chart: ChartItem) => {
    if (onPlayCollection) {
      void onPlayCollection(chartToCollection(chart));
      return;
    }
    onOpenCollection?.(chartToCollection(chart));
  };

  const renderSongCard = (rec: DiscoveryRecommendation) => {
    const isLoading = loadingTrackId === rec.external_track_id ||
      (!!rec.matched_local_track_id && loadingTrackId === rec.matched_local_track_id);
    const hasRealLocalMatch = !!rec.matched_local_track_id &&
      !rec.matched_local_track_id.startsWith("itunes:") &&
      !rec.matched_local_track_id.startsWith("online:");
    const isPlayingLocal = isLocalPlaying && !!currentLocalTrack &&
      currentLocalTrack.format !== "online" &&
      !currentLocalTrack.file_path.startsWith("online://") &&
      ((hasRealLocalMatch && rec.matched_local_track_id === currentLocalTrack.id) ||
        rec.external_track_id === currentLocalTrack.id ||
        (rec.title.toLowerCase().trim() === currentLocalTrack.title.toLowerCase().trim() &&
          !!currentLocalTrack.artist_name &&
          rec.artist.toLowerCase().trim() === currentLocalTrack.artist_name.toLowerCase().trim()));
    const isPlayingOnline = activeOnlineTrackId === rec.external_track_id && isOnlinePlaying;
    const isPlaying = isPlayingLocal || isPlayingOnline;

    return (
      <div key={rec.external_track_id} style={{ flex: "0 0 174px", width: "174px" }}>
        <DiscoveryTrackCard
          rec={rec}
          isPlaying={isPlaying}
          isLoading={isLoading}
          isSelected={isPlaying || isLoading || activeOnlineTrackId === rec.external_track_id}
          onPlay={onPlayOnlineTrack}
          onStop={onStopTrack || onPlayOnlineTrack}
          onArtistClick={(artist) => {
            setSearchQuery(artist);
            handleSearchOnline(artist);
          }}
          onAddToWishlist={onAddToWishlist}
          onDownload={onSearchDirect}
          downloads={downloads}
          trackPlaylistMap={trackPlaylistMap}
          onAddToPlaylist={onAddToPlaylist}
          isQueued={queuedTrackIds?.has(rec.matched_local_track_id || rec.external_track_id)}
          onEnqueue={onEnqueueTrack}
          isLiked={likedTrackIds?.has(rec.matched_local_track_id || rec.external_track_id)}
        />
      </div>
    );
  };

  if (!isOnline) {
    return (
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          minHeight: "65vh",
          textAlign: "center",
          padding: "40px 20px",
        }}
      >
        <div
          style={{
            backgroundColor: "rgba(24, 24, 27, 0.95)",
            border: "1px solid rgba(239, 68, 68, 0.35)",
            borderRadius: "16px",
            padding: "48px 36px",
            textAlign: "center",
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            gap: "12px",
            maxWidth: "460px",
            width: "100%",
            boxShadow: "0 8px 30px rgba(0, 0, 0, 0.4)",
          }}
        >
          <div
            style={{
              width: "56px",
              height: "56px",
              borderRadius: "50%",
              backgroundColor: "rgba(239, 68, 68, 0.12)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              marginBottom: "4px",
            }}
          >
            <WifiOff size={28} color="#ef4444" />
          </div>
          <h2 style={{ fontSize: "1.25rem", fontWeight: 700, color: "#e8d8c9", margin: 0 }}>
            No internet connection
          </h2>
          <p style={{ fontSize: "0.92rem", color: "var(--text-muted)", margin: 0, maxWidth: "420px", lineHeight: 1.5 }}>
            No internet connection. Listen to local songs.
          </p>
          {onGoToLibrary && (
            <button
              type="button"
              className="btn btn-primary"
              onClick={onGoToLibrary}
              style={{
                marginTop: "12px",
                padding: "9px 20px",
                fontSize: "0.88rem",
                borderRadius: "10px",
                display: "inline-flex",
                alignItems: "center",
                gap: "8px",
                fontWeight: 600,
              }}
            >
              <Music size={16} />
              <span>Go to Downloaded Songs</span>
            </button>
          )}
        </div>
      </div>
    );
  }

  return (
    <div
      ref={discoveryRef}
      className="discovery-view"
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "28px",
        paddingBottom: "40px",
      }}
    >

      {/* SEPARATE SEARCH SECTION (Displayed exclusively when search results are active) */}
      {searchResults !== null ? (
        <OnlineSearchResultsSection
          searchQuery={searchQuery}
          searchResults={searchResults}
          onClearSearch={handleClearSearch}
          onPlayOnlineTrack={onPlayOnlineTrack}
          onStopTrack={onStopTrack}
          activeOnlineTrackId={activeOnlineTrackId}
          isOnlinePlaying={isOnlinePlaying}
          isOnlineLoading={loadingTrackId !== null}
          loadingTrackId={loadingTrackId}
          currentLocalTrack={currentLocalTrack}
          isLocalPlaying={isLocalPlaying}
          onAddToPlaylist={onAddToPlaylist}
          onAddToWishlist={onAddToWishlist}
          onSearchDirect={onSearchDirect}
          downloads={downloads}
          downloadTargets={downloadTargets}
          trackPlaylistMap={trackPlaylistMap}
          playlists={playlists}
          onSelectArtist={(artist) => {
            setSearchQuery(artist);
            handleSearchOnline(artist);
          }}
        />
      ) : (
        <>
          <section className="discovery-hero" aria-label="Welcome to Kaze">
            <div className="discovery-hero-copy">
              <p className="hero-eyebrow">A DIFFERENT KIND OF FEELING</p>
              <h1>Let<br />the music<br />take you<br />somewhere else.</h1>
              <p className="hero-japanese" lang="ja">どこかへ、音楽と。</p>
            </div>
            <div className="hero-signature" aria-hidden="true"><span lang="ja">音で、呼吸する。</span></div>
          </section>
          <div className="discovery-scroll-content">
          {/* SECTION 1: Trending Songs */}
      <div>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            marginBottom: "12px",
          }}
        >
          <div>
            <h2
              style={{
                fontSize: "1.18rem",
                fontWeight: 700,
                color: "#e8d8c9",
                display: "flex",
                alignItems: "center",
                gap: "8px",
              }}
            >
              <Music2 size={18} color="var(--accent-light)" />
              <span>Trending Songs</span>
            </h2>
            <p style={{ fontSize: "0.8rem", color: "var(--text-muted)", marginTop: "2px" }}>
              Popular songs across world charts
            </p>
          </div>

          <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
            {_onRefresh && (
              <button
                type="button"
                className="btn btn-secondary"
                onClick={handleRefreshTrending}
                disabled={isRefreshingTrending}
                style={{
                  padding: "5px 12px",
                  fontSize: "0.82rem",
                  display: "inline-flex",
                  alignItems: "center",
                  gap: "6px",
                  borderRadius: "8px",
                }}
                title="Refresh trending songs"
              >
                <RefreshCw
                  size={14}
                  className={isRefreshingTrending ? "spin-animation" : ""}
                />
              </button>
            )}
            <button
              type="button"
              className="scroll-arrow-btn"
              onClick={() => handleScroll(songsScrollRef, "left")}
              title="Scroll left"
            >
              <ChevronLeft size={16} />
            </button>
            <button
              type="button"
              className="scroll-arrow-btn"
              onClick={() => handleScroll(songsScrollRef, "right")}
              title="Scroll right"
            >
              <ChevronRight size={16} />
            </button>
          </div>
        </div>

        {trending.length === 0 ? (
          <div
            className="content-card"
            style={{ textAlign: "center", padding: "45px 20px", color: "var(--text-dim)", borderStyle: "dashed" }}
          >
            <Sparkles size={36} color="var(--accent-light)" style={{ marginBottom: "10px" }} />
            <p style={{ fontSize: "1.05rem", fontWeight: 600, color: "var(--text-main)", marginBottom: "4px" }}>
              No trending songs found
            </p>
            <p style={{ fontSize: "0.85rem", color: "var(--text-muted)", maxWidth: "480px", margin: "0 auto" }}>
              Click Refresh to discover fresh trending tracks.
            </p>
          </div>
        ) : (
          <div
            ref={songsScrollRef}
            className="horizontal-scroll-row"
            onScroll={handleSongsScroll}
          >
            {trending.slice(0, visibleCount).map(renderSongCard)}
          </div>
        )}
      </div>

      {/* SECTION 2: For You (new songs based on listening taste) */}
      <section aria-labelledby="for-you-heading">
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "12px" }}>
          <div>
            <h2
              id="for-you-heading"
              style={{ fontSize: "1.18rem", fontWeight: 700, color: "#e8d8c9", display: "flex", alignItems: "center", gap: "8px" }}
            >
              <Sparkles size={18} color="var(--accent-secondary)" />
              <span>For You</span>
            </h2>
            <p style={{ fontSize: "0.8rem", color: "var(--text-muted)", marginTop: "2px" }}>
              Songs picked from your taste that you have not saved or downloaded
            </p>
          </div>
          {forYou.length > 0 && (
            <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
              <button type="button" className="scroll-arrow-btn" onClick={() => handleScroll(forYouScrollRef, "left")} title="Scroll For You left" aria-label="Scroll For You left">
                <ChevronLeft size={16} />
              </button>
              <button type="button" className="scroll-arrow-btn" onClick={() => handleScroll(forYouScrollRef, "right")} title="Scroll For You right" aria-label="Scroll For You right">
                <ChevronRight size={16} />
              </button>
            </div>
          )}
        </div>

        {forYou.length > 0 && (forYouGenreCount > 0 || forYouSimilarCount > 0) && (
          <div style={{ display: "flex", alignItems: "center", gap: "8px", flexWrap: "wrap", marginBottom: "14px" }}>
            <button
              type="button"
              onClick={() => handleForYouFilterChange("ALL")}
              className={`subtab-btn ${forYouFilter === "ALL" ? "active" : ""}`}
            >
              All For You ({forYou.length})
            </button>
            {forYouGenreCount > 0 && (
              <button
                type="button"
                onClick={() => handleForYouFilterChange("GENRE")}
                className={`subtab-btn ${forYouFilter === "GENRE" ? "active" : ""}`}
                style={{ display: "inline-flex", alignItems: "center", gap: "5px" }}
              >
                <Radio size={12} color="var(--accent-light)" />
                <span>Top Genres ({forYouGenreCount})</span>
              </button>
            )}
            {forYouSimilarCount > 0 && (
              <button
                type="button"
                onClick={() => handleForYouFilterChange("SIMILAR")}
                className={`subtab-btn ${forYouFilter === "SIMILAR" ? "active" : ""}`}
                style={{ display: "inline-flex", alignItems: "center", gap: "5px" }}
              >
                <Sparkles size={12} color="var(--accent-light)" />
                <span>Similar Artists ({forYouSimilarCount})</span>
              </button>
            )}
          </div>
        )}

        {forYou.length > 0 ? (
          filteredForYou.length === 0 ? (
            <div className="content-card" style={{ padding: "24px", color: "var(--text-muted)", borderStyle: "dashed", textAlign: "center" }}>
              No songs found matching this filter.
            </div>
          ) : (
            <div ref={forYouScrollRef} className="horizontal-scroll-row">
              {filteredForYou.map(renderSongCard)}
            </div>
          )
        ) : (
          <div className="content-card" style={{ padding: "24px", color: "var(--text-muted)", borderStyle: "dashed" }}>
            Play or add more music to your library to shape new recommendations.
          </div>
        )}
      </section>

      {/* SECTION 3: Mixes For You (Horizontal Scroll) */}
      <div>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            marginBottom: "12px",
          }}
        >
          <div>
            <h2
              style={{
                fontSize: "1.18rem",
                fontWeight: 700,
                color: "#e8d8c9",
                display: "flex",
                alignItems: "center",
                gap: "8px",
              }}
            >
              <Sparkles size={18} color="var(--accent-secondary)" />
              <span>Mixes For You</span>
            </h2>
            <p style={{ fontSize: "0.8rem", color: "var(--text-muted)", marginTop: "2px" }}>
              Personalized blends and custom smart mixes curated for your listening style
            </p>
          </div>

          <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
            <button
              type="button"
              className="scroll-arrow-btn"
              onClick={() => handleScroll(mixesScrollRef, "left")}
              title="Scroll left"
            >
              <ChevronLeft size={16} />
            </button>
            <button
              type="button"
              className="scroll-arrow-btn"
              onClick={() => handleScroll(mixesScrollRef, "right")}
              title="Scroll right"
            >
              <ChevronRight size={16} />
            </button>
          </div>
        </div>

        <div ref={mixesScrollRef} className="horizontal-scroll-row">
          {allMixes.length === 0 && (
            <div className="content-card" style={{ padding: "24px", color: "var(--text-muted)", borderStyle: "dashed" }}>
              Play more of your library to build mixes for your taste.
            </div>
          )}
          {allMixes.map((mix) => (
            <div key={mix.id} style={{ flex: "0 0 174px", width: "174px" }}>
              <MixCard
                mix={mix}
                onPlay={() => handlePlayMixItem(mix)}
                onOpen={() => handleOpenMix(mix)}
              />
            </div>
          ))}
        </div>
      </div>

      {/* SECTION 3: Top Charts (Horizontal Scroll) */}
      <div>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            marginBottom: "12px",
          }}
        >
          <div>
            <h2
              style={{
                fontSize: "1.18rem",
                fontWeight: 700,
                color: "#e8d8c9",
                display: "flex",
                alignItems: "center",
                gap: "8px",
              }}
            >
              <TrendingUp size={18} color="#f3701e" />
              <span>Top Charts</span>
            </h2>
            <p style={{ fontSize: "0.8rem", color: "var(--text-muted)", marginTop: "2px" }}>
              Current world and country top-song charts
            </p>
          </div>

          <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
            <button
              type="button"
              className="scroll-arrow-btn"
              onClick={() => handleScroll(chartsScrollRef, "left")}
              title="Scroll left"
            >
              <ChevronLeft size={16} />
            </button>
            <button
              type="button"
              className="scroll-arrow-btn"
              onClick={() => handleScroll(chartsScrollRef, "right")}
              title="Scroll right"
            >
              <ChevronRight size={16} />
            </button>
          </div>
        </div>

        <div ref={chartsScrollRef} className="horizontal-scroll-row">
          {topCharts.map((chart) => (
            <div key={chart.id} style={{ flex: "0 0 174px", width: "174px" }}>
              <ChartCard
                chart={chart}
                onPlay={() => handlePlayChartItem(chart)}
                onOpen={() => handleOpenChart(chart)}
              />
            </div>
          ))}
        </div>
      </div>
          </div>
        </>
      )}
    </div>
  );
};
