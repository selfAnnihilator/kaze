import React, { useState, useMemo } from "react";
import {
  Play,
  Pause,
  Plus,
  Check,
  DownloadCloud,
  X,
  Disc,
  User,
  Music,
  ListMusic,
} from "lucide-react";
import { DiscoveryRecommendation, Track, Playlist, DownloadTask } from "../../types";

interface OnlineSearchResultsSectionProps {
  searchQuery: string;
  searchResults: DiscoveryRecommendation[];
  onClearSearch: () => void;
  onPlayOnlineTrack: (rec: DiscoveryRecommendation) => void;
  onStopTrack?: (rec: DiscoveryRecommendation) => void;
  activeOnlineTrackId?: string | null;
  isOnlinePlaying: boolean;
  isOnlineLoading: boolean;
  currentLocalTrack?: Track | null;
  isLocalPlaying?: boolean;
  onAddToPlaylist?: (rec: DiscoveryRecommendation) => void;
  onAddToWishlist?: (rec: DiscoveryRecommendation) => void;
  onSearchDirect?: (artist: string, title: string) => void;
  downloads?: DownloadTask[];
  trackPlaylistMap?: Record<string, string[]>;
  playlists?: Playlist[];
  onSelectArtist?: (artist: string) => void;
}

type SearchCategoryTab = "all" | "songs" | "artists" | "albums" | "playlists";

export const OnlineSearchResultsSection: React.FC<OnlineSearchResultsSectionProps> = ({
  searchQuery,
  searchResults,
  onClearSearch,
  onPlayOnlineTrack,
  onStopTrack,
  activeOnlineTrackId = null,
  isOnlinePlaying,
  isOnlineLoading,
  currentLocalTrack,
  isLocalPlaying = false,
  onAddToPlaylist,
  onAddToWishlist: _onAddToWishlist,
  onSearchDirect,
  downloads,
  trackPlaylistMap,
  playlists = [],
  onSelectArtist,
}) => {
  const [activeTab, setActiveTab] = useState<SearchCategoryTab>("all");
  const [hoveredTrackId, setHoveredTrackId] = useState<string | null>(null);

  const formatDuration = (secs?: number) => {
    if (!secs || secs <= 0) return "--:--";
    const m = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    return `${m}:${s < 10 ? "0" : ""}${s}`;
  };

  const isTrackPlaying = (rec: DiscoveryRecommendation) => {
    const hasRealLocalMatch =
      !!rec.matched_local_track_id &&
      !rec.matched_local_track_id.startsWith("itunes:") &&
      !rec.matched_local_track_id.startsWith("online:");

    const isPlayingLocal =
      isLocalPlaying &&
      !!currentLocalTrack &&
      currentLocalTrack.format !== "online" &&
      !currentLocalTrack.file_path.startsWith("online://") &&
      ((hasRealLocalMatch && rec.matched_local_track_id === currentLocalTrack.id) ||
        rec.external_track_id === currentLocalTrack.id ||
        (rec.title.toLowerCase().trim() === currentLocalTrack.title.toLowerCase().trim() &&
          currentLocalTrack.artist_name &&
          rec.artist.toLowerCase().trim() === currentLocalTrack.artist_name.toLowerCase().trim()));

    const isPlayingOnline =
      activeOnlineTrackId === rec.external_track_id && isOnlinePlaying;

    return isPlayingLocal || isPlayingOnline;
  };

  const isTrackLoading = (rec: DiscoveryRecommendation) => {
    return activeOnlineTrackId === rec.external_track_id && isOnlineLoading;
  };

  const getDownloadStatus = (rec: DiscoveryRecommendation) => {
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

    const isDownloading =
      activeDownload &&
      (activeDownload.status === "DOWNLOADING" || activeDownload.status === "QUEUED");
    const isCompleted = activeDownload?.status === "COMPLETED";

    return { isDownloading, isCompleted };
  };

  const handleTogglePlay = (rec: DiscoveryRecommendation) => {
    if (isTrackPlaying(rec)) {
      if (onStopTrack) {
        onStopTrack(rec);
      } else {
        onPlayOnlineTrack(rec);
      }
    } else {
      onPlayOnlineTrack(rec);
    }
  };

  // Top result: first scored match
  const topResult = searchResults.length > 0 ? searchResults[0] : null;
  const songsList = searchResults.slice(0, 5);
  const remainingSongs = searchResults.slice(5);

  // Extract distinct artists
  const distinctArtists = useMemo(() => {
    const map = new Map<string, { artist: string; cover?: string; count: number }>();
    for (const r of searchResults) {
      const lower = r.artist.toLowerCase().trim();
      if (!map.has(lower)) {
        map.set(lower, { artist: r.artist, cover: r.cover_art_url, count: 1 });
      } else {
        const item = map.get(lower)!;
        item.count++;
        if (!item.cover && r.cover_art_url) item.cover = r.cover_art_url;
      }
    }
    return Array.from(map.values());
  }, [searchResults]);

  // Extract distinct albums
  const distinctAlbums = useMemo(() => {
    const map = new Map<string, { album: string; artist: string; cover?: string }>();
    for (const r of searchResults) {
      if (r.album && r.album.trim()) {
        const key = `${r.artist.toLowerCase().trim()}:${r.album.toLowerCase().trim()}`;
        if (!map.has(key)) {
          map.set(key, { album: r.album, artist: r.artist, cover: r.cover_art_url });
        }
      }
    }
    return Array.from(map.values());
  }, [searchResults]);

  // Filter user's playlists matching search query
  const matchingPlaylists = useMemo(() => {
    const qLower = searchQuery.toLowerCase().trim();
    if (!qLower) return [];
    return playlists.filter(
      (p) =>
        p.name.toLowerCase().includes(qLower) ||
        (p.description && p.description.toLowerCase().includes(qLower))
    );
  }, [playlists, searchQuery]);

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "18px",
        marginBottom: "32px",
        backgroundColor: "rgba(18, 20, 28, 0.75)",
        border: "1px solid rgba(255, 255, 255, 0.08)",
        borderRadius: "14px",
        padding: "20px 24px",
        boxShadow: "0 8px 32px rgba(0, 0, 0, 0.35)",
      }}
    >
      {/* Top Header: Category Filter Pills + Clear Button */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          flexWrap: "wrap",
          gap: "12px",
          borderBottom: "1px solid rgba(255, 255, 255, 0.07)",
          paddingBottom: "14px",
        }}
      >
        {/* Category Filter Pills (Spotify Desktop style) */}
        <div style={{ display: "flex", alignItems: "center", gap: "8px", flexWrap: "wrap" }}>
          <button
            type="button"
            onClick={() => setActiveTab("all")}
            style={{
              padding: "6px 14px",
              borderRadius: "9999px",
              fontSize: "0.82rem",
              fontWeight: 600,
              cursor: "pointer",
              border: "none",
              backgroundColor: activeTab === "all" ? "#e8d8c9" : "rgba(255, 255, 255, 0.08)",
              color: activeTab === "all" ? "#1a1714" : "#e8d8c9",
              transition: "all 0.15s ease",
            }}
          >
            All
          </button>
          <button
            type="button"
            onClick={() => setActiveTab("songs")}
            style={{
              padding: "6px 14px",
              borderRadius: "9999px",
              fontSize: "0.82rem",
              fontWeight: 600,
              cursor: "pointer",
              border: "none",
              backgroundColor: activeTab === "songs" ? "#e8d8c9" : "rgba(255, 255, 255, 0.08)",
              color: activeTab === "songs" ? "#1a1714" : "#e8d8c9",
              transition: "all 0.15s ease",
            }}
          >
            Songs ({searchResults.length})
          </button>
          {distinctArtists.length > 0 && (
            <button
              type="button"
              onClick={() => setActiveTab("artists")}
              style={{
                padding: "6px 14px",
                borderRadius: "9999px",
                fontSize: "0.82rem",
                fontWeight: 600,
                cursor: "pointer",
                border: "none",
                backgroundColor: activeTab === "artists" ? "#e8d8c9" : "rgba(255, 255, 255, 0.08)",
                color: activeTab === "artists" ? "#1a1714" : "#e8d8c9",
                transition: "all 0.15s ease",
              }}
            >
              Artists ({distinctArtists.length})
            </button>
          )}
          {distinctAlbums.length > 0 && (
            <button
              type="button"
              onClick={() => setActiveTab("albums")}
              style={{
                padding: "6px 14px",
                borderRadius: "9999px",
                fontSize: "0.82rem",
                fontWeight: 600,
                cursor: "pointer",
                border: "none",
                backgroundColor: activeTab === "albums" ? "#e8d8c9" : "rgba(255, 255, 255, 0.08)",
                color: activeTab === "albums" ? "#1a1714" : "#e8d8c9",
                transition: "all 0.15s ease",
              }}
            >
              Albums ({distinctAlbums.length})
            </button>
          )}
          {matchingPlaylists.length > 0 && (
            <button
              type="button"
              onClick={() => setActiveTab("playlists")}
              style={{
                padding: "6px 14px",
                borderRadius: "9999px",
                fontSize: "0.82rem",
                fontWeight: 600,
                cursor: "pointer",
                border: "none",
                backgroundColor: activeTab === "playlists" ? "#e8d8c9" : "rgba(255, 255, 255, 0.08)",
                color: activeTab === "playlists" ? "#1a1714" : "#e8d8c9",
                transition: "all 0.15s ease",
              }}
            >
              Playlists ({matchingPlaylists.length})
            </button>
          )}
        </div>

        {/* Clear Search & Results info */}
        <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
          <span style={{ fontSize: "0.82rem", color: "var(--text-dim)" }}>
            Search results for &ldquo;<span style={{ color: "#e8d8c9" }}>{searchQuery}</span>&rdquo;
          </span>
          <button
            type="button"
            onClick={onClearSearch}
            className="btn btn-secondary"
            style={{
              padding: "5px 12px",
              fontSize: "0.8rem",
              display: "inline-flex",
              alignItems: "center",
              gap: "6px",
              borderRadius: "8px",
            }}
            title="Clear search results"
          >
            <X size={14} />
            <span>Clear Search</span>
          </button>
        </div>
      </div>

      {/* Empty State if 0 Results */}
      {searchResults.length === 0 ? (
        <div
          style={{
            padding: "48px 24px",
            textAlign: "center",
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            gap: "12px",
          }}
        >
          <Music size={40} color="var(--text-dim)" />
          <h3 style={{ fontSize: "1.1rem", fontWeight: 600, color: "#e8d8c9" }}>
            No relevant results found for &ldquo;{searchQuery}&rdquo;
          </h3>
          <p style={{ fontSize: "0.86rem", color: "var(--text-muted)", maxWidth: "420px" }}>
            Please check your spelling, try another artist or song title, or explore the trending
            music and mixes below.
          </p>
        </div>
      ) : (
        <>
          {/* TAB: ALL */}
          {activeTab === "all" && (
            <div style={{ display: "flex", flexDirection: "column", gap: "24px" }}>
              <div
                style={{
                  display: "grid",
                  gridTemplateColumns: "minmax(280px, 380px) 1fr",
                  gap: "28px",
                  alignItems: "start",
                }}
              >
                {/* Column 1: Top Result Card */}
                {topResult && (
                  <div>
                    <h3
                      style={{
                        fontSize: "1.2rem",
                        fontWeight: 700,
                        color: "#e8d8c9",
                        marginBottom: "14px",
                      }}
                    >
                      Top result
                    </h3>
                    <div
                      onClick={() => handleTogglePlay(topResult)}
                      style={{
                        backgroundColor: isTrackPlaying(topResult)
                          ? "rgba(139, 124, 246, 0.14)"
                          : "var(--bg-card)",
                        border: isTrackPlaying(topResult)
                          ? "1.5px solid var(--accent-secondary)"
                          : "1px solid var(--border)",
                        borderRadius: "12px",
                        padding: "20px",
                        cursor: "pointer",
                        display: "flex",
                        flexDirection: "column",
                        gap: "12px",
                        position: "relative",
                        transition: "all 0.2s cubic-bezier(0.4, 0, 0.2, 1)",
                        boxShadow: "0 6px 20px rgba(0, 0, 0, 0.25)",
                      }}
                    >
                      {/* Top Result Artwork */}
                      <div
                        style={{
                          width: "96px",
                          height: "96px",
                          borderRadius: "8px",
                          overflow: "hidden",
                          backgroundColor: "rgba(255, 255, 255, 0.05)",
                          boxShadow: "0 8px 24px rgba(0, 0, 0, 0.4)",
                          position: "relative",
                          flexShrink: 0,
                        }}
                      >
                        {topResult.cover_art_url ? (
                          <img
                            src={topResult.cover_art_url}
                            alt={topResult.title}
                            style={{ width: "100%", height: "100%", objectFit: "cover" }}
                          />
                        ) : (
                          <div
                            style={{
                              width: "100%",
                              height: "100%",
                              display: "flex",
                              alignItems: "center",
                              justifyContent: "center",
                              background:
                                "linear-gradient(135deg, rgba(243, 112, 30, 0.2), rgba(232, 216, 201, 0.15))",
                            }}
                          >
                            <Disc size={40} color="var(--accent-light)" />
                          </div>
                        )}
                      </div>

                      {/* Title */}
                      <div
                        style={{
                          fontSize: "1.45rem",
                          fontWeight: 700,
                          color: isTrackPlaying(topResult) ? "var(--accent-secondary)" : "#e8d8c9",
                          whiteSpace: "nowrap",
                          overflow: "hidden",
                          textOverflow: "ellipsis",
                          marginTop: "6px",
                        }}
                        title={topResult.title}
                      >
                        {topResult.title}
                      </div>

                      {/* Subtitle with Badge */}
                      <div
                        style={{
                          display: "flex",
                          alignItems: "center",
                          gap: "8px",
                          fontSize: "0.86rem",
                          color: "var(--text-muted)",
                        }}
                      >
                        <span
                          style={{
                            padding: "2px 8px",
                            borderRadius: "10px",
                            backgroundColor: "rgba(255, 255, 255, 0.1)",
                            color: "#e8d8c9",
                            fontSize: "0.72rem",
                            fontWeight: 600,
                          }}
                        >
                          Song
                        </span>
                        <span>•</span>
                        <span
                          onClick={(e) => {
                            e.stopPropagation();
                            onSelectArtist?.(topResult.artist);
                          }}
                          style={{
                            cursor: "pointer",
                            color: "var(--text-main)",
                            fontWeight: 500,
                          }}
                          onMouseEnter={(e) => (e.currentTarget.style.textDecoration = "underline")}
                          onMouseLeave={(e) => (e.currentTarget.style.textDecoration = "none")}
                        >
                          {topResult.artist}
                        </span>
                      </div>

                      {/* Bottom action row: buttons + large circular play button */}
                      <div
                        style={{
                          display: "flex",
                          alignItems: "center",
                          justifyContent: "space-between",
                          marginTop: "12px",
                        }}
                      >
                        <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                          {onAddToPlaylist && (
                            <button
                              type="button"
                              onClick={(e) => {
                                e.stopPropagation();
                                onAddToPlaylist(topResult);
                              }}
                              style={{
                                width: "34px",
                                height: "34px",
                                borderRadius: "50%",
                                border: "1px solid rgba(255, 255, 255, 0.15)",
                                backgroundColor: "rgba(255, 255, 255, 0.06)",
                                color: "#e8d8c9",
                                display: "flex",
                                alignItems: "center",
                                justifyContent: "center",
                                cursor: "pointer",
                                transition: "all 0.15s ease",
                              }}
                              title="Add to playlist"
                            >
                              <Plus size={16} />
                            </button>
                          )}
                          {onSearchDirect && (
                            <button
                              type="button"
                              onClick={(e) => {
                                e.stopPropagation();
                                onSearchDirect(topResult.artist, topResult.title);
                              }}
                              style={{
                                width: "34px",
                                height: "34px",
                                borderRadius: "50%",
                                border: "1px solid rgba(255, 255, 255, 0.15)",
                                backgroundColor: "rgba(255, 255, 255, 0.06)",
                                color: getDownloadStatus(topResult).isCompleted
                                  ? "var(--accent-secondary)"
                                  : getDownloadStatus(topResult).isDownloading
                                  ? "var(--accent-light)"
                                  : "var(--text-muted)",
                                display: "flex",
                                alignItems: "center",
                                justifyContent: "center",
                                cursor: "pointer",
                                transition: "all 0.15s ease",
                              }}
                              title={
                                getDownloadStatus(topResult).isCompleted
                                  ? "Downloaded"
                                  : getDownloadStatus(topResult).isDownloading
                                  ? "Downloading..."
                                  : "Download track"
                              }
                            >
                              {getDownloadStatus(topResult).isCompleted ? (
                                <Check size={16} />
                              ) : (
                                <DownloadCloud size={16} />
                              )}
                            </button>
                          )}
                        </div>

                        {/* Prominent Play Button (Spotify green circular) */}
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            handleTogglePlay(topResult);
                          }}
                          style={{
                            width: "48px",
                            height: "48px",
                            borderRadius: "50%",
                            backgroundColor: "var(--accent-secondary)",
                            border: "none",
                            color: "#e8d8c9",
                            display: "flex",
                            alignItems: "center",
                            justifyContent: "center",
                            cursor: "pointer",
                            boxShadow: "0 8px 18px rgba(139, 124, 246, 0.35)",
                            transition: "transform 0.15s ease, background-color 0.15s ease",
                          }}
                          onMouseEnter={(e) => (e.currentTarget.style.transform = "scale(1.06)")}
                          onMouseLeave={(e) => (e.currentTarget.style.transform = "scale(1)")}
                          title={isTrackPlaying(topResult) ? "Pause" : "Play"}
                        >
                          {isTrackLoading(topResult) ? (
                            <div
                              className="spin-animation"
                              style={{
                                width: "18px",
                                height: "18px",
                                border: "2px solid #e8d8c9",
                                borderTopColor: "transparent",
                                borderRadius: "50%",
                              }}
                            />
                          ) : isTrackPlaying(topResult) ? (
                            <Pause size={22} fill="#e8d8c9" />
                          ) : (
                            <Play size={22} fill="#e8d8c9" style={{ marginLeft: "2px" }} />
                          )}
                        </button>
                      </div>
                    </div>
                  </div>
                )}

                {/* Column 2: Songs List */}
                <div>
                  <h3
                    style={{
                      fontSize: "1.2rem",
                      fontWeight: 700,
                      color: "#e8d8c9",
                      marginBottom: "14px",
                    }}
                  >
                    Songs
                  </h3>
                  <div style={{ display: "flex", flexDirection: "column", gap: "4px" }}>
                    {songsList.map((rec) => {
                      const isPlaying = isTrackPlaying(rec);
                      const isLoading = isTrackLoading(rec);
                      const isHovered = hoveredTrackId === rec.external_track_id;

                      const trackId = rec.matched_local_track_id || rec.external_track_id;
                      const inPlaylist =
                        trackId && trackPlaylistMap?.[trackId]?.length ? true : false;

                      return (
                        <div
                          key={rec.external_track_id}
                          onMouseEnter={() => setHoveredTrackId(rec.external_track_id)}
                          onMouseLeave={() => setHoveredTrackId(null)}
                          onClick={() => handleTogglePlay(rec)}
                          style={{
                            display: "flex",
                            alignItems: "center",
                            gap: "12px",
                            padding: "8px 12px",
                            borderRadius: "8px",
                            backgroundColor: isPlaying
                              ? "rgba(139, 124, 246, 0.1)"
                              : isHovered
                              ? "rgba(255, 255, 255, 0.06)"
                              : "transparent",
                            cursor: "pointer",
                            transition: "background-color 0.15s ease",
                          }}
                        >
                          {/* Thumbnail / Play trigger */}
                          <div
                            style={{
                              width: "42px",
                              height: "42px",
                              borderRadius: "6px",
                              overflow: "hidden",
                              backgroundColor: "rgba(255, 255, 255, 0.05)",
                              position: "relative",
                              flexShrink: 0,
                            }}
                          >
                            {rec.cover_art_url ? (
                              <img
                                src={rec.cover_art_url}
                                alt={rec.title}
                                style={{ width: "100%", height: "100%", objectFit: "cover" }}
                              />
                            ) : (
                              <div
                                style={{
                                  width: "100%",
                                  height: "100%",
                                  display: "flex",
                                  alignItems: "center",
                                  justifyContent: "center",
                                  background: "rgba(255, 255, 255, 0.05)",
                                }}
                              >
                                <Disc size={20} color="var(--accent-light)" />
                              </div>
                            )}

                            {/* Play Overlay on Hover or when Playing */}
                            {(isHovered || isPlaying || isLoading) && (
                              <div
                                style={{
                                  position: "absolute",
                                  inset: 0,
                                  backgroundColor: "rgba(0, 0, 0, 0.55)",
                                  display: "flex",
                                  alignItems: "center",
                                  justifyContent: "center",
                                }}
                              >
                                {isLoading ? (
                                  <div
                                    className="spin-animation"
                                    style={{
                                      width: "14px",
                                      height: "14px",
                                      border: "2px solid #e8d8c9",
                                      borderTopColor: "transparent",
                                      borderRadius: "50%",
                                    }}
                                  />
                                ) : isPlaying ? (
                                  <Pause size={16} fill="var(--accent-secondary)" color="var(--accent-secondary)" />
                                ) : (
                                  <Play size={16} fill="#e8d8c9" color="#e8d8c9" />
                                )}
                              </div>
                            )}
                          </div>

                          {/* Song Title & Artist */}
                          <div style={{ flex: 1, minWidth: 0 }}>
                            <div
                              style={{
                                fontSize: "0.93rem",
                                fontWeight: 600,
                                color: isPlaying ? "var(--accent-secondary)" : "#e8d8c9",
                                whiteSpace: "nowrap",
                                overflow: "hidden",
                                textOverflow: "ellipsis",
                              }}
                              title={rec.title}
                            >
                              {rec.title}
                            </div>
                            <div
                              style={{
                                fontSize: "0.8rem",
                                color: "var(--text-muted)",
                                whiteSpace: "nowrap",
                                overflow: "hidden",
                                textOverflow: "ellipsis",
                                marginTop: "2px",
                              }}
                              title={rec.artist}
                            >
                              {rec.artist}
                            </div>
                          </div>

                          {/* Badge: Single or Song */}
                          <div style={{ flexShrink: 0 }}>
                            <span
                              style={{
                                padding: "2px 8px",
                                borderRadius: "8px",
                                backgroundColor: "rgba(255, 255, 255, 0.07)",
                                color: "var(--text-muted)",
                                fontSize: "0.72rem",
                                fontWeight: 500,
                              }}
                            >
                              {rec.album ? "Album" : "Song"}
                            </span>
                          </div>

                          {/* Add to Playlist button */}
                          {onAddToPlaylist && (
                            <button
                              type="button"
                              onClick={(e) => {
                                e.stopPropagation();
                                onAddToPlaylist(rec);
                              }}
                              style={{
                                background: "none",
                                border: "none",
                                color: inPlaylist ? "var(--accent-secondary)" : "var(--text-dim)",
                                cursor: "pointer",
                                padding: "6px",
                                display: "flex",
                                alignItems: "center",
                                justifyContent: "center",
                                transition: "color 0.15s ease",
                              }}
                              title={inPlaylist ? "Added to playlist" : "Add to playlist"}
                            >
                              {inPlaylist ? <Check size={16} /> : <Plus size={16} />}
                            </button>
                          )}

                          {/* Download Button */}
                          {onSearchDirect && (
                            <button
                              type="button"
                              onClick={(e) => {
                                e.stopPropagation();
                                onSearchDirect(rec.artist, rec.title);
                              }}
                              style={{
                                background: "none",
                                border: "none",
                                color: getDownloadStatus(rec).isCompleted
                                  ? "var(--accent-secondary)"
                                  : getDownloadStatus(rec).isDownloading
                                  ? "var(--accent-light)"
                                  : "var(--text-dim)",
                                cursor: "pointer",
                                padding: "6px",
                                display: "flex",
                                alignItems: "center",
                                justifyContent: "center",
                                transition: "color 0.15s ease",
                              }}
                              title={
                                getDownloadStatus(rec).isCompleted
                                  ? "Downloaded"
                                  : getDownloadStatus(rec).isDownloading
                                  ? "Downloading..."
                                  : "Download track"
                              }
                            >
                              {getDownloadStatus(rec).isCompleted ? (
                                <Check size={16} />
                              ) : (
                                <DownloadCloud size={16} />
                              )}
                            </button>
                          )}

                          {/* Duration */}
                          <div
                            style={{
                              fontSize: "0.8rem",
                              color: "var(--text-dim)",
                              minWidth: "38px",
                              textAlign: "right",
                            }}
                          >
                            {formatDuration(rec.duration_secs)}
                          </div>
                        </div>
                      );
                    })}
                  </div>
                </div>
              </div>

              {/* More Matching Songs row (if more than 5 results exist) */}
              {remainingSongs.length > 0 && (
                <div style={{ marginTop: "8px" }}>
                  <div
                    style={{
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "space-between",
                      marginBottom: "12px",
                    }}
                  >
                    <h4 style={{ fontSize: "1.05rem", fontWeight: 700, color: "#e8d8c9" }}>
                      More Matching Results ({remainingSongs.length})
                    </h4>
                    <button
                      type="button"
                      onClick={() => setActiveTab("songs")}
                      style={{
                        background: "none",
                        border: "none",
                        color: "var(--accent-light)",
                        fontSize: "0.82rem",
                        fontWeight: 600,
                        cursor: "pointer",
                      }}
                    >
                      Show all songs &rarr;
                    </button>
                  </div>

                  <div
                    style={{
                      display: "grid",
                      gridTemplateColumns: "repeat(auto-fill, minmax(280px, 1fr))",
                      gap: "10px",
                    }}
                  >
                    {remainingSongs.slice(0, 6).map((rec) => {
                      const isPlaying = isTrackPlaying(rec);
                      return (
                        <div
                          key={rec.external_track_id}
                          onClick={() => handleTogglePlay(rec)}
                          style={{
                            display: "flex",
                            alignItems: "center",
                            gap: "10px",
                            padding: "8px 12px",
                            borderRadius: "8px",
                            backgroundColor: isPlaying
                              ? "rgba(139, 124, 246, 0.1)"
                              : "rgba(255, 255, 255, 0.03)",
                            border: "1px solid rgba(255, 255, 255, 0.05)",
                            cursor: "pointer",
                            transition: "background-color 0.15s ease",
                          }}
                        >
                          <div
                            style={{
                              width: "36px",
                              height: "36px",
                              borderRadius: "4px",
                              overflow: "hidden",
                              flexShrink: 0,
                            }}
                          >
                            {rec.cover_art_url ? (
                              <img
                                src={rec.cover_art_url}
                                alt={rec.title}
                                style={{ width: "100%", height: "100%", objectFit: "cover" }}
                              />
                            ) : (
                              <div
                                style={{
                                  width: "100%",
                                  height: "100%",
                                  display: "flex",
                                  alignItems: "center",
                                  justifyContent: "center",
                                  background: "rgba(255, 255, 255, 0.05)",
                                }}
                              >
                                <Disc size={16} color="var(--accent-light)" />
                              </div>
                            )}
                          </div>
                          <div style={{ flex: 1, minWidth: 0 }}>
                            <div
                              style={{
                                fontSize: "0.88rem",
                                fontWeight: 600,
                                color: isPlaying ? "var(--accent-secondary)" : "#e8d8c9",
                                whiteSpace: "nowrap",
                                overflow: "hidden",
                                textOverflow: "ellipsis",
                              }}
                            >
                              {rec.title}
                            </div>
                            <div
                              style={{
                                fontSize: "0.78rem",
                                color: "var(--text-muted)",
                                whiteSpace: "nowrap",
                                overflow: "hidden",
                                textOverflow: "ellipsis",
                              }}
                            >
                              {rec.artist}
                            </div>
                          </div>
                          {onAddToPlaylist && (
                            <button
                              type="button"
                              onClick={(e) => {
                                e.stopPropagation();
                                onAddToPlaylist(rec);
                              }}
                              style={{
                                background: "none",
                                border: "none",
                                color: "var(--text-dim)",
                                cursor: "pointer",
                                padding: "4px",
                              }}
                              title="Add to playlist"
                            >
                              <Plus size={15} />
                            </button>
                          )}
                        </div>
                      );
                    })}
                  </div>
                </div>
              )}
            </div>
          )}

          {/* TAB: SONGS (Full List) */}
          {activeTab === "songs" && (
            <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  marginBottom: "8px",
                  padding: "0 12px",
                }}
              >
                <span style={{ fontSize: "0.82rem", fontWeight: 600, color: "var(--text-dim)" }}>
                  TRACK
                </span>
                <span style={{ fontSize: "0.82rem", fontWeight: 600, color: "var(--text-dim)" }}>
                  DURATION
                </span>
              </div>

              {searchResults.map((rec) => {
                const isPlaying = isTrackPlaying(rec);
                const isHovered = hoveredTrackId === rec.external_track_id;
                const trackId = rec.matched_local_track_id || rec.external_track_id;
                const inPlaylist = trackId && trackPlaylistMap?.[trackId]?.length ? true : false;

                return (
                  <div
                    key={rec.external_track_id}
                    onMouseEnter={() => setHoveredTrackId(rec.external_track_id)}
                    onMouseLeave={() => setHoveredTrackId(null)}
                    onClick={() => handleTogglePlay(rec)}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: "12px",
                      padding: "8px 12px",
                      borderRadius: "8px",
                      backgroundColor: isPlaying
                        ? "rgba(139, 124, 246, 0.1)"
                        : isHovered
                        ? "rgba(255, 255, 255, 0.06)"
                        : "transparent",
                      cursor: "pointer",
                      transition: "background-color 0.15s ease",
                    }}
                  >
                    <div
                      style={{
                        width: "40px",
                        height: "40px",
                        borderRadius: "6px",
                        overflow: "hidden",
                        backgroundColor: "rgba(255, 255, 255, 0.05)",
                        position: "relative",
                        flexShrink: 0,
                      }}
                    >
                      {rec.cover_art_url ? (
                        <img
                          src={rec.cover_art_url}
                          alt={rec.title}
                          style={{ width: "100%", height: "100%", objectFit: "cover" }}
                        />
                      ) : (
                        <div
                          style={{
                            width: "100%",
                            height: "100%",
                            display: "flex",
                            alignItems: "center",
                            justifyContent: "center",
                            background: "rgba(255, 255, 255, 0.05)",
                          }}
                        >
                          <Disc size={18} color="var(--accent-light)" />
                        </div>
                      )}

                      {(isHovered || isPlaying) && (
                        <div
                          style={{
                            position: "absolute",
                            inset: 0,
                            backgroundColor: "rgba(0, 0, 0, 0.55)",
                            display: "flex",
                            alignItems: "center",
                            justifyContent: "center",
                          }}
                        >
                          {isPlaying ? (
                            <Pause size={16} fill="var(--accent-secondary)" color="var(--accent-secondary)" />
                          ) : (
                            <Play size={16} fill="#e8d8c9" color="#e8d8c9" />
                          )}
                        </div>
                      )}
                    </div>

                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div
                        style={{
                          fontSize: "0.93rem",
                          fontWeight: 600,
                          color: isPlaying ? "var(--accent-secondary)" : "#e8d8c9",
                          whiteSpace: "nowrap",
                          overflow: "hidden",
                          textOverflow: "ellipsis",
                        }}
                      >
                        {rec.title}
                      </div>
                      <div
                        style={{
                          fontSize: "0.8rem",
                          color: "var(--text-muted)",
                          whiteSpace: "nowrap",
                          overflow: "hidden",
                          textOverflow: "ellipsis",
                          marginTop: "2px",
                        }}
                      >
                        {rec.artist} {rec.album ? `• ${rec.album}` : ""}
                      </div>
                    </div>

                    {onAddToPlaylist && (
                      <button
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation();
                          onAddToPlaylist(rec);
                        }}
                        style={{
                          background: "none",
                          border: "none",
                          color: inPlaylist ? "var(--accent-secondary)" : "var(--text-dim)",
                          cursor: "pointer",
                          padding: "6px",
                          display: "flex",
                          alignItems: "center",
                          justifyContent: "center",
                        }}
                        title={inPlaylist ? "Added to playlist" : "Add to playlist"}
                      >
                        {inPlaylist ? <Check size={16} /> : <Plus size={16} />}
                      </button>
                    )}

                    {onSearchDirect && (
                      <button
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation();
                          onSearchDirect(rec.artist, rec.title);
                        }}
                        style={{
                          background: "none",
                          border: "none",
                          color: "var(--text-dim)",
                          cursor: "pointer",
                          padding: "6px",
                        }}
                        title="Download track"
                      >
                        <DownloadCloud size={16} />
                      </button>
                    )}

                    <div
                      style={{
                        fontSize: "0.8rem",
                        color: "var(--text-dim)",
                        minWidth: "38px",
                        textAlign: "right",
                      }}
                    >
                      {formatDuration(rec.duration_secs)}
                    </div>
                  </div>
                );
              })}
            </div>
          )}

          {/* TAB: ARTISTS */}
          {activeTab === "artists" && (
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(auto-fill, minmax(160px, 1fr))",
                gap: "20px",
              }}
            >
              {distinctArtists.map((a) => (
                <div
                  key={a.artist}
                  onClick={() => onSelectArtist?.(a.artist)}
                  style={{
                    backgroundColor: "var(--bg-card)",
                    border: "1px solid var(--border)",
                    borderRadius: "12px",
                    padding: "16px",
                    textAlign: "center",
                    cursor: "pointer",
                    transition: "transform 0.18s ease, border-color 0.18s ease",
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.transform = "translateY(-4px)";
                    e.currentTarget.style.borderColor = "var(--accent-light)";
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.transform = "none";
                    e.currentTarget.style.borderColor = "var(--border)";
                  }}
                >
                  <div
                    style={{
                      width: "88px",
                      height: "88px",
                      borderRadius: "50%",
                      overflow: "hidden",
                      margin: "0 auto 12px auto",
                      backgroundColor: "rgba(243, 112, 30, 0.15)",
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "center",
                    }}
                  >
                    {a.cover ? (
                      <img
                        src={a.cover}
                        alt={a.artist}
                        style={{ width: "100%", height: "100%", objectFit: "cover" }}
                      />
                    ) : (
                      <User size={36} color="var(--accent-light)" />
                    )}
                  </div>
                  <div
                    style={{
                      fontWeight: 700,
                      fontSize: "0.95rem",
                      color: "#e8d8c9",
                      whiteSpace: "nowrap",
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                    }}
                  >
                    {a.artist}
                  </div>
                  <div style={{ fontSize: "0.78rem", color: "var(--text-muted)", marginTop: "4px" }}>
                    Artist • {a.count} track{a.count === 1 ? "" : "s"}
                  </div>
                </div>
              ))}
            </div>
          )}

          {/* TAB: ALBUMS */}
          {activeTab === "albums" && (
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(auto-fill, minmax(180px, 1fr))",
                gap: "20px",
              }}
            >
              {distinctAlbums.map((alb) => (
                <div
                  key={`${alb.artist}:${alb.album}`}
                  onClick={() => onSelectArtist?.(`${alb.artist} ${alb.album}`)}
                  style={{
                    backgroundColor: "var(--bg-card)",
                    border: "1px solid var(--border)",
                    borderRadius: "12px",
                    padding: "14px",
                    cursor: "pointer",
                    transition: "transform 0.18s ease, border-color 0.18s ease",
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.transform = "translateY(-4px)";
                    e.currentTarget.style.borderColor = "var(--accent-light)";
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.transform = "none";
                    e.currentTarget.style.borderColor = "var(--border)";
                  }}
                >
                  <div
                    style={{
                      width: "100%",
                      aspectRatio: "1/1",
                      borderRadius: "8px",
                      overflow: "hidden",
                      backgroundColor: "rgba(255, 255, 255, 0.05)",
                      marginBottom: "10px",
                    }}
                  >
                    {alb.cover ? (
                      <img
                        src={alb.cover}
                        alt={alb.album}
                        style={{ width: "100%", height: "100%", objectFit: "cover" }}
                      />
                    ) : (
                      <div
                        style={{
                          width: "100%",
                          height: "100%",
                          display: "flex",
                          alignItems: "center",
                          justifyContent: "center",
                          background: "rgba(255, 255, 255, 0.05)",
                        }}
                      >
                        <Disc size={36} color="var(--accent-light)" />
                      </div>
                    )}
                  </div>
                  <div
                    style={{
                      fontWeight: 700,
                      fontSize: "0.92rem",
                      color: "#e8d8c9",
                      whiteSpace: "nowrap",
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                    }}
                  >
                    {alb.album}
                  </div>
                  <div
                    style={{
                      fontSize: "0.8rem",
                      color: "var(--text-muted)",
                      marginTop: "2px",
                      whiteSpace: "nowrap",
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                    }}
                  >
                    {alb.artist}
                  </div>
                </div>
              ))}
            </div>
          )}

          {/* TAB: PLAYLISTS */}
          {activeTab === "playlists" && (
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(auto-fill, minmax(200px, 1fr))",
                gap: "20px",
              }}
            >
              {matchingPlaylists.map((pl) => (
                <div
                  key={pl.id}
                  style={{
                    backgroundColor: "var(--bg-card)",
                    border: "1px solid var(--border)",
                    borderRadius: "12px",
                    padding: "16px",
                    cursor: "pointer",
                  }}
                >
                  <div
                    style={{
                      width: "100%",
                      aspectRatio: "1/1",
                      borderRadius: "8px",
                      backgroundColor: "rgba(243, 112, 30, 0.15)",
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "center",
                      marginBottom: "12px",
                    }}
                  >
                    <ListMusic size={40} color="var(--accent-light)" />
                  </div>
                  <div style={{ fontWeight: 700, fontSize: "0.95rem", color: "#e8d8c9" }}>
                    {pl.name}
                  </div>
                  <div style={{ fontSize: "0.8rem", color: "var(--text-muted)", marginTop: "4px" }}>
                    {pl.track_count || 0} tracks
                  </div>
                </div>
              ))}
            </div>
          )}
        </>
      )}
    </div>
  );
};
