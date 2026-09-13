import React, { useState, useRef, useEffect } from "react";
import {
  DownloadCloud,
  Sparkles,
  Bookmark,
  BookmarkCheck,
  WifiOff,
  Music,
  RefreshCw,
  Flame,
  Radio,
  Music2,
  Play,
  Pause,
  X,
  Globe,
  CheckCircle2,
  ChevronLeft,
  ChevronRight,
  TrendingUp,
} from "lucide-react";
import { DiscoveryRecommendation, Track, Playlist, DownloadTask } from "../../types";
import { executeQuery } from "../../services/api";
import { CollectionData } from "./CollectionDetailView";

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

  const isDownloaded =
    rec.provider === "library" ||
    rec.match_status === "EXACT_MATCH" ||
    !!rec.matched_local_track_id ||
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
          ? "rgba(16, 185, 129, 0.09)"
          : isSelected
          ? "rgba(99, 102, 241, 0.12)"
          : "var(--bg-card)",
        border: isPlaying
          ? "1.5px solid #10b981"
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
          ? "0 0 20px rgba(16, 185, 129, 0.35), 0 6px 18px rgba(0, 0, 0, 0.45)"
          : isHovered
          ? "0 10px 24px rgba(0, 0, 0, 0.4)"
          : isSelected
          ? "0 4px 18px rgba(99, 102, 241, 0.2)"
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
              background: "linear-gradient(135deg, rgba(99, 102, 241, 0.2), rgba(168, 85, 247, 0.15))",
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
              backgroundColor: "rgba(0, 0, 0, 0.8)",
              backdropFilter: "blur(4px)",
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
            <CheckCircle2 size={16} color="#10b981" />
          </div>
        )}

        {/* Now Playing Animated Equalizer Badge on top left of thumbnail */}
        {isPlaying && (
          <div
            style={{
              position: "absolute",
              top: "7px",
              left: "7px",
              backgroundColor: "rgba(16, 185, 129, 0.92)",
              backdropFilter: "blur(4px)",
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
            <span style={{ fontSize: "0.64rem", fontWeight: 700, color: "#fff", letterSpacing: "0.4px" }}>
              PLAYING
            </span>
          </div>
        )}

        {/* Hover Play Button (Fades in, bottom-left corner of thumbnail) */}
        {/* On hover when playing, play icon becomes pause and clicking stops music */}
        <button
          type="button"
          onClick={handlePlayBtnClick}
          style={{
            position: "absolute",
            bottom: "8px",
            left: "8px",
            width: "38px",
            height: "38px",
            borderRadius: "50%",
            backgroundColor: isPlaying ? "#10b981" : "var(--accent)",
            border: "none",
            color: "#fff",
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
            <RefreshCw size={16} className="animate-spin" />
          ) : isPlaying ? (
            <Pause size={17} fill="#fff" />
          ) : (
            <Play size={17} fill="#fff" style={{ marginLeft: "2px" }} />
          )}
        </button>
      </div>

      {/* 2. Name (Below thumbnail, bold, smaller than thumbnail) */}
      <div
        style={{
          marginTop: "10px",
          fontWeight: 700,
          fontSize: "0.88rem",
          color: isPlaying ? "#10b981" : "#fff",
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
              color: "#10b981",
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
              background: "rgba(99, 102, 241, 0.15)",
              border: "1px solid rgba(99, 102, 241, 0.35)",
              fontSize: "0.72rem",
              fontWeight: 600,
              color: "#818cf8",
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

        <button
          type="button"
          style={{
            background: "none",
            border: "none",
            cursor: "pointer",
            color: isInPlaylist ? "#10b981" : rec.in_wishlist ? "var(--accent-light)" : "var(--text-dim)",
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
            <BookmarkCheck size={16} color="#10b981" />
          ) : (
            <Bookmark size={14} fill={rec.in_wishlist ? "currentColor" : "none"} />
          )}
        </button>
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
}

const MixCard: React.FC<{ mix: MixItem; onPlay: () => void; onOpen?: () => void }> = ({
  mix,
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
          background: mix.bgGradient,
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          padding: "16px",
          textAlign: "center",
        }}
      >
        <Sparkles size={38} color="#fff" style={{ opacity: 0.9, filter: "drop-shadow(0 2px 8px rgba(0,0,0,0.4))" }} />
        <span
          style={{
            marginTop: "8px",
            fontSize: "0.72rem",
            fontWeight: 700,
            letterSpacing: "1px",
            color: "rgba(255, 255, 255, 0.9)",
            textTransform: "uppercase",
            textShadow: "0 1px 4px rgba(0,0,0,0.5)",
          }}
        >
          MIX
        </span>

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
            color: "#fff",
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
          <Play size={17} fill="#fff" style={{ marginLeft: "2px" }} />
        </button>
      </div>

      <div
        style={{
          marginTop: "10px",
          fontWeight: 700,
          fontSize: "0.88rem",
          color: "#fff",
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
  searchQuery: string;
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
            backgroundColor: "rgba(0,0,0,0.38)",
            backdropFilter: "blur(4px)",
            padding: "3px 8px",
            borderRadius: "6px",
            fontSize: "0.68rem",
            fontWeight: 800,
            letterSpacing: "1.5px",
            color: "#fff",
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
            color: "#fff",
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
            color: "#fff",
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
          <Play size={17} fill="#fff" style={{ marginLeft: "2px" }} />
        </button>
      </div>

      <div
        style={{
          marginTop: "10px",
          fontWeight: 700,
          fontSize: "0.88rem",
          color: "#fff",
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
  playlists?: Playlist[];
  onPlayPlaylist?: (id: string) => void;
  onAddToWishlist: (rec: DiscoveryRecommendation) => void;
  onSearchDirect: (artist: string, title: string) => void;
  onRefresh?: (force?: boolean) => Promise<void>;
  onPlayOnlineTrack: (rec: DiscoveryRecommendation) => void;
  onStopTrack?: (rec: DiscoveryRecommendation) => void;
  activeOnlineTrackId?: string | null;
  isOnlinePlaying?: boolean;
  isOnlineLoading?: boolean;
  currentLocalTrack?: Track | null;
  isLocalPlaying?: boolean;
  onOpenCollection?: (collection: CollectionData) => void;
  downloads?: DownloadTask[];
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
}

export const DiscoveryView: React.FC<DiscoveryViewProps> = ({
  recommendations,
  playlists = [],
  onPlayPlaylist,
  onAddToWishlist,
  onSearchDirect,
  onRefresh: _onRefresh,
  onPlayOnlineTrack,
  onStopTrack,
  activeOnlineTrackId,
  isOnlinePlaying = false,
  isOnlineLoading = false,
  currentLocalTrack,
  isLocalPlaying = false,
  onOpenCollection,
  downloads = [],
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
}) => {
  const [filter, setFilter] = useState<"ALL" | "TRENDING" | "GENRE" | "SIMILAR">("ALL");
  const [isOnline, setIsOnline] = useState(
    typeof navigator !== "undefined" ? navigator.onLine : true
  );

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

      // Case-insensitive merge with loaded recommendations matching query
      // so songs that were already displayed on screen never vanish
      const qLower = q.toLowerCase();
      const seenKeys = new Set(
        incoming.map((r) => `${r.artist.toLowerCase().trim()}:${r.title.toLowerCase().trim()}`)
      );

      for (const rec of recommendations) {
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

      setSearchResults(incoming);
    } catch (err) {
      console.error("Online music search failed:", err);
      // Fallback: Show loaded recommendations matching the query case-insensitively
      const qLower = q.toLowerCase();
      const fallbackMatches = recommendations.filter(
        (rec) =>
          rec.title.toLowerCase().includes(qLower) ||
          rec.artist.toLowerCase().includes(qLower) ||
          (rec.album && rec.album.toLowerCase().includes(qLower))
      );
      setSearchResults(fallbackMatches);
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

  const isSearchActive = searchResults !== null;
  const currentList = isSearchActive ? searchResults : recommendations;

  const qClean = searchQuery.trim();
  const qLower = qClean.toLowerCase();

  const filteredRecs = currentList.filter((rec) => {
    if (isSearchActive) return true;

    // Instant case-insensitive filtering on loaded recommendations
    if (qLower) {
      const matchTitle = rec.title.toLowerCase().includes(qLower);
      const matchArtist = rec.artist.toLowerCase().includes(qLower);
      const matchAlbum = rec.album ? rec.album.toLowerCase().includes(qLower) : false;
      const matchGenre = rec.genre ? rec.genre.toLowerCase().includes(qLower) : false;
      if (!matchTitle && !matchArtist && !matchAlbum && !matchGenre) {
        return false;
      }
    }

    if (filter === "ALL") return true;
    const r = rec.recommendation_reason.toLowerCase();
    if (filter === "TRENDING") return r.includes("trending") || r.includes("chart");
    if (filter === "GENRE") return r.includes("genre") || r.includes("popular in") || r.includes("trending in");
    if (filter === "SIMILAR") return r.includes("similar") || r.includes("listening") || r.includes("library artist");
    return true;
  });

  const trendingCount = recommendations.filter((r) => {
    const s = r.recommendation_reason.toLowerCase();
    return s.includes("trending") || s.includes("chart");
  }).length;

  const genreCount = recommendations.filter((r) => {
    const s = r.recommendation_reason.toLowerCase();
    return s.includes("genre") || s.includes("popular in") || s.includes("trending in");
  }).length;

  const similarCount = recommendations.filter((r) => {
    const s = r.recommendation_reason.toLowerCase();
    return s.includes("similar") || s.includes("listening") || s.includes("library artist");
  }).length;

  const songsScrollRef = useRef<HTMLDivElement>(null);
  const mixesScrollRef = useRef<HTMLDivElement>(null);
  const chartsScrollRef = useRef<HTMLDivElement>(null);

  const handleScroll = (ref: React.RefObject<HTMLDivElement | null>, direction: "left" | "right") => {
    if (ref.current) {
      const amount = direction === "left" ? -500 : 500;
      ref.current.scrollBy({ left: amount, behavior: "smooth" });
    }
  };

  const userSmartMixes: MixItem[] = (playlists || [])
    .filter((p) => p.is_smart_mix === 1 || (p.track_count && p.track_count > 0))
    .map((p, idx) => {
      const gradients = [
        "linear-gradient(135deg, #4f46e5 0%, #7c3aed 100%)",
        "linear-gradient(135deg, #059669 0%, #0d9488 100%)",
        "linear-gradient(135deg, #dc2626 0%, #ea580c 100%)",
        "linear-gradient(135deg, #1e1b4b 0%, #4338ca 100%)",
        "linear-gradient(135deg, #d946ef 0%, #ec4899 100%)",
      ];
      return {
        id: p.id,
        name: p.name,
        subtitle: p.description || p.generation_reason || `${p.track_count || 0} tracks`,
        bgGradient: gradients[idx % gradients.length],
        accentColor: "#a78bfa",
        playlistId: p.id,
      };
    });

  const curatedMixes: MixItem[] = [
    {
      id: "daily_mix_1",
      name: "Daily Mix 1",
      subtitle: "Personalized blend tailored to your recent favorites",
      bgGradient: "linear-gradient(135deg, #4f46e5 0%, #7c3aed 100%)",
      accentColor: "#818cf8",
      searchQuery: "Daily Mix Hits",
    },
    {
      id: "chill_vibes",
      name: "Chill Vibes",
      subtitle: "Mellow acoustic, lo-fi, and downtempo ambient melodies",
      bgGradient: "linear-gradient(135deg, #059669 0%, #0d9488 100%)",
      accentColor: "#34d399",
      searchQuery: "Chill Lo-Fi Vibes",
    },
    {
      id: "hiphop_urban",
      name: "Hip-Hop & Urban",
      subtitle: "Hard-hitting beats, lyrical flows, and urban anthems",
      bgGradient: "linear-gradient(135deg, #dc2626 0%, #ea580c 100%)",
      accentColor: "#f87171",
      searchQuery: "Hip-Hop Hits",
    },
    {
      id: "late_night_drive",
      name: "Late Night Drive",
      subtitle: "Moody synths, atmospheric basslines, and nocturnal rhythms",
      bgGradient: "linear-gradient(135deg, #1e1b4b 0%, #4338ca 100%)",
      accentColor: "#a5b4fc",
      searchQuery: "Synthwave Night Drive",
    },
    {
      id: "pop_viral_hits",
      name: "Pop & Viral Hits",
      subtitle: "Catchy melodies, trending hooks, and radio favorites",
      bgGradient: "linear-gradient(135deg, #d946ef 0%, #ec4899 100%)",
      accentColor: "#f472b6",
      searchQuery: "Top Pop Hits",
    },
    {
      id: "acoustic_afternoon",
      name: "Acoustic Afternoon",
      subtitle: "Warm acoustic guitars, gentle keys, and soul-stirring vocals",
      bgGradient: "linear-gradient(135deg, #d97706 0%, #b45309 100%)",
      accentColor: "#fbbf24",
      searchQuery: "Acoustic Pop Indie",
    },
    {
      id: "edm_dance_mix",
      name: "EDM Energy",
      subtitle: "Festival bangers, dance floor anthems, and club beats",
      bgGradient: "linear-gradient(135deg, #0891b2 0%, #2563eb 100%)",
      accentColor: "#38bdf8",
      searchQuery: "EDM Dance Hits",
    },
  ];

  const allMixes = [...userSmartMixes, ...curatedMixes];

  const topCharts: ChartItem[] = [
    {
      id: "chart_top50_global",
      title: "Top 50 - Global",
      subtitle: "The most played tracks worldwide right now",
      chartNumber: "GLOBAL",
      region: "Worldwide",
      bgGradient: "linear-gradient(135deg, #065f46 0%, #047857 50%, #059669 100%)",
      badgeBg: "#10b981",
      searchQuery: "Top 50 Global",
    },
    {
      id: "chart_top50_india",
      title: "Top 50 - India",
      subtitle: "Trending blockbusters, Hindi, Punjabi, and Indian indie hits",
      chartNumber: "INDIA",
      region: "India",
      bgGradient: "linear-gradient(135deg, #c2410c 0%, #ea580c 50%, #f97316 100%)",
      badgeBg: "#f97316",
      searchQuery: "Top 50 India",
    },
    {
      id: "chart_viral50_global",
      title: "Viral 50 - Global",
      subtitle: "The tracks going viral on social and streaming charts",
      chartNumber: "VIRAL",
      region: "Worldwide",
      bgGradient: "linear-gradient(135deg, #831843 0%, #be185d 50%, #ec4899 100%)",
      badgeBg: "#f43f5e",
      searchQuery: "Viral 50 Global",
    },
    {
      id: "chart_top50_usa",
      title: "Top 50 - USA",
      subtitle: "Hottest charting tracks and Billboard favorites in the United States",
      chartNumber: "USA",
      region: "United States",
      bgGradient: "linear-gradient(135deg, #1e3a8a 0%, #2563eb 50%, #3b82f6 100%)",
      badgeBg: "#3b82f6",
      searchQuery: "Top 50 USA",
    },
    {
      id: "chart_bollywood_punjabi",
      title: "Top 50 - Bollywood & Punjabi",
      subtitle: "Blockbuster film songs, Punjabi hits, and Desi pop anthems",
      chartNumber: "DESI",
      region: "India / Global",
      bgGradient: "linear-gradient(135deg, #701a75 0%, #a21caf 50%, #c026d3 100%)",
      badgeBg: "#d946ef",
      searchQuery: "Bollywood Punjabi Hits",
    },
    {
      id: "chart_top50_dance_edm",
      title: "Top 50 - Dance & EDM",
      subtitle: "Top club anthems, festival bangers, and electronic dance hits",
      chartNumber: "DANCE",
      region: "Worldwide",
      bgGradient: "linear-gradient(135deg, #0e7490 0%, #06b6d4 50%, #22d3ee 100%)",
      badgeBg: "#06b6d4",
      searchQuery: "Top 50 EDM Dance",
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
      });
    } else {
      handlePlayMixItem(mix);
    }
  };

  const handleOpenChart = (chart: ChartItem) => {
    if (onOpenCollection) {
      onOpenCollection({
        id: chart.id,
        type: "chart",
        title: chart.title,
        subtitle: chart.subtitle,
        tag: `TOP CHART • ${chart.region.toUpperCase()}`,
        bgGradient: chart.bgGradient,
        accentColor: chart.badgeBg,
        searchQuery: chart.searchQuery,
      });
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
    setSearchQuery(chart.searchQuery);
    handleSearchOnline(chart.searchQuery);
  };

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "28px",
        paddingBottom: "40px",
      }}
    >
      {/* Music Discovery Title Header */}
      <div className="view-header" style={{ marginBottom: 0 }}>
        <div>
          <h1 className="view-title" style={{ display: "flex", alignItems: "center", gap: "10px" }}>
            <Sparkles size={26} color="var(--accent-light)" />
            <span>Music Discovery</span>
          </h1>
          <p style={{ color: "var(--text-muted)", fontSize: "0.88rem", marginTop: "4px" }}>
            Explore trending charts, user-tailored genres, or search any music online worldwide.
          </p>
        </div>
      </div>

      {/* Active Search Results Banner */}
      {isSearchActive && (
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            backgroundColor: "rgba(99, 102, 241, 0.12)",
            border: "1px solid rgba(99, 102, 241, 0.3)",
            borderRadius: "10px",
            padding: "10px 18px",
          }}
        >
          <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
            <Globe size={18} color="var(--accent-light)" />
            <span style={{ fontSize: "0.92rem", fontWeight: 600, color: "#fff" }}>
              Online Search: {searchResults.length} {searchResults.length === 1 ? "track" : "tracks"} found for &ldquo;{searchQuery}&rdquo;
            </span>
          </div>
          <button
            onClick={handleClearSearch}
            className="btn btn-secondary"
            style={{
              padding: "5px 12px",
              fontSize: "0.8rem",
              display: "inline-flex",
              alignItems: "center",
              gap: "6px",
            }}
          >
            <X size={14} />
            <span>Clear & Show Recommendations</span>
          </button>
        </div>
      )}

      {/* Curated Category Tabs (Only when not in active online search) */}
      {!isSearchActive && recommendations.length > 0 && (
        <div style={{ display: "flex", alignItems: "center", gap: "8px", flexWrap: "wrap" }}>
          <button
            onClick={() => setFilter("ALL")}
            className={`subtab-btn ${filter === "ALL" ? "active" : ""}`}
          >
            All For You ({recommendations.length})
          </button>
          {genreCount > 0 && (
            <button
              onClick={() => setFilter("GENRE")}
              className={`subtab-btn ${filter === "GENRE" ? "active" : ""}`}
              style={{ display: "inline-flex", alignItems: "center", gap: "5px" }}
            >
              <Radio size={12} color="var(--accent-light)" />
              <span>Top Genres ({genreCount})</span>
            </button>
          )}
          {similarCount > 0 && (
            <button
              onClick={() => setFilter("SIMILAR")}
              className={`subtab-btn ${filter === "SIMILAR" ? "active" : ""}`}
              style={{ display: "inline-flex", alignItems: "center", gap: "5px" }}
            >
              <Sparkles size={12} color="var(--accent-light)" />
              <span>Similar Artists ({similarCount})</span>
            </button>
          )}
          {trendingCount > 0 && (
            <button
              onClick={() => setFilter("TRENDING")}
              className={`subtab-btn ${filter === "TRENDING" ? "active" : ""}`}
              style={{ display: "inline-flex", alignItems: "center", gap: "5px" }}
            >
              <Flame size={12} color="#f59e0b" />
              <span>Trending Hits ({trendingCount})</span>
            </button>
          )}
        </div>
      )}

      {/* Offline State Banner */}
      {!isOnline && (
        <div
          style={{
            backgroundColor: "rgba(24, 24, 27, 0.95)",
            border: "1px solid rgba(239, 68, 68, 0.35)",
            borderRadius: "16px",
            padding: "36px 24px",
            textAlign: "center",
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            justifyContent: "center",
            gap: "10px",
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
          <h2 style={{ fontSize: "1.25rem", fontWeight: 700, color: "#fff", margin: 0 }}>
            No internet connection
          </h2>
          <p style={{ fontSize: "0.92rem", color: "var(--text-muted)", margin: 0, maxWidth: "420px" }}>
            No internet connection. Listen to local songs.
          </p>
          {onGoToLibrary && (
            <button
              type="button"
              className="btn btn-primary"
              onClick={onGoToLibrary}
              style={{
                marginTop: "10px",
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
      )}

      {/* SECTION 1: Trending & Recommended Songs (Horizontal Scroll) */}
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
                color: "#fff",
                display: "flex",
                alignItems: "center",
                gap: "8px",
              }}
            >
              <Music2 size={18} color="var(--accent-light)" />
              <span>{isSearchActive ? `Search Results (${filteredRecs.length})` : "Trending Songs"}</span>
            </h2>
            <p style={{ fontSize: "0.8rem", color: "var(--text-muted)", marginTop: "2px" }}>
              {isSearchActive
                ? `Songs found online for "${searchQuery}"`
                : "Top recommended songs tailored to your taste"}
            </p>
          </div>

          <div style={{ display: "flex", alignItems: "center", gap: "6px" }}>
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

        {filteredRecs.length === 0 ? (
          <div
            className="content-card"
            style={{ textAlign: "center", padding: "45px 20px", color: "var(--text-dim)", borderStyle: "dashed" }}
          >
            {isSearchActive ? (
              <>
                <Globe size={36} color="var(--accent-light)" style={{ marginBottom: "10px" }} />
                <p style={{ fontSize: "1.05rem", fontWeight: 600, color: "var(--text-main)", marginBottom: "4px" }}>
                  No online songs found matching &ldquo;{searchQuery}&rdquo;
                </p>
                <p style={{ fontSize: "0.85rem", color: "var(--text-muted)", maxWidth: "480px", margin: "0 auto" }}>
                  Try searching for another artist or title, or browse our curated mixes below.
                </p>
              </>
            ) : (
              <>
                <Sparkles size={36} color="var(--accent-light)" style={{ marginBottom: "10px" }} />
                <p style={{ fontSize: "1.05rem", fontWeight: 600, color: "var(--text-main)", marginBottom: "4px" }}>
                  No recommendations found
                </p>
                <p style={{ fontSize: "0.85rem", color: "var(--text-muted)", maxWidth: "480px", margin: "0 auto" }}>
                  Click Refresh to discover fresh trending and genre tracks.
                </p>
              </>
            )}
          </div>
        ) : (
          <div ref={songsScrollRef} className="horizontal-scroll-row">
            {filteredRecs.map((rec) => {
              const isPlayingLocal =
                isLocalPlaying &&
                !!currentLocalTrack &&
                ((rec.matched_local_track_id && rec.matched_local_track_id === currentLocalTrack.id) ||
                  rec.external_track_id === currentLocalTrack.id ||
                  (rec.title.toLowerCase().trim() === currentLocalTrack.title.toLowerCase().trim() &&
                    currentLocalTrack.artist_name &&
                    rec.artist.toLowerCase().trim() === currentLocalTrack.artist_name.toLowerCase().trim()));

              const isPlayingOnline =
                activeOnlineTrackId === rec.external_track_id && isOnlinePlaying;

              const isPlayingThis = isPlayingLocal || isPlayingOnline;
              const isSelected = isPlayingThis || activeOnlineTrackId === rec.external_track_id;
              const isResolvingThis =
                activeOnlineTrackId === rec.external_track_id && isOnlineLoading;

              return (
                <div key={rec.external_track_id} style={{ flex: "0 0 174px", width: "174px" }}>
                  <DiscoveryTrackCard
                    rec={rec}
                    isPlaying={isPlayingThis}
                    isLoading={isResolvingThis}
                    isSelected={isSelected}
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
                  />
                </div>
              );
            })}
          </div>
        )}
      </div>

      {/* SECTION 2: Mixes For You (Horizontal Scroll) */}
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
                color: "#fff",
                display: "flex",
                alignItems: "center",
                gap: "8px",
              }}
            >
              <Sparkles size={18} color="#c084fc" />
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
                color: "#fff",
                display: "flex",
                alignItems: "center",
                gap: "8px",
              }}
            >
              <TrendingUp size={18} color="#f59e0b" />
              <span>Top Charts</span>
            </h2>
            <p style={{ fontSize: "0.8rem", color: "var(--text-muted)", marginTop: "2px" }}>
              Top 50 Global, Top 50 India, and trending viral music charts worldwide
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
  );
};
