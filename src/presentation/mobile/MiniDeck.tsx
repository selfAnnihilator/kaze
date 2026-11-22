import { memo } from "react";
import { Pause, Play } from "lucide-react";
import { usePlaybackProgress } from "../../services/playbackProgress";
import { Artwork } from "../Artwork";
import { MobileIconButton } from "./primitives";
import type { Track } from "../../types";

/** Thin progress line. The only subscriber to position ticks in the mini deck. */
function MiniProgress() {
  const { position, duration } = usePlaybackProgress();
  const ratio = duration > 0 ? Math.min(1, Math.max(0, position / duration)) : 0;
  return <span className="m-minideck__progress" style={{ transform: `scaleX(${ratio})` }} aria-hidden="true" />;
}

/** Collapsed player above the tab bar; tapping it opens the deck. */
export const MiniDeck = memo(function MiniDeck({
  track, coverArtUrl, isPlaying, isLoading, onOpen, onPlayPause,
}: {
  track?: Track; coverArtUrl?: string | null; isPlaying: boolean; isLoading: boolean;
  onOpen: () => void; onPlayPause: () => void;
}) {
  if (!track) return null;
  return (
    <div className="m-minideck">
      <button type="button" className="m-minideck__main" onClick={onOpen} aria-label="Open the deck">
        <Artwork src={coverArtUrl} trackId={track.id} alt="" className="m-minideck__art" />
        <span className="m-minideck__text">
          <span className="m-minideck__title">{track.title}</span>
          <span className="m-minideck__meta">{track.artist_name ?? ""}</span>
        </span>
      </button>
      <MobileIconButton label={isPlaying ? "Pause" : "Play"} disabled={isLoading} onClick={onPlayPause}>
        {isPlaying ? <Pause size={22} /> : <Play size={22} />}
      </MobileIconButton>
      <MiniProgress />
    </div>
  );
});
