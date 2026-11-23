import { memo } from "react";
import { usePlaybackProgress } from "../../services/playbackProgress";
import { formatDuration } from "./format";

/** Elapsed time of the playing song; a leaf so position ticks never re-render its parent. */
export const PlaybackTime = memo(function PlaybackTime() {
  const { position } = usePlaybackProgress();
  return <span className="m-track-row__time m-playback-time">{formatDuration(position)}</span>;
});
