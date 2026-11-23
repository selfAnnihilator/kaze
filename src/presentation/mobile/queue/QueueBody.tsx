import { useMemo } from "react";
import { Trash2, X } from "lucide-react";
import { WindowedList } from "../../WindowedList";
import { TrackRow } from "../TrackRow";
import { PlaybackTime } from "../PlaybackTime";
import type { QueueEntry } from "../../queuePresentation";
import type { MobileScreenProps } from "../types";

const ROW_HEIGHT = 67;

/**
 * The queue itself (Figma E2): heading with Clear, the playing song, then the
 * windowed upcoming list. Shown by the Queue tab and inside The Deck.
 */
export function QueueBody({ c, kicker }: { c: MobileScreenProps["c"]; kicker: string }) {
  const current = c.playbackState.current_track;
  const queue = c.queueEntries;
  // Same lookup as the desktop poster: playback events carry ids, the library carries names.
  const byId = useMemo(() => new Map(c.tracks.map((t) => [t.id, t])), [c.tracks]);

  return (
    <>
    <div className="m-queue-header">
      <div className="m-queue-header__text">
        <span className="m-kicker">{kicker}</span>
        <h1 className="m-display m-queue-header__title">UP NEXT.</h1>
      </div>
      {queue.length > 0 && (
        <button type="button" className="m-btn m-btn--outline m-queue-header__clear" onClick={c.handleClearQueue}>
          <Trash2 size={18} strokeWidth={2} />
          <span>CLEAR</span>
        </button>
      )}
    </div>

    <div className="m-queue-label"><span className="m-kicker m-kicker--ink">NOW PLAYING</span></div>
    {current ? (
      <TrackRow
        active
        title={current.title}
        subtitle={[current.artist_name, current.album_title].filter(Boolean).join(" · ")}
        src={c.activePlayingArtwork}
        trackId={current.id}
        onPress={() => void c.handleUnifiedPlayPause()}
        trailing={<PlaybackTime />}
      />
    ) : (
      <p className="m-empty-msg">Select a song to play.</p>
    )}

    <div className="m-queue-label m-queue-label--upcoming">
      <span className="m-kicker m-kicker--ink">UPCOMING</span>
      <span className="m-kicker m-kicker--muted">{queue.length} TRACKS</span>
    </div>
    <div className="m-queue-list">
      {queue.length > 0 ? (
        <WindowedList
          items={queue}
          rowHeight={ROW_HEIGHT}
          label="Queue"
          fill
          renderItem={(entry: QueueEntry, idx) => (
            <TrackRow
              key={entry.key}
              index={idx + 1}
              title={entry.title || byId.get(entry.trackId)?.title || "Queued track"}
              subtitle={entry.artist || byId.get(entry.trackId)?.artist_name}
              src={byId.get(entry.trackId)?.cover_art_url}
              trackId={entry.trackId}
              onPress={() => c.handlePlayQueueIndex(entry.index)}
              trailing={
                <button type="button" className="m-track-row__action-btn" onClick={() => c.handleDequeueTrack(entry.trackId)}
                  aria-label={`Remove ${entry.title ?? "track"}`}>
                  <X size={18} strokeWidth={2} />
                </button>
              }
            />
          )}
        />
      ) : (
        <p className="m-empty-msg">Queue is empty. Add songs from Library or Discover.</p>
      )}
    </div>
    </>
  );
}
