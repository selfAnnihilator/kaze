import { memo } from "react";
import { ArrowUpRight } from "lucide-react";
import type { Track } from "../types";
import type { QueueEntry } from "./queuePresentation";
import { Artwork } from "./Artwork";
import { OverflowTitle } from "./OverflowTitle";

export const BrutalistPoster = memo(function BrutalistPoster(p: {
  track?: Track;
  albumId?: string;
  cover?: string | null;
  index?: number | null;
  playing: boolean;
  loading?: boolean;
  entries: QueueEntry[];
  tracks: Track[];
  onPlayIndex: (index: number) => void;
  onOpenAlbum: (id: string) => void;
}) {
  const album = p.albumId;
  return (
    <div className="press-poster-page">
      <section className="press-listen" aria-label="Now playing">
        <div className="press-poster-title">
          <span className="press-micro">
            {p.loading
              ? "LOADING / BUFFERING"
              : p.playing
                ? "NOW PLAYING"
                : "PAUSED"}
          </span>
          <h1><OverflowTitle text={p.track?.title || "YOUR NEXT RECORD"} focusable /></h1>
        </div>
        <div className="press-poster-body">
          <button
            className="press-poster-cover"
            disabled={!album}
            onClick={() => album && p.onOpenAlbum(album)}
            aria-label={`Open ${p.track?.album_title || "record"}`}
          >
            <Artwork
              src={p.cover || p.track?.cover_art_url}
              trackId={p.track?.id}
              alt={p.track?.album_title || p.track?.title}
            />
            <span className="press-cover-open">
              OPEN RECORD
              <ArrowUpRight size={20} />
            </span>
          </button>
          <div className="press-poster-annotation">
            <div className="press-poster-count">
              {p.track ? String((p.index ?? 0) + 1).padStart(2, "0") : "00"}
            </div>
            <span className="press-micro">{p.entries.length} TRACKS AHEAD</span>
            <h2>{p.track?.artist_name || "Make room for music."}</h2>
            <button
              className="press-text-button"
              disabled={!album}
              onClick={() => album && p.onOpenAlbum(album)}
            >
              {p.track?.album_title || "Choose a track, or start your Flow."}
              {album && <ArrowUpRight size={23} />}
            </button>
          </div>
        </div>
        <div className="press-poster-future">
          <div className="press-line-label">
            <span>NEXT ON DECK</span>
            <span>PICK YOUR NEXT MOVE</span>
          </div>
          <div className="press-ticket-row">
            {p.entries.slice(0, 3).map((entry, i) => {
              const t = p.tracks.find((t) => t.id === entry.trackId);
              return (
                <button
                  className="press-ticket"
                  key={entry.key}
                  onClick={() => p.onPlayIndex(entry.index)}
                >
                  <span className="press-ticket-number">
                    {String(i + 1).padStart(2, "0")}
                  </span>
                  <Artwork src={t?.cover_art_url} trackId={entry.trackId} />
                  <span>
                    <strong>{t?.title || entry.title || "Queued track"}</strong>
                    <small>{t?.artist_name || entry.artist}</small>
                  </span>
                  <ArrowUpRight size={18} />
                </button>
              );
            })}
            {!p.entries.length && (
              <p className="press-side-end">
                End of the side. Find your next record in Discover.
              </p>
            )}
          </div>
        </div>
      </section>
    </div>
  );
});
