import React, { useState } from "react";
import {
  Play,
  Pause,
  SkipBack,
  SkipForward,
  Repeat,
  Repeat1,
  Shuffle,
  Volume2,
  VolumeX,
  Heart,
  Music,
  Download,
  Bookmark,
  BookmarkCheck,
  Mic2,
  Maximize2,
  Minimize2,
} from "lucide-react";
import { PlaybackState, Track } from "../types";
import { usePlaybackProgress } from "../services/playbackProgress";

interface NowPlayingBarProps {
  playbackState: PlaybackState;
  currentTrack?: Track;
  coverArtUrl?: string | null;
  onPlayPause: () => void;
  onNext: () => void;
  onPrevious: () => void;
  onSeek: (seconds: number) => void;
  onVolumeChange: (volume: number) => void;
  onToggleMute: () => void;
  onToggleRepeat: () => void;
  onToggleShuffle: () => void;
  onLike: (trackId: string) => void;
  onLikeOnline?: (track: { id: string; title: string; artist: string; album?: string; cover_art_url?: string; preview_url?: string; duration_secs?: number }) => void;
  isOnlineLiked?: boolean;
  onRemoveFeedback: (trackId: string) => void;
  onDownloadOnlineTrack?: (artist: string, title: string) => void;
  isInPlaylist?: boolean;
  onOpenAddToPlaylist?: (track: {
    id: string;
    title: string;
    artist?: string;
    album?: string;
    cover_art_url?: string;
  }) => void;
  onOpenOrigin?: () => void;
  originName?: string;
  isLyricsActive?: boolean;
  onToggleLyrics?: () => void;
  isFullscreen?: boolean;
  onToggleFullscreen?: () => void;
}

const formatTime = (seconds: number): string => {
  if (isNaN(seconds) || seconds < 0) return "0:00";
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins}:${secs < 10 ? "0" : ""}${secs}`;
};

const NowPlayingProgressBar: React.FC<{
  duration: number;
  onSeek: (seconds: number) => void;
}> = React.memo(({ duration: propDuration, onSeek }) => {
  const { position, duration: progressDuration } = usePlaybackProgress(propDuration);
  const [seekingValue, setSeekingValue] = useState<number | null>(null);

  const duration = propDuration > 0 ? propDuration : progressDuration;
  const displayPos = seekingValue !== null ? seekingValue : position;
  const seekProgress = duration > 0 ? Math.min(100, Math.max(0, (displayPos / duration) * 100)) : 0;

  return (
    <div className="progress-container">
      <span className="time-label">{formatTime(displayPos)}</span>
      <input
        type="range"
        className="scrubber"
        min={0}
        max={duration > 0 ? duration : 100}
        step={0.5}
        value={displayPos}
        style={{ "--range-progress": `${seekProgress}%` } as React.CSSProperties}
        onMouseDown={() => setSeekingValue(position)}
        onTouchStart={() => setSeekingValue(position)}
        onChange={(e) => setSeekingValue(parseFloat(e.target.value))}
        onMouseUp={(e) => {
          const val = parseFloat((e.target as HTMLInputElement).value);
          onSeek(val);
          setSeekingValue(null);
        }}
        onTouchEnd={() => {
          if (seekingValue !== null) {
            onSeek(seekingValue);
            setSeekingValue(null);
          }
        }}
        onKeyUp={(e) => {
          const val = parseFloat((e.target as HTMLInputElement).value);
          onSeek(val);
          setSeekingValue(null);
        }}
      />
      <span className="time-label">{formatTime(duration)}</span>
    </div>
  );
});

export const NowPlayingBar: React.FC<NowPlayingBarProps> = React.memo(({
  playbackState,
  currentTrack,
  coverArtUrl,
  onPlayPause,
  onNext,
  onPrevious,
  onSeek,
  onVolumeChange,
  onToggleMute,
  onToggleRepeat,
  onToggleShuffle,
  onLike,
  onLikeOnline,
  isOnlineLiked = false,
  onRemoveFeedback,
  onDownloadOnlineTrack,
  isInPlaylist = false,
  onOpenAddToPlaylist,
  onOpenOrigin,
  originName,
  isLyricsActive = false,
  onToggleLyrics,
  isFullscreen = false,
  onToggleFullscreen,
}) => {
  const isOnline = !!currentTrack && (currentTrack.format === "online" || currentTrack.file_path?.startsWith("online://"));
  const activeTitle = currentTrack?.title || "No Track Selected";
  const activeArtist = currentTrack?.artist_name || (currentTrack ? "Unknown Artist" : "Select a track to start playback");
  const activeArtworkUrl = currentTrack?.cover_art_url || coverArtUrl || undefined;
  const isPlaying = playbackState.is_playing;
  const duration = playbackState.duration_secs || currentTrack?.duration_secs || 0;
  const volumeProgress = playbackState.is_muted
    ? 0
    : Math.min(100, Math.max(0, playbackState.volume * 100));

  return (
    <footer className="player-bar">
      {/* Left: Track Information & Quick Feedback */}
      <div className="player-track-info">
        <div
          className="player-artwork"
          style={{
            overflow: "hidden",
            position: "relative",
            width: "48px",
            height: "48px",
            borderRadius: "6px",
            backgroundColor: "rgba(255, 255, 255, 0.05)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            flexShrink: 0,
          }}
        >
          {activeArtworkUrl ? (
            <img
              src={activeArtworkUrl}
              alt={activeTitle}
              style={{ width: "100%", height: "100%", objectFit: "cover", display: "block" }}
            />
          ) : (
            <Music size={24} color="var(--text-dim)" />
          )}
        </div>
        <div className="player-track-metadata">
          <div
            className="player-track-title"
            style={{
              fontWeight: 600,
              fontSize: "0.92rem",
              whiteSpace: "nowrap",
              overflow: "hidden",
              textOverflow: "ellipsis",
            }}
            title={activeTitle}
          >
            <span
              onClick={onOpenOrigin}
              style={{
                overflow: "hidden",
                textOverflow: "ellipsis",
                cursor: onOpenOrigin ? "pointer" : "default",
                textDecoration: onOpenOrigin ? "underline" : "none",
                textDecorationColor: onOpenOrigin ? "rgba(255, 255, 255, 0.4)" : "transparent",
                transition: "color 0.15s ease",
                display: "block",
              }}
              title={
                onOpenOrigin
                  ? `Playing from: ${originName || "Collection"} (Click to open)`
                  : activeTitle
              }
              onMouseEnter={(e) => {
                if (onOpenOrigin) e.currentTarget.style.color = "var(--accent-light)";
              }}
              onMouseLeave={(e) => {
                if (onOpenOrigin) e.currentTarget.style.color = "inherit";
              }}
            >
              {activeTitle}
            </span>
          </div>
          <div
            style={{
              fontSize: "0.8rem",
              color: "var(--text-muted)",
              whiteSpace: "nowrap",
              overflow: "hidden",
              textOverflow: "ellipsis",
            }}
            title={activeArtist}
          >
            {activeArtist}
          </div>
        </div>
        {/* Heart — works for all tracks */}
        {currentTrack && (
          <div style={{ display: "flex", gap: "6px", marginLeft: "6px" }}>
            <button
              className="player-icon-btn"
              title={currentTrack.manual_like === 1 || isOnlineLiked ? "Unlike track" : "Like track"}
              onClick={() => {
                if (currentTrack.manual_like === 1 || isOnlineLiked) {
                  onRemoveFeedback(currentTrack.id);
                } else if (isOnline && onLikeOnline) {
                  onLikeOnline({
                    id: currentTrack.id,
                    title: currentTrack.title,
                    artist: currentTrack.artist_name || "Unknown Artist",
                    album: currentTrack.album_title,
                    cover_art_url: currentTrack.cover_art_url,
                    preview_url: currentTrack.preview_url,
                    duration_secs: currentTrack.duration_secs,
                  });
                } else {
                  onLike(currentTrack.id);
                }
              }}
              style={{ color: currentTrack.manual_like === 1 || isOnlineLiked ? "#ef4444" : "var(--text-muted)" }}
            >
              <Heart size={16} fill={currentTrack.manual_like === 1 || isOnlineLiked ? "#ef4444" : "none"} />
            </button>
          </div>
        )}
        {isOnline && onDownloadOnlineTrack && currentTrack && (
          <button
            className="player-icon-btn"
            title="Download this online track to library"
            onClick={() => onDownloadOnlineTrack(currentTrack.artist_name || "", currentTrack.title)}
            style={{ color: "var(--accent-light)", marginLeft: "4px" }}
          >
            <Download size={16} />
          </button>
        )}
        {onOpenAddToPlaylist && currentTrack && (
          <button
            className="player-icon-btn"
            title={isInPlaylist ? "In playlist (click to manage)" : "Add to playlist"}
            onClick={() => {
              onOpenAddToPlaylist({
                id: currentTrack.id,
                title: currentTrack.title,
                artist: currentTrack.artist_name,
                album: currentTrack.album_title,
                cover_art_url: activeArtworkUrl,
              });
            }}
            style={{
              color: isInPlaylist ? "var(--accent-secondary)" : "var(--text-muted)",
              marginLeft: "4px",
            }}
          >
            {isInPlaylist ? <BookmarkCheck size={16} color="var(--accent-secondary)" /> : <Bookmark size={16} />}
          </button>
        )}
      </div>

      {/* Center: Controls & Scrubber */}
      <div className="player-controls">
        <div className="player-buttons">
          <button
            className={`player-icon-btn ${playbackState.is_shuffled ? "active" : ""}`}
            title="Toggle Shuffle"
            onClick={onToggleShuffle}
          >
            <Shuffle size={16} />
          </button>
          <button
            className="player-icon-btn"
            title="Previous Track"
            onClick={onPrevious}
          >
            <SkipBack size={18} />
          </button>
          <button
            className="play-pause-btn"
            title={isPlaying ? "Pause" : "Play"}
            onClick={onPlayPause}
          >
            {isPlaying ? (
              <Pause size={18} />
            ) : (
              <Play size={18} />
            )}
          </button>
          <button
            className="player-icon-btn"
            title="Next Track"
            onClick={onNext}
          >
            <SkipForward size={18} />
          </button>
          <button
            className={`player-icon-btn ${playbackState.repeat_mode === "one" ? "repeat-active" : ""}`}
            title={playbackState.repeat_mode === "one" ? "Turn off song loop" : "Loop current song"}
            onClick={onToggleRepeat}
          >
            {playbackState.repeat_mode === "one" ? <Repeat1 size={16} /> : <Repeat size={16} />}
          </button>
        </div>


        <NowPlayingProgressBar duration={duration} onSeek={onSeek} />
      </div>

      {/* Right: Lyrics, Volume, and Full Screen Controls */}
      <div className="player-volume" style={{ display: "flex", alignItems: "center", gap: "6px" }}>
        {onToggleLyrics && (
          <button
            className={`player-icon-btn ${isLyricsActive ? "active" : ""}`}
            title={isLyricsActive ? "Hide Lyrics" : "Show Lyrics"}
            onClick={onToggleLyrics}
            style={{
              color: isLyricsActive ? "var(--accent-secondary)" : "var(--text-muted)",
              transition: "all 0.15s ease",
            }}
          >
            <Mic2 size={18} color={isLyricsActive ? "var(--accent-secondary)" : undefined} />
          </button>
        )}

        <button className="player-icon-btn" onClick={onToggleMute}>
          {playbackState.is_muted ? <VolumeX size={18} /> : <Volume2 size={18} />}
        </button>
        <input
          type="range"
          className="scrubber"
          style={{ width: "90px", "--range-progress": `${volumeProgress}%` } as React.CSSProperties}
          min={0}
          max={1}
          step={0.01}
          value={playbackState.is_muted ? 0 : playbackState.volume}
          onChange={(e) => onVolumeChange(parseFloat(e.target.value))}
        />

        {onToggleFullscreen && (
          <button
            className="player-icon-btn"
            title={isFullscreen ? "Exit Full Screen" : "Full Screen"}
            onClick={onToggleFullscreen}
            style={{
              color: isFullscreen ? "#e8d8c9" : "var(--text-muted)",
              marginLeft: "4px",
            }}
          >
            {isFullscreen ? <Minimize2 size={18} /> : <Maximize2 size={18} />}
          </button>
        )}
      </div>
    </footer>
  );
});
