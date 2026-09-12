import React, { useState, useRef, useEffect } from "react";
import {
  DownloadCloud,
  Sparkles,
  Check,
  Bookmark,
  RefreshCw,
  Flame,
  Radio,
  Music2,
  Play,
  Pause,
  Volume2,
  X,
  Headphones,
} from "lucide-react";
import { DiscoveryRecommendation } from "../../types";
import { executeQuery } from "../../services/api";

interface DiscoveryViewProps {
  recommendations: DiscoveryRecommendation[];
  onAddToWishlist: (rec: DiscoveryRecommendation) => void;
  onSearchDirect: (artist: string, title: string) => void;
  onRefresh?: () => Promise<void>;
  onPausePlayback?: () => void;
}

export const DiscoveryView: React.FC<DiscoveryViewProps> = ({
  recommendations,
  onAddToWishlist,
  onSearchDirect,
  onRefresh,
  onPausePlayback,
}) => {
  const [filter, setFilter] = useState<"ALL" | "TRENDING" | "GENRE" | "SIMILAR">("ALL");
  const [refreshing, setRefreshing] = useState(false);

  // Audio Preview State & Cache for Full Track Streams
  const [playingPreviewId, setPlayingPreviewId] = useState<string | null>(null);
  const [isPreviewPlaying, setIsPreviewPlaying] = useState<boolean>(false);
  const [previewProgress, setPreviewProgress] = useState<number>(0);
  const [previewCurrentTime, setPreviewCurrentTime] = useState<number>(0);
  const [activePreviewTrack, setActivePreviewTrack] = useState<DiscoveryRecommendation | null>(null);
  const [activeDuration, setActiveDuration] = useState<number>(30);
  const [isFullSongActive, setIsFullSongActive] = useState<boolean>(false);
  const [resolvingId, setResolvingId] = useState<string | null>(null);
  const [resolvedAudios, setResolvedAudios] = useState<Record<string, { streamUrl: string; duration: number }>>({});

  const audioRef = useRef<HTMLAudioElement | null>(null);
  const playingPreviewIdRef = useRef<string | null>(null);

  useEffect(() => {
    playingPreviewIdRef.current = playingPreviewId;
  }, [playingPreviewId]);

  // Clean up audio playback when component unmounts
  const cleanupCurrentAudio = () => {
    if (audioRef.current) {
      const a = audioRef.current;
      a.onended = null;
      a.onerror = null;
      a.ontimeupdate = null;
      a.onplay = null;
      a.onpause = null;
      a.onloadedmetadata = null;
      a.pause();
      a.removeAttribute("src");
      a.load();
      audioRef.current = null;
    }
  };

  useEffect(() => {
    return () => {
      cleanupCurrentAudio();
    };
  }, []);

  // Keyboard shortcut: Escape closes preview
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape" && activePreviewTrack) {
        handleStopPreview();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [activePreviewTrack]);

  const setupAndPlayAudio = (url: string, duration: number, startAt: number = 0) => {
    cleanupCurrentAudio();

    const audio = new Audio(url);
    audioRef.current = audio;
    setActiveDuration(duration);

    audio.ontimeupdate = () => {
      const dur = audio.duration && !isNaN(audio.duration) && audio.duration > 0 ? audio.duration : duration;
      setActiveDuration(dur);
      setPreviewProgress((audio.currentTime / dur) * 100);
      setPreviewCurrentTime(audio.currentTime);
    };

    audio.onplay = () => {
      setIsPreviewPlaying(true);
    };

    audio.onpause = () => {
      setIsPreviewPlaying(false);
    };

    audio.onended = () => {
      setIsPreviewPlaying(false);
      setPreviewProgress(0);
      setPreviewCurrentTime(0);
      setPlayingPreviewId(null);
      setActivePreviewTrack(null);
      setIsFullSongActive(false);
    };

    audio.onerror = (err) => {
      console.warn("Preview audio playback error:", err);
      setIsPreviewPlaying(false);
    };

    if (startAt > 0) {
      audio.onloadedmetadata = () => {
        try {
          audio.currentTime = Math.min(startAt, audio.duration || startAt);
        } catch (_) {}
      };
    }

    setIsPreviewPlaying(true);
    audio.play().catch((err) => {
      console.warn("Audio play prevented:", err);
      setIsPreviewPlaying(false);
    });
  };

  const fetchFullSongStream = async (rec: DiscoveryRecommendation) => {
    setResolvingId(rec.external_track_id);
    try {
      const res = await executeQuery({
        query: "ResolveFullTrackAudio",
        payload: { artist: rec.artist, title: rec.title },
      });

      if (res && res.type === "FullTrackAudio" && res.data) {
        const streamUrl = res.data.stream_url;
        const duration = res.data.duration_secs || 240;

        setResolvedAudios((prev) => ({
          ...prev,
          [rec.external_track_id]: { streamUrl, duration },
        }));

        // If user is still selected on this track, play the full song directly!
        if (playingPreviewIdRef.current === rec.external_track_id) {
          setIsFullSongActive(true);
          setActiveDuration(duration);
          setupAndPlayAudio(streamUrl, duration, 0);
        }
      }
    } catch (e) {
      console.warn("Could not resolve full song stream:", e);
    } finally {
      setResolvingId((curr) => (curr === rec.external_track_id ? null : curr));
    }
  };

  const handleTogglePreview = (rec: DiscoveryRecommendation) => {
    if (playingPreviewId === rec.external_track_id) {
      if (resolvingId === rec.external_track_id) {
        // Clicking while fetching cancels/stops
        handleStopPreview();
        return;
      }
      if (isPreviewPlaying) {
        audioRef.current?.pause();
        setIsPreviewPlaying(false);
      } else {
        onPausePlayback?.();
        audioRef.current?.play().catch((err) => {
          console.warn("Playback prevented:", err);
        });
        setIsPreviewPlaying(true);
      }
      return;
    }

    // Stop current preview audio if playing
    cleanupCurrentAudio();

    // Pause main library playback if active
    onPausePlayback?.();

    setPlayingPreviewId(rec.external_track_id);
    setActivePreviewTrack(rec);
    setPreviewProgress(0);
    setPreviewCurrentTime(0);

    const cached = resolvedAudios[rec.external_track_id];

    if (cached) {
      // Instant full song playback from cache
      setIsFullSongActive(true);
      setActiveDuration(cached.duration);
      setupAndPlayAudio(cached.streamUrl, cached.duration, 0);
    } else {
      // Only fetch full song and play directly, never play the 30s snippet
      setIsFullSongActive(false);
      setActiveDuration(240);
      fetchFullSongStream(rec);
    }
  };

  const handleStopPreview = () => {
    cleanupCurrentAudio();
    setIsPreviewPlaying(false);
    setPlayingPreviewId(null);
    setActivePreviewTrack(null);
    setIsFullSongActive(false);
    setPreviewProgress(0);
    setPreviewCurrentTime(0);
  };

  const handleSeek = (e: React.MouseEvent<HTMLDivElement>) => {
    if (!audioRef.current) return;
    const rect = e.currentTarget.getBoundingClientRect();
    const clickPos = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    const dur = audioRef.current.duration && !isNaN(audioRef.current.duration) && audioRef.current.duration > 0
      ? audioRef.current.duration
      : activeDuration;
    audioRef.current.currentTime = clickPos * dur;
    setPreviewProgress(clickPos * 100);
    setPreviewCurrentTime(clickPos * dur);
  };

  const handleRefreshClick = async () => {
    if (!onRefresh) return;
    setRefreshing(true);
    try {
      await onRefresh();
    } finally {
      setRefreshing(false);
    }
  };

  const getBadgeClass = (status: string) => {
    switch (status) {
      case "EXACT_MATCH":
        return "badge badge-exact";
      case "LIKELY_MATCH":
        return "badge badge-likely";
      case "POSSIBLE_MATCH":
        return "badge badge-possible";
      default:
        return "badge badge-missing";
    }
  };

  const getBadgeLabel = (status: string) => {
    switch (status) {
      case "EXACT_MATCH":
        return "In Library";
      case "LIKELY_MATCH":
        return "Likely Owned";
      case "POSSIBLE_MATCH":
        return "Alternate Version";
      default:
        return "Missing Track";
    }
  };

  const formatSeconds = (secs: number) => {
    const s = Math.floor(secs);
    const m = Math.floor(s / 60);
    const rem = s % 60;
    return `${m}:${rem < 10 ? "0" : ""}${rem}`;
  };

  const filteredRecs = recommendations.filter((rec) => {
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
        paddingBottom: activePreviewTrack ? "140px" : "30px",
      }}
    >
      <div className="view-header" style={{ marginBottom: 0 }}>
        <div>
          <h1 className="view-title" style={{ display: "flex", alignItems: "center", gap: "10px" }}>
            <Sparkles size={26} color="var(--accent-light)" />
            <span>Music Discovery</span>
          </h1>
          <p style={{ color: "var(--text-muted)", fontSize: "0.88rem", marginTop: "4px" }}>
            Trending music tailored to your taste profile, top genres, and artist listening habits.
          </p>
        </div>

        {onRefresh && (
          <button
            onClick={handleRefreshClick}
            disabled={refreshing}
            className="btn btn-secondary"
            style={{ display: "flex", alignItems: "center", gap: "8px", fontSize: "0.85rem" }}
            title="Refresh discovery recommendations"
          >
            <RefreshCw size={15} className={refreshing ? "animate-spin" : ""} />
            <span>{refreshing ? "Refreshing..." : "Refresh Discovery"}</span>
          </button>
        )}
      </div>

      {recommendations.length > 0 && (
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
          <Sparkles size={40} color="var(--accent-light)" style={{ marginBottom: "14px" }} />
          <p style={{ fontSize: "1.1rem", fontWeight: 600, color: "var(--text-main)", marginBottom: "6px" }}>
            No discovery recommendations found in this view
          </p>
          <p style={{ fontSize: "0.88rem", color: "var(--text-muted)", maxWidth: "500px", margin: "0 auto" }}>
            Listen to your local library or click Refresh Discovery to discover fresh trending and genre tracks.
          </p>
          {onRefresh && (
            <button
              onClick={handleRefreshClick}
              className="btn btn-primary"
              style={{ marginTop: "18px", fontSize: "0.85rem", display: "inline-flex", alignItems: "center", gap: "8px" }}
            >
              <RefreshCw size={15} />
              <span>Refresh Discovery Now</span>
            </button>
          )}
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: "10px" }}>
          {filteredRecs.map((rec) => {
            const isPlayingThis = playingPreviewId === rec.external_track_id && isPreviewPlaying;
            const isSelected = playingPreviewId === rec.external_track_id;
            const isResolvingThis = resolvingId === rec.external_track_id;

            return (
              <div
                key={rec.external_track_id}
                className="content-card"
                style={{
                  marginBottom: 0,
                  padding: "14px 18px",
                  display: "flex",
                  flexDirection: "column",
                  gap: isSelected ? "12px" : "0px",
                  border: isSelected ? "1px solid var(--accent)" : "1px solid var(--border)",
                  backgroundColor: isSelected ? "rgba(99, 102, 241, 0.07)" : "var(--bg-card)",
                  boxShadow: isSelected ? "0 4px 20px rgba(99, 102, 241, 0.12)" : undefined,
                  transition: "all 0.2s ease",
                }}
              >
                {/* Main Card Row */}
                <div
                  style={{
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "space-between",
                    gap: "16px",
                  }}
                >
                  <div style={{ display: "flex", alignItems: "center", gap: "14px", flex: 1, minWidth: 0 }}>
                    {/* Album Cover Thumbnail with interactive hover / playing overlay */}
                    <div
                      className={`discovery-thumb-container ${isPlayingThis ? "is-playing" : ""}`}
                      onClick={() => handleTogglePreview(rec)}
                      title={isPlayingThis ? "Pause Preview" : "Play Full Song Preview"}
                      style={{ cursor: "pointer" }}
                    >
                      {rec.cover_art_url ? (
                        <img
                          src={rec.cover_art_url}
                          alt={rec.title}
                          style={{
                            width: "100%",
                            height: "100%",
                            objectFit: "cover",
                            backgroundColor: "var(--bg-sidebar)",
                          }}
                        />
                      ) : (
                        <div
                          style={{
                            width: "100%",
                            height: "100%",
                            backgroundColor: "var(--bg-sidebar)",
                            display: "flex",
                            alignItems: "center",
                            justifyContent: "center",
                            border: "1px solid var(--border)",
                          }}
                        >
                          <Music2 size={22} color="var(--accent-light)" />
                        </div>
                      )}

                      {/* Clean Hover / Playing Overlay */}
                      <div className={`discovery-thumb-overlay ${isSelected && isPreviewPlaying ? "is-playing" : ""}`}>
                        {isSelected && isPreviewPlaying ? (
                          <div className="discovery-eq-container">
                            <span className="discovery-eq-bar" />
                            <span className="discovery-eq-bar" />
                            <span className="discovery-eq-bar" />
                          </div>
                        ) : isResolvingThis && !isSelected ? (
                          <RefreshCw size={18} color="#fff" className="animate-spin" />
                        ) : (
                          <Play size={18} color="#fff" style={{ marginLeft: "2px" }} />
                        )}
                      </div>
                    </div>

                    {/* Track Info */}
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div style={{ display: "flex", alignItems: "center", gap: "8px", marginBottom: "3px", flexWrap: "wrap" }}>
                        <span
                          style={{
                            fontWeight: 600,
                            fontSize: "0.95rem",
                            color: "var(--text-main)",
                            whiteSpace: "nowrap",
                            overflow: "hidden",
                            textOverflow: "ellipsis",
                          }}
                          title={rec.title}
                        >
                          {rec.title}
                        </span>
                        <span className={getBadgeClass(rec.match_status)}>
                          {getBadgeLabel(rec.match_status)}
                        </span>
                        {rec.genre && (
                          <span
                            style={{
                              fontSize: "0.7rem",
                              padding: "2px 7px",
                              borderRadius: "10px",
                              backgroundColor: "rgba(255, 255, 255, 0.07)",
                              color: "var(--text-muted)",
                              border: "1px solid var(--border)",
                            }}
                          >
                            {rec.genre}
                          </span>
                        )}
                      </div>

                      <div
                        style={{
                          fontSize: "0.85rem",
                          color: "var(--text-muted)",
                          marginBottom: "4px",
                          whiteSpace: "nowrap",
                          overflow: "hidden",
                          textOverflow: "ellipsis",
                        }}
                      >
                        {rec.artist} {rec.album && rec.album !== rec.genre ? `— ${rec.album}` : ""}
                      </div>

                      <div
                        style={{
                          fontSize: "0.76rem",
                          color: "var(--text-dim)",
                          display: "flex",
                          alignItems: "center",
                          gap: "6px",
                        }}
                      >
                        <Sparkles size={11} color="var(--accent-light)" />
                        <span>{rec.recommendation_reason}</span>
                      </div>
                    </div>
                  </div>

                  {/* Right Actions */}
                  <div style={{ display: "flex", gap: "8px", alignItems: "center", flexShrink: 0 }}>
                    {/* Full Song Preview Button */}
                    <button
                      className={`btn ${isSelected ? "btn-primary" : "btn-secondary"}`}
                      onClick={() => handleTogglePreview(rec)}
                      title={
                        isSelected && isPreviewPlaying
                          ? "Pause Preview"
                          : isSelected && !isPreviewPlaying
                          ? "Resume Preview"
                          : isResolvingThis
                          ? "Loading full song stream..."
                          : "Preview full song before downloading"
                      }
                      style={{
                        fontSize: "0.8rem",
                        padding: "6px 12px",
                        display: "inline-flex",
                        alignItems: "center",
                        gap: "6px",
                      }}
                    >
                      {isSelected && isResolvingThis ? (
                        <>
                          <RefreshCw size={14} className="animate-spin" color={isSelected ? "#fff" : "var(--accent-light)"} />
                          <span>Loading Full Song...</span>
                        </>
                      ) : isSelected && isPreviewPlaying ? (
                        <>
                          <Pause size={14} />
                          <span>Pause Preview</span>
                        </>
                      ) : isSelected && !isPreviewPlaying ? (
                        <>
                          <Play size={14} />
                          <span>Resume Preview</span>
                        </>
                      ) : isResolvingThis ? (
                        <>
                          <RefreshCw size={14} className="animate-spin" color="var(--accent-light)" />
                          <span>Loading Song...</span>
                        </>
                      ) : (
                        <>
                          <Headphones size={14} color="var(--accent-light)" />
                          <span>Preview</span>
                        </>
                      )}
                    </button>

                    {/* Direct Download */}
                    <button
                      className="btn btn-secondary"
                      onClick={() => onSearchDirect(rec.artist, rec.title)}
                      title="Search & download directly in-app"
                      style={{ fontSize: "0.8rem", padding: "6px 12px", display: "inline-flex", alignItems: "center", gap: "6px" }}
                    >
                      <DownloadCloud size={14} color="var(--accent-light)" />
                      <span>Download Direct</span>
                    </button>

                    {/* Wishlist Status */}
                    {rec.in_wishlist ? (
                      <button
                        className="btn btn-secondary"
                        disabled
                        style={{ opacity: 0.6, cursor: "default", fontSize: "0.8rem", padding: "6px 12px" }}
                      >
                        <Check size={14} color="var(--success)" />
                        <span>In Wishlist</span>
                      </button>
                    ) : (
                      <button
                        className="btn btn-primary"
                        onClick={() => onAddToWishlist(rec)}
                        style={{ fontSize: "0.8rem", padding: "6px 12px", display: "inline-flex", alignItems: "center", gap: "6px" }}
                      >
                        <Bookmark size={14} />
                        <span>Add to Wishlist</span>
                      </button>
                    )}
                  </div>
                </div>

                {/* Inline Card Preview Player (Only rendered on the active track) */}
                {isSelected && (
                  <div
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: "12px",
                      padding: "8px 12px",
                      backgroundColor: "rgba(0, 0, 0, 0.2)",
                      borderRadius: "8px",
                      border: "1px solid rgba(99, 102, 241, 0.25)",
                    }}
                  >
                    <button
                      className="btn btn-primary"
                      onClick={() => handleTogglePreview(rec)}
                      style={{
                        width: "28px",
                        height: "28px",
                        padding: 0,
                        borderRadius: "50%",
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "center",
                        flexShrink: 0,
                      }}
                      title={isPlayingThis ? "Pause" : "Resume"}
                    >
                      {isPlayingThis ? <Pause size={13} /> : <Play size={13} style={{ marginLeft: "1px" }} />}
                    </button>

                    <span style={{ fontSize: "0.74rem", color: "var(--text-dim)", minWidth: "30px" }}>
                      {formatSeconds(previewCurrentTime)}
                    </span>

                    {/* Interactive Scrubbable Progress Bar */}
                    <div
                      style={{
                        flex: 1,
                        height: "6px",
                        backgroundColor: "rgba(255, 255, 255, 0.1)",
                        borderRadius: "3px",
                        overflow: "hidden",
                        cursor: "pointer",
                        position: "relative",
                      }}
                      onClick={handleSeek}
                      title="Click to seek preview"
                    >
                      <div
                        style={{
                          width: `${previewProgress}%`,
                          height: "100%",
                          backgroundColor: "var(--accent-light)",
                          borderRadius: "3px",
                          transition: "width 0.1s linear",
                        }}
                      />
                    </div>

                    <span style={{ fontSize: "0.74rem", color: "var(--text-dim)", minWidth: "30px" }}>
                      {formatSeconds(activeDuration)}
                    </span>

                    {isFullSongActive ? (
                      <span
                        style={{
                          fontSize: "0.72rem",
                          color: "#10b981",
                          display: "inline-flex",
                          alignItems: "center",
                          gap: "4px",
                          fontWeight: 600,
                          backgroundColor: "rgba(16, 185, 129, 0.12)",
                          padding: "2px 8px",
                          borderRadius: "10px",
                        }}
                      >
                        <Sparkles size={11} />
                        <span>Full Song Stream</span>
                      </span>
                    ) : isResolvingThis ? (
                      <span
                        style={{
                          fontSize: "0.72rem",
                          color: "var(--accent-light)",
                          display: "inline-flex",
                          alignItems: "center",
                          gap: "4px",
                        }}
                      >
                        <RefreshCw size={11} className="animate-spin" />
                        <span>Fetching full track...</span>
                      </span>
                    ) : (
                      <span style={{ fontSize: "0.72rem", color: "var(--text-dim)" }}>
                        30s Preview
                      </span>
                    )}

                    <button
                      className="btn btn-secondary"
                      onClick={handleStopPreview}
                      title="Stop Preview"
                      style={{
                        padding: "3px 8px",
                        fontSize: "0.72rem",
                        color: "var(--text-muted)",
                      }}
                    >
                      Stop
                    </button>
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}

      {/* Floating Bottom Preview Player Bar (Docked cleanly above player bar) */}
      {activePreviewTrack && (
        <div
          style={{
            position: "fixed",
            bottom: "102px",
            left: "272px",
            right: "32px",
            backgroundColor: "rgba(22, 24, 34, 0.95)",
            borderRadius: "12px",
            boxShadow: "0 12px 40px rgba(0, 0, 0, 0.65), 0 0 0 1px rgba(99, 102, 241, 0.4)",
            border: "1px solid var(--accent)",
            padding: "10px 18px",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            gap: "18px",
            zIndex: 60,
            backdropFilter: "blur(16px)",
          }}
        >
          {/* Active Track Info */}
          <div style={{ display: "flex", alignItems: "center", gap: "12px", minWidth: "200px", maxWidth: "320px" }}>
            {activePreviewTrack.cover_art_url ? (
              <img
                src={activePreviewTrack.cover_art_url}
                alt={activePreviewTrack.title}
                style={{ width: "40px", height: "40px", borderRadius: "6px", objectFit: "cover" }}
              />
            ) : (
              <div
                style={{
                  width: "40px",
                  height: "40px",
                  borderRadius: "6px",
                  backgroundColor: "var(--bg-sidebar)",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                }}
              >
                <Music2 size={18} color="var(--accent-light)" />
              </div>
            )}
            <div style={{ minWidth: 0 }}>
              <div
                style={{
                  fontWeight: 600,
                  fontSize: "0.88rem",
                  color: "var(--text-main)",
                  whiteSpace: "nowrap",
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                }}
              >
                {activePreviewTrack.title}
              </div>
              <div
                style={{
                  fontSize: "0.76rem",
                  color: "var(--text-muted)",
                  whiteSpace: "nowrap",
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                }}
              >
                {activePreviewTrack.artist}
              </div>
            </div>
          </div>

          {/* Progress & Controls */}
          <div style={{ flex: 1, display: "flex", alignItems: "center", gap: "12px", maxWidth: "550px" }}>
            <button
              className="btn btn-primary"
              onClick={() => handleTogglePreview(activePreviewTrack)}
              style={{
                width: "34px",
                height: "34px",
                padding: 0,
                borderRadius: "50%",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                flexShrink: 0,
              }}
              title={isPreviewPlaying ? "Pause Preview" : "Play Preview"}
            >
              {isPreviewPlaying ? <Pause size={16} /> : <Play size={16} style={{ marginLeft: "1.5px" }} />}
            </button>

            <span style={{ fontSize: "0.75rem", color: "var(--text-dim)", minWidth: "32px" }}>
              {formatSeconds(previewCurrentTime)}
            </span>

            {/* Preview Progress Bar */}
            <div
              style={{
                flex: 1,
                height: "6px",
                backgroundColor: "rgba(255, 255, 255, 0.12)",
                borderRadius: "3px",
                overflow: "hidden",
                cursor: "pointer",
                position: "relative",
              }}
              onClick={handleSeek}
              title="Click to seek"
            >
              <div
                style={{
                  width: `${previewProgress}%`,
                  height: "100%",
                  backgroundColor: "var(--accent-light)",
                  borderRadius: "3px",
                  transition: "width 0.1s linear",
                }}
              />
            </div>

            <span style={{ fontSize: "0.75rem", color: "var(--text-dim)", minWidth: "32px" }}>
              {formatSeconds(activeDuration)}
            </span>

            {isFullSongActive ? (
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: "5px",
                  color: "#10b981",
                  backgroundColor: "rgba(16, 185, 129, 0.15)",
                  padding: "3px 8px",
                  borderRadius: "12px",
                  fontSize: "0.72rem",
                  fontWeight: 600,
                  whiteSpace: "nowrap",
                }}
              >
                <Sparkles size={12} />
                <span>Full Song Stream</span>
              </div>
            ) : resolvingId === activePreviewTrack.external_track_id ? (
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: "5px",
                  color: "var(--accent-light)",
                  fontSize: "0.72rem",
                  whiteSpace: "nowrap",
                }}
              >
                <RefreshCw size={12} className="animate-spin" />
                <span>Loading Full Track...</span>
              </div>
            ) : (
              <div style={{ display: "flex", alignItems: "center", gap: "4px", color: "var(--accent-light)" }}>
                <Volume2 size={15} />
                <span style={{ fontSize: "0.72rem", fontWeight: 500 }}>Preview</span>
              </div>
            )}
          </div>

          {/* Actions & Close */}
          <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
            <button
              className="btn btn-primary"
              onClick={() => onSearchDirect(activePreviewTrack.artist, activePreviewTrack.title)}
              style={{
                fontSize: "0.78rem",
                padding: "6px 14px",
                display: "inline-flex",
                alignItems: "center",
                gap: "6px",
                fontWeight: 600,
              }}
              title="Liked this track? Download the full audio directly to your library"
            >
              <DownloadCloud size={14} />
              <span>Download This Song</span>
            </button>

            <button
              className="btn btn-secondary"
              onClick={handleStopPreview}
              title="Close Preview (Esc)"
              style={{
                padding: "6px",
                borderRadius: "6px",
                color: "var(--text-dim)",
              }}
            >
              <X size={16} />
            </button>
          </div>
        </div>
      )}
    </div>
  );
};
