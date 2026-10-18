import { memo, useEffect, useRef, useState } from "react";
import {
  ArrowDown,
  ArrowUp,
  ArrowUpRight,
  Disc3,
  Play,
  Search,
  X,
} from "lucide-react";
import { List, RowComponentProps } from "react-window";
import { Track, formatTime } from "./model";

export const Cover = memo(function Cover({
  track,
  eager = false,
}: {
  track: { artwork: string; album?: string; title?: string };
  eager?: boolean;
}) {
  const [failed, setFailed] = useState(false);
  return failed ? (
    <div
      className="bk-cover-fallback"
      role="img"
      aria-label="Artwork unavailable"
    >
      <Disc3 size={48} />
    </div>
  ) : (
    <img
      src={track.artwork}
      alt={track.album ?? track.title ?? "Album artwork"}
      width={600}
      height={600}
      loading={eager ? "eager" : "lazy"}
      decoding="async"
      draggable={false}
      onError={() => setFailed(true)}
    />
  );
});

export const SearchBox = memo(function SearchBox({
  onSearch,
}: {
  onSearch: (query: string) => void;
}) {
  const [value, setValue] = useState("");
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => {
    const timeout = setTimeout(() => onSearch(value.trim()), 200);
    return () => clearTimeout(timeout);
  }, [value, onSearch]);
  useEffect(() => {
    const shortcut = (event: KeyboardEvent) => {
      if (
        event.key !== "/" ||
        event.ctrlKey ||
        event.metaKey ||
        event.altKey ||
        event.target instanceof HTMLInputElement ||
        event.target instanceof HTMLTextAreaElement ||
        (event.target instanceof HTMLElement && event.target.isContentEditable)
      )
        return;
      event.preventDefault();
      input.current?.focus();
    };
    window.addEventListener("keydown", shortcut);
    return () => window.removeEventListener("keydown", shortcut);
  }, []);
  return (
    <label className="bk-search">
      <Search size={17} />
      <input
        ref={input}
        type="search"
        value={value}
        onChange={(event) => setValue(event.target.value)}
        placeholder="Find a track / artist"
        aria-label="Search sample music"
      />
      <kbd>/</kbd>
    </label>
  );
});

export const Ticket = memo(function Ticket({
  track,
  index,
  onPlay,
}: {
  track: Track;
  index: number;
  onPlay: (id: string) => void;
}) {
  return (
    <button
      className="bk-ticket"
      onClick={() => onPlay(track.id)}
      aria-label={`Play ${track.title} by ${track.artist}`}
    >
      <span className="bk-ticket-number">
        {String(index + 1).padStart(2, "0")}
      </span>
      <Cover track={track} />
      <span>
        <strong>{track.title}</strong>
        <small>{track.artist}</small>
      </span>
      <ArrowUpRight size={19} />
    </button>
  );
});

interface TrackRowsProps {
  tracks: Track[];
  currentId: string;
  onPlay: (id: string) => void;
  onMove?: (id: string, offset: number) => void;
  onRemove?: (id: string) => void;
}
function TrackRow({
  index,
  style,
  ariaAttributes,
  tracks,
  currentId,
  onPlay,
  onMove,
  onRemove,
}: RowComponentProps<TrackRowsProps>) {
  const track = tracks[index];
  return (
    <div
      {...ariaAttributes}
      style={style}
      className={`bk-track-row ${currentId === track.id ? "bk-track-current" : ""}`}
    >
      <span className="bk-row-number">
        {String(index + 1).padStart(2, "0")}
      </span>
      <button
        className="bk-row-song"
        onClick={() => onPlay(track.id)}
        aria-label={`Play ${track.title} by ${track.artist}`}
      >
        <Cover key={track.artwork} track={track} />
        <span>
          <strong>{track.title}</strong>
          <small>{track.artist}</small>
        </span>
      </button>
      {onMove ? (
        <div className="bk-order">
          <button
            className="bk-icon"
            disabled={index === 0}
            aria-label={`Move ${track.title} earlier`}
            onClick={() => onMove(track.id, -1)}
          >
            <ArrowUp size={17} />
          </button>
          <button
            className="bk-icon"
            disabled={index === tracks.length - 1}
            aria-label={`Move ${track.title} later`}
            onClick={() => onMove(track.id, 1)}
          >
            <ArrowDown size={17} />
          </button>
        </div>
      ) : (
        <span className="bk-row-album">{track.album}</span>
      )}
      {onRemove ? (
        <button
          className="bk-icon"
          aria-label={`Remove ${track.title} from queue`}
          onClick={() => onRemove(track.id)}
        >
          <X size={17} />
        </button>
      ) : (
        <span className="bk-row-duration">{formatTime(track.duration)}</span>
      )}
    </div>
  );
}
export const TrackRows = memo(function TrackRows(props: TrackRowsProps) {
  return (
    <div className="bk-track-list" aria-label="Track list">
      <List
        rowComponent={TrackRow}
        rowCount={props.tracks.length}
        rowHeight={88}
        rowProps={props}
        overscanCount={3}
        style={{ height: "100%", minHeight: 160 }}
      />
    </div>
  );
});

export const Empty = memo(function Empty({
  title,
  text,
  onDiscover,
}: {
  title: string;
  text: string;
  onDiscover?: () => void;
}) {
  return (
    <div className="bk-empty">
      <Disc3 size={48} />
      <h2>{title}</h2>
      <p>{text}</p>
      {onDiscover && (
        <button className="bk-text-button" onClick={onDiscover}>
          Find some music <ArrowUpRight size={21} />
        </button>
      )}
    </div>
  );
});

export const SleeveButton = memo(function SleeveButton({
  track,
  index,
  onPlay,
  onAlbum,
}: {
  track: Track;
  index: number;
  onPlay: (id: string) => void;
  onAlbum?: (id: string) => void;
}) {
  return (
    <article className="bk-sleeve">
      <span className="bk-sleeve-index">
        {String(index + 1).padStart(2, "0")} / {track.genre}
      </span>
      <button
        className="bk-sleeve-art"
        onClick={() => onPlay(track.id)}
        aria-label={`Play ${track.title} by ${track.artist}`}
      >
        <Cover track={track} />
        <span className="bk-sleeve-play" aria-hidden="true">
          <Play fill="currentColor" size={26} />
        </span>
      </button>
      <button
        className="bk-sleeve-title"
        onClick={() => (onAlbum ? onAlbum(track.albumId) : onPlay(track.id))}
      >
        {track.title}
        <ArrowUpRight size={20} />
      </button>
      <p>{track.artist}</p>
    </article>
  );
});
