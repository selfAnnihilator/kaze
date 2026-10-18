import { memo, useEffect, useRef, useState } from "react";
import {
  ArrowLeft,
  ArrowRight,
  Disc3,
  Heart,
  Play,
  Search,
  X,
} from "lucide-react";
import { SpatialTrack } from "./data";

export const Artwork = memo(function Artwork({
  src,
  title,
  eager = false,
  className = "",
}: {
  src: string;
  title: string;
  eager?: boolean;
  className?: string;
}) {
  const [failed, setFailed] = useState(false);
  return failed ? (
    <div
      className={`sp-art-fallback ${className}`}
      role="img"
      aria-label={`${title}, artwork unavailable`}
    >
      <Disc3 size={48} />
      <span>{title}</span>
    </div>
  ) : (
    <img
      className={className}
      src={src}
      alt={title}
      width="600"
      height="600"
      loading={eager ? "eager" : "lazy"}
      decoding="async"
      draggable={false}
      onError={() => setFailed(true)}
    />
  );
});

export const MusicCard = memo(function MusicCard({
  track,
  onPlay,
  onAlbum,
  className = "",
  onPreview,
}: {
  track: SpatialTrack;
  onPlay: (id: string) => void;
  onAlbum: (id: string) => void;
  className?: string;
  onPreview?: (id: string) => void;
}) {
  return (
    <article
      className={`sp-music-card ${className}`}
      onPointerEnter={(event) => {
        if (event.pointerType === "mouse") onPreview?.(track.id);
      }}
      onFocusCapture={() => onPreview?.(track.id)}
    >
      <button
        className="sp-cover-button"
        aria-label={`Play ${track.title} by ${track.artist}`}
        onClick={() => onPlay(track.id)}
      >
        <Artwork
          src={track.artwork}
          title={`${track.album} by ${track.artist}`}
        />
        <span className="sp-cover-play" aria-hidden="true">
          <Play size={22} fill="currentColor" />
        </span>
      </button>
      <button className="sp-card-title" onClick={() => onAlbum(track.albumId)}>
        {track.title}
      </button>
      <span className="sp-card-artist">{track.artist}</span>
    </article>
  );
});

export const MusicStrip = memo(function MusicStrip({
  tracks,
  label,
  onPlay,
  onAlbum,
  onPreview,
}: {
  tracks: SpatialTrack[];
  label: string;
  onPlay: (id: string) => void;
  onAlbum: (id: string) => void;
  onPreview?: (id: string) => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const gesture = useRef({
    down: false,
    moved: false,
    x: 0,
    left: 0,
    last: 0,
    at: 0,
    velocity: 0,
    pointer: 0,
  });
  const frame = useRef(0);
  useEffect(() => () => cancelAnimationFrame(frame.current), []);
  const move = (direction: number) => {
    cancelAnimationFrame(frame.current);
    ref.current?.scrollBy({
      left: direction * (ref.current.clientWidth * 0.72),
      behavior: window.matchMedia("(prefers-reduced-motion: reduce)").matches
        ? "auto"
        : "smooth",
    });
  };
  return (
    <div className="sp-strip-wrap">
      <div className="sp-strip-actions">
        <span>{label}</span>
        <div>
          <button
            className="sp-icon"
            aria-label={`Previous ${label}`}
            onClick={() => move(-1)}
          >
            <ArrowLeft size={17} />
          </button>
          <button
            className="sp-icon"
            aria-label={`Next ${label}`}
            onClick={() => move(1)}
          >
            <ArrowRight size={17} />
          </button>
        </div>
      </div>
      <div
        className="sp-strip"
        ref={ref}
        tabIndex={0}
        aria-label={label}
        onKeyDown={(event) => {
          if (event.key === "ArrowRight" || event.key === "ArrowLeft") {
            event.preventDefault();
            move(event.key === "ArrowRight" ? 1 : -1);
          }
        }}
        onPointerDown={(event) => {
          if (
            event.pointerType !== "mouse" ||
            event.button !== 0 ||
            !ref.current
          )
            return;
          cancelAnimationFrame(frame.current);
          gesture.current = {
            down: true,
            moved: false,
            x: event.clientX,
            left: ref.current.scrollLeft,
            last: event.clientX,
            at: performance.now(),
            velocity: 0,
            pointer: event.pointerId,
          };
        }}
        onPointerMove={(event) => {
          const state = gesture.current,
            element = ref.current;
          if (!state.down || !element) return;
          const distance = event.clientX - state.x;
          if (Math.abs(distance) > 5) {
            state.moved = true;
            element.setPointerCapture(event.pointerId);
            element.dataset.dragging = "true";
          }
          if (!state.moved) return;
          const now = performance.now();
          state.velocity =
            (event.clientX - state.last) / Math.max(now - state.at, 1);
          state.last = event.clientX;
          state.at = now;
          element.scrollLeft = state.left - distance;
        }}
        onPointerUp={(event) => {
          const state = gesture.current,
            element = ref.current;
          if (!state.down || !element) return;
          state.down = false;
          element.dataset.dragging = "false";
          if (element.hasPointerCapture(event.pointerId))
            element.releasePointerCapture(event.pointerId);
          if (
            !state.moved ||
            window.matchMedia("(prefers-reduced-motion: reduce)").matches
          )
            return;
          let velocity =
            performance.now() - state.at < 80 ? state.velocity * 16 : 0;
          const coast = () => {
            if (!ref.current || Math.abs(velocity) < 0.5) return;
            ref.current.scrollLeft -= velocity;
            velocity *= 0.91;
            frame.current = requestAnimationFrame(coast);
          };
          frame.current = requestAnimationFrame(coast);
        }}
        onPointerCancel={() => {
          gesture.current.down = false;
          if (ref.current) ref.current.dataset.dragging = "false";
        }}
        onClickCapture={(event) => {
          if (gesture.current.moved) {
            event.preventDefault();
            event.stopPropagation();
            gesture.current.moved = false;
          }
        }}
      >
        {tracks.slice(0, 12).map((track) => (
          <MusicCard
            key={track.id}
            track={track}
            onPlay={onPlay}
            onAlbum={onAlbum}
            onPreview={
              onPreview
                ? (id) => {
                    if (!gesture.current.down) onPreview(id);
                  }
                : undefined
            }
          />
        ))}
      </div>
    </div>
  );
});

export const SearchField = memo(function SearchField({
  onSearch,
}: {
  onSearch: (query: string) => void;
}) {
  const [value, setValue] = useState("");
  useEffect(() => {
    const timer = window.setTimeout(() => onSearch(value), 200);
    return () => clearTimeout(timer);
  }, [value, onSearch]);
  return (
    <label className="sp-search">
      <Search size={17} />
      <input
        aria-label="Search sample music"
        placeholder="Find your next favourite"
        value={value}
        onChange={(event) => setValue(event.target.value)}
      />
      {value && (
        <button
          className="sp-icon"
          aria-label="Clear search"
          onClick={() => setValue("")}
        >
          <X size={15} />
        </button>
      )}
      <kbd>/</kbd>
    </label>
  );
});

export const LikeButton = memo(function LikeButton({
  liked,
  onClick,
  title,
}: {
  liked: boolean;
  onClick: () => void;
  title: string;
}) {
  return (
    <button
      className={`sp-icon ${liked ? "sp-is-liked" : ""}`}
      aria-label={`${liked ? "Unlike" : "Like"} ${title}`}
      aria-pressed={liked}
      onClick={onClick}
    >
      <Heart size={20} fill={liked ? "currentColor" : "none"} />
    </button>
  );
});
