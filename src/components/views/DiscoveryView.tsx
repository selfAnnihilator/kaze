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

interface DiscoveryViewProps {
  recommendations: DiscoveryRecommendation[];
  onAddToWishlist: (rec: DiscoveryRecommendation) => void;
  onSearchDirect: (artist: string, title: string) => void;
  onRefresh?: () => Promise<void>;
}

export const DiscoveryView: React.FC<DiscoveryViewProps> = ({
  recommendations,
  onAddToWishlist,
  onSearchDirect,
  onRefresh,
}) => {
  const [filter, setFilter] = useState<"ALL" | "TRENDING" | "GENRE" | "SIMILAR">("ALL");
  const [refreshing, setRefreshing] = useState(false);

  // Audio Preview State
  const [playingPreviewId, setPlayingPreviewId] = useState<string | null>(null);
  const [isPreviewPlaying, setIsPreviewPlaying] = useState<boolean>(false);
  const [previewProgress, setPreviewProgress] = useState<number>(0);
  const [previewCurrentTime, setPreviewCurrentTime] = useState<number>(0);
  const [activePreviewTrack, setActivePreviewTrack] = useState<DiscoveryRecommendation | null>(null);

  const audioRef = useRef<HTMLAudioElement | null>(null);

  // Clean up audio playback when component unmounts
  useEffect(() => {
    return () => {
      if (audioRef.current) {
        audioRef.current.pause();
        audioRef.current.src = "";
        audioRef.current = null;
      }
    };
  }, []);

  const handleTogglePreview = (rec: DiscoveryRecommendation) => {
    if (!rec.preview_url) return;

    if (playingPreviewId === rec.external_track_id) {
      if (isPreviewPlaying) {
        audioRef.current?.pause();
        setIsPreviewPlaying(false);
      } else {
        audioRef.current?.play().catch((err) => {
          console.warn("Playback prevented:", err);
        });
        setIsPreviewPlaying(true);
      }
      return;
    }

    // Stop current audio if playing
    if (audioRef.current) {
      audioRef.current.pause();
      audioRef.current.src = "";
    }

    const audio = new Audio(rec.preview_url);
    audioRef.current = audio;
    setPlayingPreviewId(rec.external_track_id);
    setActivePreviewTrack(rec);
    setIsPreviewPlaying(true);
    setPreviewProgress(0);
    setPreviewCurrentTime(0);

    audio.ontimeupdate = () => {
      if (audio.duration && !isNaN(audio.duration)) {
        setPreviewProgress((audio.currentTime / audio.duration) * 100);
        setPreviewCurrentTime(audio.currentTime);
      }
    };

    audio.onended = () => {
      setIsPreviewPlaying(false);
      setPreviewProgress(0);
      setPreviewCurrentTime(0);
      setPlayingPreviewId(null);
    };

    audio.onerror = () => {
      setIsPreviewPlaying(false);
      setPlayingPreviewId(null);
    };

    audio.play().catch((err) => {
      console.warn("Audio play prevented:", err);
      setIsPreviewPlaying(false);
    });
  };

  const handleStopPreview = () => {
    if (audioRef.current) {
      audioRef.current.pause();
      audioRef.current.src = "";
      audioRef.current = null;
    }
    setIsPreviewPlaying(false);
    setPlayingPreviewId(null);
    setActivePreviewTrack(null);
    setPreviewProgress(0);
    setPreviewCurrentTime(0);
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
    <div style={{ display: "flex", flexDirection: "column", gap: "20px", paddingBottom: activePreviewTrack ? "90px" : "20px" }}>
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

            return (
              <div
                key={rec.external_track_id}
                className="content-card"
                style={{
                  marginBottom: 0,
                  padding: "14px 18px",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  gap: "16px",
                  border: isSelected ? "1px solid var(--accent)" : undefined,
                  backgroundColor: isSelected ? "rgba(99, 102, 241, 0.05)" : undefined,
                  transition: "all 0.2s ease",
                }}
              >
                <div style={{ display: "flex", alignItems: "center", gap: "14px", flex: 1, minWidth: 0 }}>
                  {/* Album Cover Thumbnail with interactive preview overlay */}
                  <div
                    style={{
                      position: "relative",
                      width: "48px",
                      height: "48px",
                      borderRadius: "6px",
                      overflow: "hidden",
                      flexShrink: 0,
                      cursor: rec.preview_url ? "pointer" : "default",
                    }}
                    onClick={() => rec.preview_url && handleTogglePreview(rec)}
                    title={rec.preview_url ? (isPlayingThis ? "Pause Preview" : "Play 30s Preview") : undefined}
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

                    {/* Interactive Play/Pause hover overlay for preview */}
                    {rec.preview_url && (
                      <div
                        style={{
                          position: "absolute",
                          inset: 0,
                          backgroundColor: isPlayingThis ? "rgba(0, 0, 0, 0.55)" : "rgba(0, 0, 0, 0.35)",
                          display: "flex",
                          alignItems: "center",
                          justifyContent: "center",
                          opacity: isSelected ? 1 : 0.8,
                          transition: "opacity 0.2s ease",
                        }}
                      >
                        {isPlayingThis ? (
                          <Pause size={20} color="#fff" />
                        ) : (
                          <Play size={20} color="#fff" style={{ marginLeft: "2px" }} />
                        )}
                      </div>
                    )}
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

                {/* Actions */}
                <div style={{ display: "flex", gap: "8px", alignItems: "center", flexShrink: 0 }}>
                  {/* 30s Audio Preview Button */}
                  {rec.preview_url && (
                    <button
                      className={`btn ${isSelected ? "btn-primary" : "btn-secondary"}`}
                      onClick={() => handleTogglePreview(rec)}
                      title={isPlayingThis ? "Pause Preview (30s)" : "Preview Track (30s snippet)"}
                      style={{
                        fontSize: "0.8rem",
                        padding: "6px 12px",
                        display: "inline-flex",
                        alignItems: "center",
                        gap: "6px",
                      }}
                    >
                      {isPlayingThis ? (
                        <>
                          <Pause size={14} />
                          <span>Pause Preview</span>
                        </>
                      ) : (
                        <>
                          <Headphones size={14} color={isSelected ? "#fff" : "var(--accent-light)"} />
                          <span>Preview</span>
                        </>
                      )}
                    </button>
                  )}

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
            );
          })}
        </div>
      )}

      {/* Floating Audio Preview Player Bar */}
      {activePreviewTrack && (
        <div
          style={{
            position: "fixed",
            bottom: "85px",
            left: "260px",
            right: "24px",
            backgroundColor: "var(--bg-card)",
            borderRadius: "12px",
            boxShadow: "0 8px 32px rgba(0, 0, 0, 0.45)",
            border: "1px solid var(--accent)",
            padding: "12px 18px",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            gap: "18px",
            zIndex: 90,
            backdropFilter: "blur(12px)",
          }}
        >
          {/* Active Track Info */}
          <div style={{ display: "flex", alignItems: "center", gap: "12px", minWidth: "220px", maxWidth: "340px" }}>
            {activePreviewTrack.cover_art_url ? (
              <img
                src={activePreviewTrack.cover_art_url}
                alt={activePreviewTrack.title}
                style={{ width: "42px", height: "42px", borderRadius: "6px", objectFit: "cover" }}
              />
            ) : (
              <div
                style={{
                  width: "42px",
                  height: "42px",
                  borderRadius: "6px",
                  backgroundColor: "var(--bg-sidebar)",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                }}
              >
                <Music2 size={20} color="var(--accent-light)" />
              </div>
            )}
            <div style={{ minWidth: 0 }}>
              <div
                style={{
                  fontWeight: 600,
                  fontSize: "0.9rem",
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
                  fontSize: "0.78rem",
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
          <div style={{ flex: 1, display: "flex", alignItems: "center", gap: "14px", maxWidth: "550px" }}>
            <button
              className="btn btn-primary"
              onClick={() => handleTogglePreview(activePreviewTrack)}
              style={{
                width: "36px",
                height: "36px",
                padding: 0,
                borderRadius: "50%",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                flexShrink: 0,
              }}
              title={isPreviewPlaying ? "Pause Preview" : "Play Preview"}
            >
              {isPreviewPlaying ? <Pause size={18} /> : <Play size={18} style={{ marginLeft: "2px" }} />}
            </button>

            <span style={{ fontSize: "0.75rem", color: "var(--text-dim)", minWidth: "35px" }}>
              {formatSeconds(previewCurrentTime)}
            </span>

            {/* Preview Progress Bar */}
            <div
              style={{
                flex: 1,
                height: "5px",
                backgroundColor: "var(--border)",
                borderRadius: "3px",
                overflow: "hidden",
                cursor: "pointer",
                position: "relative",
              }}
              onClick={(e) => {
                if (audioRef.current && audioRef.current.duration) {
                  const rect = e.currentTarget.getBoundingClientRect();
                  const clickPos = (e.clientX - rect.left) / rect.width;
                  audioRef.current.currentTime = clickPos * audioRef.current.duration;
                }
              }}
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

            <span style={{ fontSize: "0.75rem", color: "var(--text-dim)", minWidth: "35px" }}>
              {formatSeconds(30)}
            </span>

            <div style={{ display: "flex", alignItems: "center", gap: "4px", color: "var(--text-dim)" }}>
              <Volume2 size={16} />
              <span style={{ fontSize: "0.72rem" }}>Preview</span>
            </div>
          </div>

          {/* Actions & Close */}
          <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
            <button
              className="btn btn-secondary"
              onClick={() => onSearchDirect(activePreviewTrack.artist, activePreviewTrack.title)}
              style={{ fontSize: "0.78rem", padding: "5px 10px", display: "inline-flex", alignItems: "center", gap: "5px" }}
            >
              <DownloadCloud size={13} color="var(--accent-light)" />
              <span>Download Direct</span>
            </button>

            <button
              className="btn btn-secondary"
              onClick={handleStopPreview}
              title="Close Preview Bar"
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
