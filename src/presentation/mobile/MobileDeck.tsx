import { memo } from "react";
import { ChevronDown, Heart } from "lucide-react";
import { NowPlayingProgressBar } from "../../components/NowPlayingProgressBar";
import { MobileIconButton, EmptyNote } from "./primitives";
import { Sleeve } from "./Sleeve";
import { TransportControls } from "./TransportControls";
import { joinMeta } from "./format";
import type { MobilePlayer } from "./types";

/** The Deck: full-screen now playing with cover, seek bar and transport. */
export const MobileDeck = memo(function MobileDeck({
  player, onClose,
}: { player: MobilePlayer; onClose: () => void }) {
  const { currentTrack: track, playbackState: state } = player;
  const liked = (track?.manual_like ?? 0) === 1;
  return (
    <section className="m-deck" aria-label="The Deck">
      <header className="m-deck__bar">
        <MobileIconButton label="Close the deck" onClick={onClose}><ChevronDown size={24} /></MobileIconButton>
        <span className="m-deck__kicker">The Deck</span>
      </header>
      {!track ? <EmptyNote>Nothing is playing.</EmptyNote> : (
        <>
          <Sleeve size="large" title={track.title} src={player.coverArtUrl} trackId={track.id} />
          <div className="m-deck__info">
            <h1 className="m-deck__title">{track.title}</h1>
            <p className="m-deck__meta">{joinMeta(track.artist_name, track.album_title)}</p>
          </div>
          <NowPlayingProgressBar duration={state.duration_secs} canSeek={state.can_seek !== false}
            onSeek={player.onSeek} />
          <TransportControls isPlaying={state.is_playing} isLoading={!!player.isLoading} repeat={state.repeat_mode}
            onPlayPause={player.onPlayPause} onNext={player.onNext} onPrevious={player.onPrevious}
            onToggleRepeat={player.onToggleRepeat} />
          <div className="m-deck__actions">
            <MobileIconButton label={liked ? "Remove like" : "Like"} active={liked}
              onClick={() => (liked ? player.onRemoveFeedback(track.id) : player.onLike(track.id))}>
              <Heart size={20} />
            </MobileIconButton>
          </div>
        </>
      )}
    </section>
  );
});
