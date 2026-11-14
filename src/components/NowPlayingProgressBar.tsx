import React, { useState } from "react";
import { usePlaybackProgress } from "../services/playbackProgress";

/** m:ss, or 0:00 for unknown times. */
const formatTime = (seconds: number): string => {
  if (isNaN(seconds) || seconds < 0) return "0:00";
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60);
  return `${mins}:${secs < 10 ? "0" : ""}${secs}`;
};

/**
 * The seek bar. It alone subscribes to playback progress, so position ticks
 * never re-render the player around it.
 */
export const NowPlayingProgressBar: React.FC<{
  duration: number;
  canSeek?: boolean;
  layout?: "bar" | "deck";
  onSeek: (seconds: number) => void;
}> = React.memo(({ duration: propDuration, canSeek = true, layout = "bar", onSeek }) => {
  const { position, duration: progressDuration } =
    usePlaybackProgress(propDuration);
  const [seekingValue, setSeekingValue] = useState<number | null>(null);

  const duration = progressDuration;
  canSeek = canSeek && duration > 0;
  const displayPos = seekingValue !== null ? seekingValue : position;
  const seekProgress =
    duration > 0
      ? Math.min(100, Math.max(0, (displayPos / duration) * 100))
      : 0;

  return (
    <div className={layout === "deck" ? "press-position" : "progress-container"}>
      {layout === "deck" ? <label>POSITION <output>{formatTime(displayPos)}{duration > 0 ? ` / ${formatTime(duration)}` : ""}</output></label> : <span className="time-label">{formatTime(displayPos)}</span>}
      <input
        type="range"
        className={layout === "deck" ? undefined : `scrubber ${!canSeek ? "disabled" : ""}`}
        aria-label="Seek"
        onPointerCancel={() => setSeekingValue(null)}
        disabled={!canSeek}
        title={
          !canSeek ? "Seeking unavailable while audio is streaming" : undefined
        }
        min={0}
        max={duration > 0 ? duration : 100}
        step={0.5}
        value={displayPos}
        style={
          {
            "--range-progress": `${seekProgress}%`,
            cursor: !canSeek ? "not-allowed" : "pointer",
            opacity: !canSeek ? 0.6 : 1,
          } as React.CSSProperties
        }
        onPointerDown={() => {
          if (canSeek) setSeekingValue(position);
        }}
        onChange={(e) => {
          if (canSeek) setSeekingValue(parseFloat(e.target.value));
        }}
        onPointerUp={(e) => {
          if (!canSeek) return;
          const val = parseFloat((e.target as HTMLInputElement).value);
          onSeek(val);
          setSeekingValue(null);
        }}
        onKeyUp={(e) => {
          if (!canSeek) return;
          const val = parseFloat((e.target as HTMLInputElement).value);
          onSeek(val);
          setSeekingValue(null);
        }}
      />
      {layout !== "deck" && <span className="time-label">{duration > 0 ? formatTime(duration) : "—"}</span>}
    </div>
  );
});
