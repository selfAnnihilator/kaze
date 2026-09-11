import React from "react";
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
  ThumbsDown,
  Music,
} from "lucide-react";
import { PlaybackState, Track } from "../types";

interface NowPlayingBarProps {
  playbackState: PlaybackState;
  currentTrack?: Track;
  onPlayPause: () => void;
  onNext: () => void;
  onPrevious: () => void;
  onSeek: (seconds: number) => void;
  onVolumeChange: (volume: number) => void;
  onToggleMute: () => void;
  onToggleRepeat: () => void;
  onToggleShuffle: () => void;
  onLike: (trackId: string) => void;
  onDislike: (trackId: string) => void;
  onRemoveFeedback: (trackId: string) => void;
}

const formatTime = (seconds: number): string => {
  if (isNaN(seconds) || seconds < 0) return "0:00";
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins}:${secs < 10 ? "0" : ""}${secs}`;
};

export const NowPlayingBar: React.FC<NowPlayingBarProps> = ({
  playbackState,
  currentTrack,
  onPlayPause,
  onNext,
  onPrevious,
  onSeek,
  onVolumeChange,
  onToggleMute,
  onToggleRepeat,
  onToggleShuffle,
  onLike,
  onDislike,
  onRemoveFeedback,
}) => {
  const duration = playbackState.duration_secs || currentTrack?.duration_secs || 0;
  const position = playbackState.position_secs || 0;

  return (
    <footer className="player-bar">
      {/* Left: Track Information & Quick Feedback */}
      <div className="player-track-info">
        <div className="player-artwork">
          <Music size={24} />
        </div>
        <div style={{ overflow: "hidden" }}>
          <div
            style={{
              fontWeight: 600,
              fontSize: "0.92rem",
              whiteSpace: "nowrap",
              overflow: "hidden",
              textOverflow: "ellipsis",
            }}
          >
            {currentTrack?.title || "No Track Selected"}
          </div>
          <div
            style={{
              fontSize: "0.8rem",
              color: "var(--text-muted)",
              whiteSpace: "nowrap",
              overflow: "hidden",
              textOverflow: "ellipsis",
            }}
          >
            {currentTrack?.artist_name || "Select a track to start playback"}
          </div>
        </div>
        {currentTrack && (
          <div style={{ display: "flex", gap: "6px", marginLeft: "6px" }}>
            <button
              className="player-icon-btn"
              title={currentTrack.manual_like === 1 ? "Unlike track" : "Like track"}
              onClick={() =>
                currentTrack.manual_like === 1
                  ? onRemoveFeedback(currentTrack.id)
                  : onLike(currentTrack.id)
              }
              style={{ color: currentTrack.manual_like === 1 ? "#ef4444" : "var(--text-muted)" }}
            >
              <Heart size={16} fill={currentTrack.manual_like === 1 ? "#ef4444" : "none"} />
            </button>
            <button
              className="player-icon-btn"
              title={currentTrack.manual_like === -1 ? "Remove dislike" : "Dislike track"}
              onClick={() =>
                currentTrack.manual_like === -1
                  ? onRemoveFeedback(currentTrack.id)
                  : onDislike(currentTrack.id)
              }
              style={{ color: currentTrack.manual_like === -1 ? "#f59e0b" : "var(--text-muted)" }}
            >
              <ThumbsDown size={16} fill={currentTrack.manual_like === -1 ? "#f59e0b" : "none"} />
            </button>
          </div>
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
          <button className="player-icon-btn" title="Previous Track" onClick={onPrevious}>
            <SkipBack size={18} />
          </button>
          <button
            className="play-pause-btn"
            title={playbackState.is_playing ? "Pause" : "Play"}
            onClick={onPlayPause}
          >
            {playbackState.is_playing ? <Pause size={18} /> : <Play size={18} />}
          </button>
          <button className="player-icon-btn" title="Next Track" onClick={onNext}>
            <SkipForward size={18} />
          </button>
          <button
            className={`player-icon-btn ${playbackState.repeat_mode !== "off" ? "active" : ""}`}
            title={`Repeat: ${playbackState.repeat_mode}`}
            onClick={onToggleRepeat}
          >
            {playbackState.repeat_mode === "one" ? <Repeat1 size={16} /> : <Repeat size={16} />}
          </button>
        </div>

        <div className="progress-container">
          <span className="time-label">{formatTime(position)}</span>
          <input
            type="range"
            className="scrubber"
            min={0}
            max={duration > 0 ? duration : 100}
            value={position}
            onChange={(e) => onSeek(parseFloat(e.target.value))}
          />
          <span className="time-label">{formatTime(duration)}</span>
        </div>
      </div>

      {/* Right: Volume Controls */}
      <div className="player-volume">
        <button className="player-icon-btn" onClick={onToggleMute}>
          {playbackState.is_muted ? <VolumeX size={18} /> : <Volume2 size={18} />}
        </button>
        <input
          type="range"
          className="scrubber"
          style={{ width: "90px" }}
          min={0}
          max={1}
          step={0.01}
          value={playbackState.is_muted ? 0 : playbackState.volume}
          onChange={(e) => onVolumeChange(parseFloat(e.target.value))}
        />
      </div>
    </footer>
  );
};
