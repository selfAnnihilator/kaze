import { memo, type CSSProperties } from "react";
import { ArrowRight, Play, Trash2 } from "lucide-react";
import type { Track } from "../types";
import { useInterfaceStyle } from "./InterfaceContext";
import { Artwork } from "./Artwork";
import { WindowedList } from "./WindowedList";
import type { QueueEntry } from "./queuePresentation";

export const QueueView = memo(function QueueView({
  entries,
  tracks,
  currentTrack,
  onPlayIndex,
  onRemove,
  onClear,
}: {
  entries: QueueEntry[];
  tracks: Track[];
  currentTrack?: Track;
  onPlayIndex: (index: number) => void;
  onRemove: (id: string) => void;
  onClear: () => void;
}) {
  const style = useInterfaceStyle();
  return (
    <section className="queue-view">
      {style === "brutalist" ? (
        <header className="press-view-heading">
          <div>
            <span className="press-micro">THE QUEUE / IN YOUR ORDER</span>
            <h1 aria-label="Queue">
              WHAT
              <br />
              COMES NEXT.
            </h1>
          </div>
          <span className="press-big-index">
            {String(entries.length).padStart(2, "0")}
          </span>
        </header>
      ) : (
        <header className="view-header">
          <div>
            <p className="interface-eyebrow">
              {style === "spatial"
                ? "Keep a good thing going"
                : "The music ahead"}
            </p>
            <h1 className="view-title">
              <span aria-label="Queue">
                {style === "spatial" ? "What’s next." : "Queue"}
              </span>
              <span className="queue-count">
                {String(entries.length).padStart(2, "0")}
              </span>
            </h1>
            {style === "spatial" && (
              <p className="queue-intro">
                Your queue, with a little room to move.
              </p>
            )}
          </div>
          {style !== "spatial" && (
            <button
              className="btn btn-secondary"
              onClick={onClear}
              disabled={!entries.length}
            >
              Clear queue
            </button>
          )}
        </header>
      )}
      {style === "spatial" && (
        <div className="queue-path">
          {currentTrack && (
            <div className="queue-current">
              {style === "spatial" && (
                <span className="interface-eyebrow">Now Playing</span>
              )}
              <Artwork
                src={currentTrack.cover_art_url}
                trackId={currentTrack.id}
              />
              <div>
                {style !== "spatial" && (
                  <span className="interface-eyebrow">Now</span>
                )}
                <h2>{currentTrack.title}</h2>
                <p>{currentTrack.artist_name}</p>
              </div>
            </div>
          )}
          {style === "spatial" && entries.length > 0 && (
            <>
              <ArrowRight className="spatial-queue-arrow" size={30} />
              <div className="spatial-queue-future">
                {entries.slice(0, 3).map((entry, index) => {
                  const track = tracks.find(
                    (item) => item.id === entry.trackId,
                  );
                  return (
                    <button
                      key={entry.key}
                      onClick={() => onPlayIndex(entry.index)}
                      style={{ "--queue-order": index } as CSSProperties}
                      aria-label={`Play ${track?.title || entry.title || "queued track"} next`}
                    >
                      <Artwork
                        src={track?.cover_art_url}
                        trackId={entry.trackId}
                      />
                      <span>
                        {String(index + 1).padStart(2, "0")}
                        <strong>
                          {track?.title || entry.title || "Queued track"}
                        </strong>
                        <small>
                          {track?.artist_name || entry.artist || ""}
                        </small>
                      </span>
                    </button>
                  );
                })}
              </div>
            </>
          )}
        </div>
      )}
      <div className="queue-next-heading">
        <h2 className="queue-next-label">
          Up next
          {style === "spatial" && (
            <span className="queue-next-count">{entries.length}</span>
          )}
        </h2>
        {
          <button
            className="spatial-text-action"
            disabled={!entries.length}
            onClick={onClear}
          >
            Clear queue
          </button>
        }
      </div>
      {!entries.length ? (
        <p className="interface-empty">
          Your queue is open. Add tracks from your library, a playlist, or
          Discovery.
        </p>
      ) : (
        <WindowedList
          items={entries}
          rowHeight={88}
          label="Upcoming queue"
          renderItem={(entry, offset) => {
            const track = tracks.find((t) => t.id === entry.trackId);
            const title = track?.title || entry.title || "Queued track";
            return (
              <div className="queue-entry" key={entry.key}>
                <span className="queue-number">
                  {String(offset + 1).padStart(2, "0")}
                </span>
                <Artwork src={track?.cover_art_url} trackId={entry.trackId} />
                <button
                  className="queue-track"
                  onClick={() => onPlayIndex(entry.index)}
                  title={`Play ${title}`}
                >
                  <strong>{title}</strong>
                  <span>
                    {track?.artist_name ||
                      entry.artist ||
                      "Metadata available when playback starts"}
                  </span>
                </button>
                <button
                  className="player-icon-btn"
                  aria-label={`Play ${title}`}
                  onClick={() => onPlayIndex(entry.index)}
                >
                  <Play size={18} />
                </button>
                <button
                  className="player-icon-btn"
                  aria-label={`Remove ${title} from queue`}
                  onClick={() => onRemove(entry.trackId)}
                >
                  <Trash2 size={16} />
                </button>
              </div>
            );
          }}
        />
      )}
    </section>
  );
});
