import React, { useState } from "react";
import {
  Play,
  Pause,
  Shuffle,
  Plus,
  Check,
  Clock,
  ArrowLeft,
  Music2,
  DownloadCloud,
  Bookmark,
  CheckCircle2,
  RefreshCw,
} from "lucide-react";
import { Track, DiscoveryRecommendation } from "../../types";

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
}

interface CollectionDetailViewProps {
  collection: CollectionData;
  isLoadingTracks?: boolean;
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
}

export const CollectionDetailView: React.FC<CollectionDetailViewProps> = ({
  collection,
  isLoadingTracks = false,
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
}) => {
  const [hoveredRowId, setHoveredRowId] = useState<string | null>(null);

  const tracks = collection.tracks || [];
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
  const heroImage1 = tracks[0]?.cover_art_url;
  const heroImage2 = tracks[1]?.cover_art_url || tracks[0]?.cover_art_url;
  const heroImage3 = tracks[2]?.cover_art_url || tracks[1]?.cover_art_url;

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
            backgroundColor: "rgba(0,0,0,0.45)",
            backdropFilter: "blur(6px)",
            border: "1px solid rgba(255,255,255,0.15)",
            color: "#fff",
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
              color: "#fff",
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
            <span style={{ fontWeight: 700, color: "#fff" }}>SoundFlow</span>
            <span>•</span>
            <span>{tracks.length} {tracks.length === 1 ? "song" : "songs"}</span>
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
              position: "absolute",
              width: "170px",
              height: "170px",
              borderRadius: "50%",
              overflow: "hidden",
              border: "4px solid rgba(255, 255, 255, 0.45)",
              boxShadow: "0 12px 32px rgba(0,0,0,0.6)",
              backgroundColor: "rgba(0,0,0,0.4)",
              zIndex: 3,
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
              backgroundColor: "#10b981",
              border: "none",
              color: "#fff",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              cursor: "pointer",
              boxShadow: "0 6px 20px rgba(16, 185, 129, 0.4)",
              transition: "transform 0.15s ease, background-color 0.15s ease",
            }}
            onMouseEnter={(e) => (e.currentTarget.style.transform = "scale(1.05)")}
            onMouseLeave={(e) => (e.currentTarget.style.transform = "scale(1)")}
            title={isPlaying ? "Pause Collection" : "Play Collection"}
          >
            {isPlaying ? (
              <Pause size={24} fill="#fff" />
            ) : (
              <Play size={24} fill="#fff" style={{ marginLeft: "3px" }} />
            )}
          </button>

          {/* Shuffle Button */}
          {onShuffleAll && (
            <button
              type="button"
              onClick={onShuffleAll}
              style={{
                background: "none",
                border: "none",
                color: "var(--text-muted)",
                cursor: "pointer",
                padding: "8px",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                transition: "color 0.15s ease",
              }}
              onMouseEnter={(e) => (e.currentTarget.style.color = "#fff")}
              onMouseLeave={(e) => (e.currentTarget.style.color = "var(--text-muted)")}
              title="Shuffle collection"
            >
              <Shuffle size={22} />
            </button>
          )}

          {/* Save / Add to Library Button */}
          {onSaveToPlaylists && (
            <button
              type="button"
              onClick={() => onSaveToPlaylists(collection)}
              style={{
                background: "none",
                border: isSaved ? "1px solid #10b981" : "1px solid rgba(255, 255, 255, 0.2)",
                borderRadius: "50%",
                width: "36px",
                height: "36px",
                color: isSaved ? "#10b981" : "var(--text-muted)",
                cursor: "pointer",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                transition: "all 0.15s ease",
              }}
              title={isSaved ? "Saved to your playlists" : "Save collection to your library"}
            >
              {isSaved ? <Check size={18} color="#10b981" /> : <Plus size={18} />}
            </button>
          )}
        </div>
      </div>

      {/* 3. Track Table */}
      {isLoadingTracks ? (
        <div style={{ textAlign: "center", padding: "60px 20px", color: "var(--text-dim)" }}>
          <RefreshCw size={36} className="animate-spin" color="var(--accent-light)" style={{ margin: "0 auto 12px" }} />
          <p style={{ fontSize: "1rem", fontWeight: 600, color: "var(--text-main)" }}>Loading collection songs...</p>
        </div>
      ) : tracks.length === 0 ? (
        <div
          className="content-card"
          style={{ textAlign: "center", padding: "50px 20px", color: "var(--text-dim)", borderStyle: "dashed" }}
        >
          <Music2 size={36} color="var(--accent-light)" style={{ marginBottom: "12px" }} />
          <p style={{ fontSize: "1.05rem", fontWeight: 600, color: "var(--text-main)" }}>No songs in this collection yet</p>
        </div>
      ) : (
        <div
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
              gridTemplateColumns: "48px 1fr 1fr 70px 90px",
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

            return (
              <div
                key={track.id || idx}
                onMouseEnter={() => setHoveredRowId(track.id)}
                onMouseLeave={() => setHoveredRowId(null)}
                onClick={() => onPlayTrack(track)}
                style={{
                  display: "grid",
                  gridTemplateColumns: "48px 1fr 1fr 70px 90px",
                  padding: "10px 18px",
                  alignItems: "center",
                  fontSize: "0.88rem",
                  color: isPlayingThis ? "#10b981" : "var(--text-main)",
                  backgroundColor: isPlayingThis
                    ? "rgba(16, 185, 129, 0.08)"
                    : isHovered
                    ? "rgba(255, 255, 255, 0.04)"
                    : "transparent",
                  borderBottom: "1px solid rgba(255, 255, 255, 0.03)",
                  cursor: "pointer",
                  transition: "background-color 0.15s ease",
                }}
              >
                {/* Index / Play icon */}
                <div style={{ color: isPlayingThis ? "#10b981" : "var(--text-dim)", fontSize: "0.85rem", fontWeight: 500 }}>
                  {isPlayingThis ? (
                    isHovered ? (
                      <Pause size={15} fill="#10b981" />
                    ) : (
                      <div className="discovery-eq-container" style={{ height: "13px" }}>
                        <div className="discovery-eq-bar" style={{ width: "2.5px", backgroundColor: "#10b981" }} />
                        <div className="discovery-eq-bar" style={{ width: "2.5px", backgroundColor: "#10b981" }} />
                        <div className="discovery-eq-bar" style={{ width: "2.5px", backgroundColor: "#10b981" }} />
                      </div>
                    )
                  ) : isHovered ? (
                    <Play size={15} fill="#fff" />
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
                        color: isPlayingThis ? "#10b981" : "#fff",
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
                  {track.is_downloaded ? (
                    <span title="Downloaded in local library" style={{ color: "#10b981", display: "inline-flex" }}>
                      <CheckCircle2 size={16} />
                    </span>
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

                  {onAddToWishlist && (
                    <button
                      type="button"
                      onClick={() => onAddToWishlist(track)}
                      style={{
                        background: "none",
                        border: "none",
                        color: "var(--text-dim)",
                        cursor: "pointer",
                        padding: "4px",
                        display: "flex",
                      }}
                      title="Add to wishlist"
                    >
                      <Bookmark size={15} />
                    </button>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};
