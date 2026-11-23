import { memo, useCallback, useState, type Ref } from "react";
import { ArrowUpRight, ChevronDown, Pause, Play, SkipBack, SkipForward } from "lucide-react";
import { DeckActions, DeckProgressBar, DeckTabs, DeckVolume } from "./DeckParts";
import { Sleeve } from "./Sleeve";
import { QueueBody } from "./queue/QueueBody";
import { useMobileNav } from "./mobileNav";
import { usePresence } from "./usePresence";
import { useSleeveSlide } from "./sleeveSlide";
import type { AppViewsContext } from "../../components/layout/appViews/context";
import type { MobileDeckExtras, MobilePlayer } from "./types";

/** Open Album when the song's album is in the library, else the playlist or mix it plays from. */
function SourceLink({ c, player, deck, onClose }: { c: AppViewsContext; player: MobilePlayer; deck: MobileDeckExtras; onClose: () => void }) {
  const nav = useMobileNav();
  const track = player.currentTrack;
  const album = track?.album_title
    ? c.albums.find((a) => a.title === track.album_title && a.artist_name === track.artist_name)
    : undefined;
  if (album) {
    return (
      <button type="button" className="m-deck__album-link" onClick={() => nav.openAlbum(album.id)} aria-label={`Open album ${album.title}`}>
        <span>OPEN ALBUM</span>
        <ArrowUpRight size={12} strokeWidth={2} />
      </button>
    );
  }
  if (!deck.onOpenOrigin) return null;
  return (
    <button type="button" className="m-deck__album-link" onClick={() => { onClose(); deck.onOpenOrigin?.(); }} aria-label="Open where this song plays from">
      <span>OPEN SOURCE</span>
      <ArrowUpRight size={12} strokeWidth={2} />
    </button>
  );
}

/**
 * The Deck (Figma E1 Now Playing): previous / playing / next sleeves, title,
 * artist with Open Album, the action strip, position, transport, volume and
 * the Loop / Queue / Lyrics / Flow tabs.
 */
export const MobileDeck = memo(function MobileDeck({
  c, player, deck, onClose, sheetRef, headerRef,
}: {
  c: AppViewsContext; player: MobilePlayer; deck: MobileDeckExtras; onClose: () => void;
  /** The sheet that grows out of the mini player; its position is set by `useDeckSheet`. */
  sheetRef?: Ref<HTMLElement>;
  /** The header strip, where dragging down closes The Deck. */
  headerRef?: Ref<HTMLElement>;
}) {
  const { currentTrack: track, playbackState: state } = player;
  const isPlaying = state.is_playing;
  const previous = c.previousTrack;
  const next = c.nextQueuedTrack;
  const [queueOpen, setQueueOpen] = useState(false);
  const toggleQueue = useCallback(() => setQueueOpen((open) => !open), []);
  const queue = usePresence(queueOpen);
  const slide = useSleeveSlide(track?.id, previous?.id, next?.id);
  const meta = track ? [track.artist_name || "Unknown artist", track.album_title].filter(Boolean).join(" — ") : "Pick a song or start your Flow";

  return (
    <section ref={sheetRef} className="m-deck" aria-label="The Deck — Now Playing">
      <header ref={headerRef} className="m-deck__header">
        <button type="button" className="m-deck__btn-header" onClick={onClose} aria-label="Close The Deck">
          <ChevronDown size={20} strokeWidth={2} />
        </button>
        <span className="m-deck__header-title">THE DECK // NOW PLAYING</span>
        {/* Keeps the title centred opposite the close button. */}
        <span className="m-deck__header-spacer" aria-hidden="true" />
      </header>

      <div className="m-deck__divider" />

      <div className="m-deck__stage">
      {!queueOpen && (
        <div className="m-deck__scroll-content">
          <div className="m-deck__turntable">
            <button type="button" className="m-deck__turntable-side" onClick={player.onPrevious} disabled={!previous} aria-label="Previous track">
              <Sleeve key={previous?.id} size="side" src={previous?.cover_art_url} trackId={previous?.id} title={previous?.title} />
            </button>
            <div className="m-deck__turntable-main">
              <div key={track?.id} className={slide}>
                <Sleeve size="deck" src={player.coverArtUrl} trackId={track?.id} title={track?.title} />
              </div>
            </div>
            <button type="button" className="m-deck__turntable-side" onClick={player.onNext} disabled={!next} aria-label="Next track">
              <Sleeve key={next?.id} size="side" src={next?.cover_art_url} trackId={next?.id} title={next?.title} />
            </button>
          </div>

          <div className="m-deck__track-info">
            <h1 className="m-display m-deck__track-title">{track?.title?.toUpperCase() ?? "NOTHING PLAYING"}</h1>
            <div className="m-deck__meta-row">
              <span className="m-deck__artist-name">{meta}</span>
              <SourceLink c={c} player={player} deck={deck} onClose={onClose} />
            </div>
          </div>

          <DeckActions player={player} deck={deck} />
          <DeckProgressBar duration={state.duration_secs || track?.duration_secs || 0} onSeek={player.onSeek} />

          <div className="m-deck__transport">
            <button type="button" className="m-deck__btn-transport" onClick={player.onPrevious} aria-label="Previous track">
              <SkipBack size={22} strokeWidth={2} />
              <span className="m-deck__btn-transport-label">PREV</span>
            </button>
            <button type="button" className="m-deck__btn-play-hero" disabled={!!player.isLoading || !track}
              onClick={player.onPlayPause} aria-label={isPlaying ? "Pause" : "Play"}>
              {isPlaying ? <Pause size={30} strokeWidth={2} fill="currentColor" /> : <Play size={30} strokeWidth={2} fill="currentColor" />}
              <span className="m-deck__play-hero-text">{isPlaying ? "PAUSE" : "PLAY"}</span>
            </button>
            <button type="button" className="m-deck__btn-transport" onClick={player.onNext} aria-label="Next track">
              <SkipForward size={22} strokeWidth={2} />
              <span className="m-deck__btn-transport-label">NEXT</span>
            </button>
          </div>

          <DeckVolume player={player} />
        </div>
      )}
      {queue.mounted && (
        <div className={`m-deck__queue m-queue ${queue.leaving ? "is-leaving" : ""}`}>
          <QueueBody c={c} kicker="THE DECK // QUEUE" />
        </div>
      )}
      </div>

      <div className="m-deck__divider" />
      <DeckTabs player={player} deck={deck} queueOpen={queueOpen} onToggleQueue={toggleQueue} />
    </section>
  );
});
