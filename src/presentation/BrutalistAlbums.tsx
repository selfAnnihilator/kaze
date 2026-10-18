import { memo } from "react";
import { ArrowLeft, ArrowRight } from "lucide-react";
import { useRef } from "react";
import type { Album } from "../types";
import { Artwork } from "./Artwork";
import { WindowedStrip } from "./WindowedStrip";
import { useViewMemory } from "./ViewMemory";

export const BrutalistAlbums = memo(function BrutalistAlbums({
  albums,
  onSelectAlbum,
}: {
  albums: Album[];
  onSelectAlbum: (id: string) => void;
}) {
  const wall = useRef<HTMLDivElement>(null);
  const position = useViewMemory("album-spines", () => ({ left: 0, top: 0 }));
  return (
    <section className="press-albums albums-view">
      {albums.length ? (
        <WindowedStrip
          items={albums}
          scrollRef={wall}
          label="Album spines"
          initialScrollLeft={position.left}
          initialScrollTop={position.top}
          onScroll={() => {
            if (wall.current) {
              position.left = wall.current.scrollLeft;
              position.top = wall.current.scrollTop;
            }
          }}
          renderItem={(a, i) => (
            <button
              className="press-record-spine"
              aria-label={`Open album ${a.title}`}
              title={`${a.title} · ${a.artist_name || "Unknown Artist"}`}
              onClick={() => onSelectAlbum(a.id)}
            >
              <span>{String(i + 1).padStart(2, "0")}</span>
              <Artwork
                src={a.cover_art_path}
                trackId={a.first_track_id}
                alt={a.title}
              />
              <strong>{a.title}</strong>
              <small>{a.artist_name || "Unknown Artist"}</small>
            </button>
          )}
        />
      ) : (
        <p className="interface-empty">
          Your record shelf is waiting. Add a music folder in Settings and
          rescan your library.
        </p>
      )}
      <div className="press-archive-footer">
        <span>{albums.length} RECORDS / SELECT A SPINE TO OPEN</span>
        <div>
          <button
            className="press-icon"
            aria-label="Earlier records"
            onClick={() =>
              wall.current?.scrollBy({ left: -400, behavior: "smooth" })
            }
          >
            <ArrowLeft size={20} />
          </button>
          <button
            className="press-icon"
            aria-label="Later records"
            onClick={() =>
              wall.current?.scrollBy({ left: 400, behavior: "smooth" })
            }
          >
            <ArrowRight size={20} />
          </button>
        </div>
      </div>
    </section>
  );
});
