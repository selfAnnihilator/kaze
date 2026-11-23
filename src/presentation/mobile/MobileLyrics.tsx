import { Heart, Pause, Play, SkipBack, SkipForward, X } from "lucide-react";
import { WindowedList } from "../WindowedList";
import { DeckProgressBar } from "./DeckParts";
import { LyricLines } from "./lyrics/LyricLines";
import { useLyrics } from "./lyrics/useLyrics";
import { isPlayingLiked, togglePlayingLike } from "./likeTrack";
import { Sleeve } from "./Sleeve";
import type { MobilePlayer } from "./types";

function LyricsBody({ lyrics, onSeek }: { lyrics: ReturnType<typeof useLyrics>; onSeek: (s: number) => void }) {
  if (lyrics.synced.length) return <LyricLines lines={lyrics.synced} onSeek={onSeek} />;
  if (lyrics.plain.length) {
    return (
      <div className="m-lyrics__plain">
        <WindowedList items={lyrics.plain} rowHeight={45} label="Lyrics" fill
          renderItem={(line: string, i: number) => <p key={i} className="m-lyric m-lyric--plain">{line}</p>} />
      </div>
    );
  }
  let text = "No lyrics for this song yet. Lyrics appear here when they’re available.";
  if (lyrics.loading) text = "Finding lyrics…";
  else if (lyrics.failed) text = "Couldn’t load lyrics. Check your connection and open Lyrics again.";
  else if (lyrics.instrumental) text = "This song is instrumental. Nothing to sing along to.";
  return <p className="m-empty-msg m-lyrics__empty">{text}</p>;
}

/** Lyrics (Figma E3): full screen over the app with the song, its lines, position and transport. */
export function MobileLyrics({ player, onClose, leaving = false }: { player: MobilePlayer; onClose: () => void; leaving?: boolean }) {
  const track = player.currentTrack;
  const lyrics = useLyrics(track);
  const state = player.playbackState;
  const synced = lyrics.synced.length > 0;
  return (
    <section className={`m-sheet m-lyrics ${leaving ? "is-leaving" : ""}`} role="dialog" aria-modal="true" aria-label="Lyrics">
      <div className="m-lyrics__bar">
        <span className="m-kicker m-kicker--ink">{synced ? "LYRICS // TAP A LINE TO SEEK" : "LYRICS"}</span>
        <button type="button" className="m-backbar__btn" onClick={onClose} aria-label="Close lyrics"><X size={20} strokeWidth={2} /></button>
      </div>
      <div className="m-lyrics__song">
        <Sleeve size="mini" src={player.coverArtUrl} trackId={track?.id} title={track?.title} />
        <span className="m-set-row__text">
          <strong>{track?.title || "Nothing playing"}</strong>
          <span className="m-set-row__mono">{[track?.artist_name?.toUpperCase(), synced ? "SYNCED LYRICS" : null].filter(Boolean).join(" · ")}</span>
        </span>
        <button type="button" className="m-backbar__btn" disabled={!track} onClick={() => togglePlayingLike(player)} aria-label="Like" aria-pressed={isPlayingLiked(player)}>
          <Heart size={20} strokeWidth={2} fill={isPlayingLiked(player) ? "currentColor" : "none"} />
        </button>
      </div>
      <div className="m-lyrics__body"><LyricsBody lyrics={lyrics} onSeek={player.onSeek} /></div>
      <DeckProgressBar duration={state.duration_secs || track?.duration_secs || 0} onSeek={player.onSeek} />
      <div className="m-lyrics__controls">
        <button type="button" className="m-lyrics__skip" onClick={player.onPrevious} aria-label="Previous"><SkipBack size={20} strokeWidth={2} /></button>
        <button type="button" className="m-lyrics__play" onClick={player.onPlayPause}>
          {state.is_playing ? <Pause size={22} strokeWidth={2} /> : <Play size={22} strokeWidth={2} />}
          <span>{state.is_playing ? "PAUSE" : "PLAY"}</span>
        </button>
        <button type="button" className="m-lyrics__skip" onClick={player.onNext} aria-label="Next"><SkipForward size={20} strokeWidth={2} /></button>
      </div>
    </section>
  );
}
