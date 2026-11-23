import { memo, useState } from "react";
import { BookmarkPlus, Download, Heart, ListMusic, Mic, Radio, Repeat1, Share2, ThumbsDown, Volume1, Volume2, VolumeX } from "lucide-react";
import { DownloadButton } from "../../components/DownloadButton";
import { useSongDownloaded } from "../../services/downloadProgress";
import { usePlaybackProgress } from "../../services/playbackProgress";
import { isOnlineTrack } from "../../trackKind";
import { formatDuration } from "./format";
import { isPlayingLiked, togglePlayingLike } from "./likeTrack";
import { useMobileNav } from "./mobileNav";
import { openTrackSheet } from "./sheets/trackSheet";
import type { MobileDeckExtras, MobilePlayer } from "./types";
import { shareText } from "./androidBridge";

type BarEvent = React.PointerEvent<HTMLDivElement>;
const clamp01 = (n: number) => Math.max(0, Math.min(1, n));
const ratioAt = (e: BarEvent) => {
  const r = e.currentTarget.getBoundingClientRect();
  return clamp01((e.clientX - r.left) / r.width);
};

/**
 * Tap or drag to seek: while the finger is down the bar shows where it is
 * (`dragRatio`), and the seek happens once, when the finger lifts.
 */
function useSeekDrag(duration: number, onSeek?: (secs: number) => void) {
  const [dragRatio, setDragRatio] = useState<number | null>(null);
  const enabled = !!onSeek && duration > 0;
  const onPointerDown = (e: BarEvent) => {
    if (!enabled) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    setDragRatio(ratioAt(e));
  };
  const onPointerMove = (e: BarEvent) => { if (dragRatio !== null) setDragRatio(ratioAt(e)); };
  const onPointerUp = (e: BarEvent) => {
    if (dragRatio === null) return;
    setDragRatio(null);
    onSeek?.(ratioAt(e) * duration);
  };
  const onPointerCancel = () => setDragRatio(null);
  return { dragRatio, handlers: { onPointerDown, onPointerMove, onPointerUp, onPointerCancel } };
}

/** Position bar (Figma E1): a leaf, so position ticks re-render only this. Tap or drag to seek. */
export function DeckProgressBar({ duration, onSeek }: { duration: number; onSeek?: (secs: number) => void }) {
  const { position } = usePlaybackProgress();
  const { dragRatio, handlers } = useSeekDrag(duration, onSeek);
  const ratio = dragRatio ?? (duration > 0 ? clamp01(position / duration) : 0);
  const percent = ratio * 100;
  const shown = dragRatio === null ? position : dragRatio * duration;

  return (
    <div className="m-deck-progress">
      <div className="m-deck-progress__header">
        <span className="m-deck-progress__label">POSITION</span>
        <span className="m-deck-progress__time">{formatDuration(shown)} / {formatDuration(duration)}</span>
      </div>
      <div className={`m-deck-progress__track ${dragRatio === null ? "" : "is-dragging"}`} {...handlers} role="slider" aria-label="Seek"
        aria-valuemin={0} aria-valuemax={Math.round(duration)} aria-valuenow={Math.round(shown)}>
        <div className="m-deck-progress__fill" style={{ width: `${percent}%` }} />
        <div className="m-deck-progress__notch" style={{ left: `${percent}%` }} />
      </div>
    </div>
  );
}

async function sharePlaying(player: MobilePlayer, notify: MobileDeckExtras["notify"]) {
  const track = player.currentTrack;
  if (!track) return;
  const text = [track.title, track.artist_name].filter(Boolean).join(" — ");
  try {
    if ((await shareText(track.title, text)) === "copied") notify("success", "Copied", `“${text}” is on the clipboard.`);
  } catch (error) {
    if ((error as DOMException)?.name !== "AbortError") notify("error", "Couldn’t share", String(error));
  }
}

/** Like, add to playlist, download, not interested, why this track, share (Figma E1 action strip). */
export const DeckActions = memo(function DeckActions({ player, deck }: { player: MobilePlayer; deck: MobileDeckExtras }) {
  const track = player.currentTrack;
  const liked = isPlayingLiked(player);
  const kind = isOnlineTrack(track);
  const kazeDownload = useSongDownloaded(track?.id ?? "") && kind === false;
  const canDownload = !!track && (kind !== false || kazeDownload);

  return (
    <div className="m-deck__actions-row">
      <button type="button" className={`m-deck__action-icon ${liked ? "is-liked" : ""}`} disabled={!track}
        onClick={() => togglePlayingLike(player)} aria-label={liked ? "Remove like" : "Like"} aria-pressed={liked}>
        <Heart size={20} strokeWidth={2} fill={liked ? "currentColor" : "none"} />
      </button>
      <button type="button" className={`m-deck__action-icon ${player.isInPlaylist ? "is-liked" : ""}`} disabled={!track}
        onClick={() => track && player.onOpenAddToPlaylist?.({
          id: track.id, title: track.title, artist: track.artist_name, album: track.album_title, cover_art_url: track.cover_art_url,
        })} aria-label="Add to playlist">
        <BookmarkPlus size={20} strokeWidth={2} />
      </button>
      {canDownload ? (
        <span className="m-deck__action-icon m-deck__download">
          <DownloadButton key={`download:${track.id}`} trackId={track.id} sourceId={track.playback_source?.track_id}
            title={track.title} pending={kind === undefined} compact />
        </span>
      ) : (
        <button type="button" className="m-deck__action-icon is-muted" disabled aria-label="Already on this phone">
          <Download size={20} strokeWidth={2} />
        </button>
      )}
      {track && (
        <button type="button" className={`m-deck__action-icon ${track.manual_like === -1 ? "is-red" : ""}`} aria-label="Not interested"
          onClick={() => openTrackSheet("not_interested", { id: track.id, title: track.title, artist: track.artist_name, album: track.album_title, cover: player.coverArtUrl })}>
          <ThumbsDown size={20} strokeWidth={2} />
        </button>
      )}
      <div className="m-deck__actions-spacer" />
      <button type="button" className="m-deck__action-icon" disabled={!track}
        onClick={() => void sharePlaying(player, deck.notify)} aria-label="Share">
        <Share2 size={20} strokeWidth={2} />
      </button>
    </div>
  );
});

/** Mute and volume (0–1), the same backend volume the desktop slider sets. */
export const DeckVolume = memo(function DeckVolume({ player }: { player: MobilePlayer }) {
  const { volume, is_muted: muted } = player.playbackState;
  const level = muted ? 0 : volume;
  return (
    <div className="m-deck__volume">
      <button type="button" className="m-deck__volume-btn" onClick={player.onToggleMute} aria-label={muted ? "Unmute" : "Mute"}>
        {muted ? <VolumeX size={18} strokeWidth={2} /> : <Volume1 size={18} strokeWidth={2} />}
      </button>
      <input className="m-deck__volume-range" type="range" min={0} max={1} step={0.01} value={level} aria-label="Volume"
        style={{ "--m-volume": `${level * 100}%` } as React.CSSProperties}
        onChange={(e) => player.onVolumeChange(Number(e.target.value))} />
      <Volume2 size={18} strokeWidth={2} className="m-deck__volume-icon" />
    </div>
  );
});

/** Loop / Queue / Lyrics / Flow (Figma E1 bottom tabs). Queue toggles the queue inside The Deck. */
export const DeckTabs = memo(function DeckTabs({ player, deck, queueOpen, onToggleQueue }: {
  player: MobilePlayer; deck: MobileDeckExtras; queueOpen: boolean; onToggleQueue: () => void;
}) {
  const nav = useMobileNav();
  const looping = player.playbackState.repeat_mode === "one";
  return (
    <div className="m-deck__tabs">
      <button type="button" className={`m-deck__tab-item ${looping ? "is-active" : ""}`} onClick={player.onToggleRepeat}
        aria-label="Loop current song" aria-pressed={looping}>
        <Repeat1 size={18} strokeWidth={2} />
        <span>LOOP</span>
      </button>
      <div className="m-deck__tab-divider" />
      <button type="button" className={`m-deck__tab-item ${queueOpen ? "is-active" : ""}`} onClick={onToggleQueue}
        aria-label={queueOpen ? "Back to now playing" : "Show the queue"} aria-pressed={queueOpen}>
        <ListMusic size={18} strokeWidth={2} />
        <span>QUEUE</span>
      </button>
      <div className="m-deck__tab-divider" />
      <button type="button" className="m-deck__tab-item" onClick={() => nav.openPage("lyrics")} aria-label="Lyrics">
        <Mic size={18} strokeWidth={2} />
        <span>LYRICS</span>
      </button>
      <div className="m-deck__tab-divider" />
      <button type="button" className={`m-deck__tab-item ${deck.flowActive ? "is-active" : ""}`} onClick={deck.onToggleFlow}
        aria-label={deck.flowActive ? "Pause Flow" : "Start Flow"} aria-pressed={deck.flowActive}>
        <Radio size={18} strokeWidth={2} />
        <span>FLOW</span>
      </button>
    </div>
  );
});
