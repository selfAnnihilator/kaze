import React, { useState } from "react";
import {
  DownloadCloud,
  Sparkles,
  Bookmark,
  RefreshCw,
  Flame,
  Radio,
  Music2,
  Play,
  Pause,
  X,
  Search,
  Globe,
  CheckCircle2,
} from "lucide-react";
import { DiscoveryRecommendation, Track } from "../../types";
import { executeQuery } from "../../services/api";

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
}) => {
  const [isHovered, setIsHovered] = useState(false);
  const isDownloaded =
    rec.provider === "library" ||
    rec.match_status === "EXACT_MATCH" ||
    !!rec.matched_local_track_id;

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
            color: rec.in_wishlist ? "var(--accent-light)" : "var(--text-dim)",
            padding: "4px",
            display: "flex",
            alignItems: "center",
            transition: "color 0.15s ease",
          }}
          onClick={(e) => {
            e.stopPropagation();
            onAddToWishlist(rec);
          }}
          title={rec.in_wishlist ? "In Wishlist" : "Add to Wishlist"}
        >
          <Bookmark size={14} fill={rec.in_wishlist ? "currentColor" : "none"} />
        </button>
      </div>
    </div>
  );
};

interface DiscoveryViewProps {
  recommendations: DiscoveryRecommendation[];
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
}

export const DiscoveryView: React.FC<DiscoveryViewProps> = ({
  recommendations,
  onAddToWishlist,
  onSearchDirect,
  onRefresh,
  onPlayOnlineTrack,
  onStopTrack,
  activeOnlineTrackId,
  isOnlinePlaying = false,
  isOnlineLoading = false,
  currentLocalTrack,
  isLocalPlaying = false,
}) => {
  const [filter, setFilter] = useState<"ALL" | "TRENDING" | "GENRE" | "SIMILAR">("ALL");
  const [refreshing, setRefreshing] = useState(false);

  // Online Search State
  const [searchQuery, setSearchQuery] = useState("");
  const [isSearchingOnline, setIsSearchingOnline] = useState(false);
  const [searchResults, setSearchResults] = useState<DiscoveryRecommendation[] | null>(null);

  const handleSearchOnline = async (queryOverride?: string) => {
    const q = (typeof queryOverride === "string" ? queryOverride : searchQuery).trim();
    if (!q) {
      setSearchResults(null);
      return;
    }
    setIsSearchingOnline(true);
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
      setIsSearchingOnline(false);
    }
  };

  const handleClearSearch = () => {
    setSearchQuery("");
    setSearchResults(null);
  };

  const handleRefreshSection = async () => {
    setRefreshing(true);
    try {
      if (isSearchActive && searchQuery.trim()) {
        await handleSearchOnline(searchQuery.trim());
        if (onRefresh) {
          onRefresh(true).catch((e) => console.warn("Background discovery refresh error:", e));
        }
      } else if (onRefresh) {
        await onRefresh(true);
      }
    } finally {
      setRefreshing(false);
    }
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

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "20px",
        paddingBottom: "30px",
      }}
    >
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

      {/* Online Music Search Bar with integrated Refresh button */}
      <div
        className="content-card"
        style={{
          marginBottom: 0,
          padding: "16px 20px",
          display: "flex",
          flexDirection: "column",
          gap: "12px",
          backgroundColor: "var(--bg-card)",
          border: "1px solid var(--border)",
          borderRadius: "12px",
        }}
      >
        <form
          onSubmit={(e) => {
            e.preventDefault();
            handleSearchOnline();
          }}
          style={{ display: "flex", alignItems: "center", gap: "10px", width: "100%" }}
        >
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: "10px",
              backgroundColor: "var(--bg-main)",
              border: "1px solid var(--border)",
              borderRadius: "8px",
              padding: "9px 14px",
              flex: 1,
            }}
          >
            <Search size={18} color="var(--accent-light)" />
            <input
              type="text"
              placeholder="Search any music online (artist, song title, album, or genre)..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              style={{
                background: "none",
                border: "none",
                outline: "none",
                color: "#fff",
                fontSize: "0.92rem",
                width: "100%",
              }}
            />
            {searchQuery && (
              <button
                type="button"
                onClick={handleClearSearch}
                style={{
                  background: "none",
                  border: "none",
                  cursor: "pointer",
                  color: "var(--text-dim)",
                  padding: 0,
                  display: "flex",
                }}
                title="Clear search"
              >
                <X size={16} />
              </button>
            )}
          </div>

          <button
            type="submit"
            disabled={isSearchingOnline || !searchQuery.trim()}
            className="btn btn-primary"
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: "8px",
              padding: "10px 20px",
              fontSize: "0.88rem",
              fontWeight: 600,
              whiteSpace: "nowrap",
            }}
          >
            {isSearchingOnline ? (
              <>
                <RefreshCw size={15} className="animate-spin" />
                <span>Searching...</span>
              </>
            ) : (
              <>
                <Globe size={15} />
                <span>Search Online</span>
              </>
            )}
          </button>

          <button
            type="button"
            onClick={handleRefreshSection}
            disabled={refreshing || isSearchingOnline}
            className="btn btn-secondary"
            style={{
              display: "inline-flex",
              alignItems: "center",
              gap: "8px",
              padding: "10px 18px",
              fontSize: "0.88rem",
              fontWeight: 500,
              whiteSpace: "nowrap",
            }}
            title={isSearchActive ? "Refresh current search results" : "Refresh discovery recommendations"}
          >
            <RefreshCw size={15} className={refreshing || isSearchingOnline ? "animate-spin" : ""} />
            <span>{refreshing ? "Refreshing..." : isSearchActive ? "Refresh Results" : "Refresh"}</span>
          </button>
        </form>

        {/* Quick query chips */}
        <div style={{ display: "flex", alignItems: "center", gap: "8px", flexWrap: "wrap" }}>
          <span style={{ fontSize: "0.76rem", color: "var(--text-dim)", fontWeight: 500 }}>
            Try:
          </span>
          {[
            "Malayalam Hits",
            "Pavizha Mazha",
            "Eminem",
            "Arijit Singh",
            "Coldplay",
            "Hip-Hop",
            "EDM",
            "Taylor Swift",
          ].map((suggestion) => (
            <button
              key={suggestion}
              type="button"
              onClick={() => {
                setSearchQuery(suggestion);
                handleSearchOnline(suggestion);
              }}
              style={{
                background: "rgba(255, 255, 255, 0.05)",
                border: "1px solid var(--border)",
                borderRadius: "14px",
                padding: "3px 10px",
                fontSize: "0.75rem",
                color: "var(--text-muted)",
                cursor: "pointer",
                transition: "all 0.15s ease",
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.backgroundColor = "rgba(99, 102, 241, 0.15)";
                e.currentTarget.style.borderColor = "var(--accent-light)";
                e.currentTarget.style.color = "#fff";
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.backgroundColor = "rgba(255, 255, 255, 0.05)";
                e.currentTarget.style.borderColor = "var(--border)";
                e.currentTarget.style.color = "var(--text-muted)";
              }}
            >
              {suggestion}
            </button>
          ))}
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

      {filteredRecs.length === 0 ? (
        <div
          className="content-card"
          style={{ textAlign: "center", padding: "60px 20px", color: "var(--text-dim)", borderStyle: "dashed" }}
        >
          {isSearchActive ? (
            <>
              <Globe size={40} color="var(--accent-light)" style={{ marginBottom: "14px" }} />
              <p style={{ fontSize: "1.1rem", fontWeight: 600, color: "var(--text-main)", marginBottom: "6px" }}>
                No online songs found matching &ldquo;{searchQuery}&rdquo;
              </p>
              <p style={{ fontSize: "0.88rem", color: "var(--text-muted)", maxWidth: "500px", margin: "0 auto" }}>
                Try searching for another artist or title, or browse the curated recommendations.
              </p>
              <button
                onClick={handleClearSearch}
                className="btn btn-secondary"
                style={{ marginTop: "18px", fontSize: "0.85rem", display: "inline-flex", alignItems: "center", gap: "8px" }}
              >
                <span>Return to Recommendations</span>
              </button>
            </>
          ) : (
            <>
              <Sparkles size={40} color="var(--accent-light)" style={{ marginBottom: "14px" }} />
              <p style={{ fontSize: "1.1rem", fontWeight: 600, color: "var(--text-main)", marginBottom: "6px" }}>
                No discovery recommendations found in this view
              </p>
              <p style={{ fontSize: "0.88rem", color: "var(--text-muted)", maxWidth: "500px", margin: "0 auto" }}>
                Listen to your local library or click Refresh Discovery to discover fresh trending and genre tracks.
              </p>
              {onRefresh && (
                <button
                  onClick={handleRefreshSection}
                  className="btn btn-primary"
                  style={{ marginTop: "18px", fontSize: "0.85rem", display: "inline-flex", alignItems: "center", gap: "8px" }}
                >
                  <RefreshCw size={15} className={refreshing ? "animate-spin" : ""} />
                  <span>{refreshing ? "Refreshing..." : "Refresh Discovery Now"}</span>
                </button>
              )}
            </>
          )}
        </div>
      ) : (
        /* Passport-like card grid layout */
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fill, minmax(170px, 1fr))",
            gap: "18px",
          }}
        >
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
              <DiscoveryTrackCard
                key={rec.external_track_id}
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
              />
            );
          })}
        </div>
      )}
    </div>
  );
};
