import { Artwork } from "../../../presentation/Artwork";
import { useSongDownloaded } from "../../../services/downloadProgress";
import { DiscoveryRecommendation, DownloadTask } from "../../../types";
import { DownloadButton } from "../../DownloadButton";
import { NotInterestedButton } from "../../NotInterestedButton";
import { Bookmark, BookmarkCheck, CheckCircle2, Heart, Pause, Play, RefreshCw } from "lucide-react";
import { QueueButton } from "../../QueueButton";
import React, { useState } from "react";

export interface DiscoveryTrackCardProps {
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

export const DiscoveryTrackCard: React.FC<DiscoveryTrackCardProps> = ({
  rec,
  isPlaying,
  isLoading,
  isSelected,
  onPlay,
  onStop,
  onArtistClick,
  onAddToWishlist,
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

  // A song from your own library folders: nothing to download.
  const isLocalTrack =
    rec.provider === "library" && !rec.external_track_id.startsWith("itunes:") && !rec.external_track_id.startsWith("online:");
  // Only Kaze's own downloads count, matched by exact recording; badge and button share the rule live.
  const isDownloaded = useSongDownloaded(rec.external_track_id, rec.playback_source?.track_id) && !isLocalTrack;


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
      className="discovery-card-body"
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      onClick={handleCardClick}
      style={{
        backgroundColor: isPlaying
          ? "var(--accent-secondary-a10)"
          : isSelected
          ? "var(--accent-a12)"
          : "var(--bg-card)",
        border: isPlaying
          ? "1.5px solid var(--accent-secondary)"
          : isSelected
          ? "1px solid var(--accent-light)"
          : isHovered
          ? "1px solid var(--overlay-20)"
          : "1px solid var(--border)",
        borderRadius: "var(--radius-12)",
        padding: "var(--space-10)",
        display: "flex",
        flexDirection: "column",
        cursor: "pointer",
        position: "relative",
        transition: "transform var(--duration-base) var(--ease-standard), opacity var(--duration-base) var(--ease-standard)",
        boxShadow: isPlaying
          ? "0 0 20px var(--accent-secondary-a35), 0 6px 18px var(--shade-45)"
          : isHovered
          ? "0 10px 24px var(--shade-40)"
          : isSelected
          ? "0 4px 18px var(--accent-a20)"
          : "0 2px 8px var(--shade-15)",
      }}
    >
      {/* 1. Thumbnail (Biggest element, passport aspect ratio) */}
      <div className="discovery-card-art"
        style={{
          position: "relative",
          width: "100%",
          aspectRatio: "1 / 1",
          borderRadius: "var(--radius-8)",
          overflow: "hidden",
          backgroundColor: "var(--overlay-5)",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
        }}
      >
        <Artwork src={rec.cover_art_url} alt={rec.title} />

        {/* Plain downloaded indicator, matching the completed action below. */}
        {isDownloaded && (
          <div
            style={{
              position: "absolute",
              top: "7px",
              right: "7px",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
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
              backgroundColor: "var(--accent-secondary-a95)",
              borderRadius: "var(--radius-20)",
              padding: "var(--space-2) var(--space-8)",
              display: "flex",
              alignItems: "center",
              gap: "var(--space-6)",
              boxShadow: "0 2px 8px var(--shade-60)",
              zIndex: 2,
            }}
            title="Now Playing"
          >
            <div className="discovery-eq-container" style={{ height: "11px" }}>
              <div className="discovery-eq-bar" style={{ width: "2.5px" }} />
              <div className="discovery-eq-bar" style={{ width: "2.5px" }} />
              <div className="discovery-eq-bar" style={{ width: "2.5px" }} />
            </div>
            <span style={{ fontSize: "var(--fs-10)", fontWeight: "var(--weight-bold)", color: "var(--text-main)", letterSpacing: "0.4px",
              }}>
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
            color: "var(--text-main)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            boxShadow: "0 4px 14px var(--shade-60)",
            cursor: "pointer",
            opacity: isHovered || isPlaying || isLoading ? 1 : 0,
            transform: isHovered || isPlaying || isLoading ? "scale(1)" : "scale(0.85)",
            transition: "opacity var(--duration-base) ease, transform var(--duration-base) ease, background-color var(--duration-base) ease",
            zIndex: 3,
          }}
          title={isPlaying ? "Pause / Stop playback" : "Play"}
        >
          {isLoading ? (
            <RefreshCw size={16} className="spin-animation" />
          ) : isPlaying ? (
            <Pause size={17} fill="var(--text-main)" />
          ) : (
            <Play size={17} fill="var(--text-main)" style={{ marginLeft: "var(--space-2)" }} />
          )}
        </button>

      </div>

      {/* 2. Name (Below thumbnail, bold, smaller than thumbnail) */}
      <div className="discovery-card-title"
        style={{
          marginTop: "var(--space-10)",
          fontWeight: "var(--weight-bold)",
          fontSize: "var(--fs-14)",
          color: isPlaying ? "var(--accent-secondary)" : "var(--text-main)",
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
          marginTop: "var(--space-4)",
          fontWeight: "var(--weight-regular)",
          fontSize: "var(--fs-12)",
          color: "var(--text-muted)",
          whiteSpace: "nowrap",
          overflow: "hidden",
          textOverflow: "ellipsis",
          cursor: "pointer",
          transition: "color var(--duration-fast) ease",
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

      {/* 4. Footer Actions (Download / Queue / Not interested / Playlist) */}
      <div
        style={{
          marginTop: "auto",
          paddingTop: "var(--space-8)",
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
          borderTop: "1px solid var(--overlay-5)",
        }}
      >
        {!isLocalTrack && <DownloadButton key={rec.external_track_id} trackId={rec.external_track_id} sourceId={rec.playback_source?.track_id} title={rec.title} />}

        <div
          style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-2)" }}
        >
          {onEnqueue && <QueueButton title={rec.title} queued={isQueued} onEnqueue={() => onEnqueue(rec)} />}
          <NotInterestedButton trackId={rec.external_track_id} title={rec.title} />
          {isLiked && (
            <span
              title="Liked song"
              aria-label="Liked song"
              style={{
                color: "var(--palette-rose-300)",
                padding: "var(--space-4)",
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
              color: isInPlaylist
                ? "var(--accent-secondary)"
                : "var(--text-dim)",
              padding: "var(--space-4)",
              display: "flex",
              alignItems: "center",
              transition: "color var(--duration-fast) ease",
            }}
            onClick={(e) => {
              e.stopPropagation();
              if (onAddToPlaylist) {
                onAddToPlaylist(rec);
              } else {
                onAddToWishlist(rec);
              }
            }}
            title={
              isInPlaylist ? "In playlist (click to manage)" : "Add to playlist"
            }
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
