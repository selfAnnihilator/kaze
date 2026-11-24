import { memo, type Ref } from "react";
import { Heart, Pause, Play, SkipForward } from "lucide-react";
import { usePlaybackProgress } from "../../services/playbackProgress";
import { Sleeve } from "./Sleeve";
import type { Track } from "../../types";

function MiniProgress() {
  const { position, duration } = usePlaybackProgress();
  const ratio = duration > 0 ? Math.min(1, Math.max(0, position / duration)) : 0;
  return (
    <div className="m-minideck__progress-track" aria-hidden="true">
      <div className="m-minideck__progress-fill" style={{ transform: `scaleX(${ratio})` }} />
    </div>
  );
}

/**
 * Brutalist Mini Deck (Figma 1:7800):
 * 67px total height, 3px top progress bar, 44x44 sleeve,
 * Inter 15px title, Courier 10px meta, 52x44 solid coral play block.
 */
export const MiniDeck = memo(function MiniDeck({
  track, coverArtUrl, isPlaying, isLoading, liked, canLike = true, onOpen, onPlayPause, onNext, onLike, rootRef,
}: {
  track?: Track; coverArtUrl?: string | null; isPlaying: boolean; isLoading: boolean; liked: boolean;
  onOpen: () => void; onPlayPause: () => void; onNext?: () => void; onLike?: () => void;
  /** False for songs from the user's own folders (no likes). */
  canLike?: boolean;
  /** The bar itself, so dragging it up can pull The Deck open. */
  rootRef?: Ref<HTMLDivElement>;
}) {
  if (!track) return null;
  const isLiked = liked;

  return (
    <div className="m-minideck" ref={rootRef}>
      <MiniProgress />
      <div className="m-minideck__row">
        <button type="button" className="m-minideck__info" onClick={onOpen} aria-label="Open The Deck">
          <Sleeve size="mini" src={coverArtUrl} trackId={track.id} title={track.title} />
          <div className="m-minideck__text">
            <span className="m-minideck__title">{track.title}</span>
            <span className="m-minideck__meta">
              {(track.artist_name || "UNKNOWN").toUpperCase()} · THE DECK
            </span>
          </div>
        </button>
        <div className="m-minideck__actions">
          {onLike && (
            <button
              type="button"
              className={`m-minideck__btn-icon ${isLiked ? "is-liked" : ""}`}
              onClick={onLike}
              disabled={!canLike}
              aria-label={isLiked ? "Unlike" : "Like"}
            >
              <Heart size={20} strokeWidth={2} fill={isLiked ? "currentColor" : "none"} className="m-minideck__heart-icon" />
            </button>
          )}
          <button
            type="button"
            className="m-minideck__btn-play"
            disabled={isLoading}
            onClick={onPlayPause}
            aria-label={isPlaying ? "Pause" : "Play"}
          >
            {isPlaying ? <Pause size={20} strokeWidth={2} /> : <Play size={20} strokeWidth={2} />}
          </button>
          {onNext && (
            <button
              type="button"
              className="m-minideck__btn-icon"
              onClick={onNext}
              aria-label="Next track"
            >
              <SkipForward size={20} strokeWidth={2} />
            </button>
          )}
        </div>
      </div>
    </div>
  );
});
