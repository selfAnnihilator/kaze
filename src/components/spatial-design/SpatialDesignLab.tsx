import React, {
  Suspense,
  lazy,
  memo,
  useCallback,
  useEffect,
  useRef,
  useState,
} from "react";
import {
  ArrowLeft,
  ArrowRight,
  ArrowUpRight,
  Music2,
  Search,
  SlidersHorizontal,
} from "lucide-react";
import {
  ALBUMS,
  INITIAL_QUEUE,
  SpatialTrack,
  TRACKS,
  standardHref,
} from "./data";
import { Artwork, SearchField } from "./Primitives";
import { Home } from "./Home";
import { NowPlaying, Transport } from "./Player";
import { Navigation, NAV, Destination } from "./Navigation";
import { TrackList } from "./Library";
import "./spatial.css";

const Discovery = lazy(() =>
  import("./Discovery").then((module) => ({ default: module.Discovery })),
);
const Library = lazy(() =>
  import("./Library").then((module) => ({ default: module.Library })),
);
const Collection = lazy(() =>
  import("./Library").then((module) => ({ default: module.Collection })),
);
const Queue = memo(function Queue({
  queue,
  cursor,
  onPlay,
  onAlbum,
  onMove,
  onClear,
}: {
  queue: SpatialTrack[];
  cursor: number;
  onPlay: (id: string) => void;
  onAlbum: (id: string) => void;
  onMove: (source: string, destination: string) => void;
  onClear: () => void;
}) {
  const upcoming = React.useMemo(
    () => queue.slice(cursor + 1),
    [queue, cursor],
  );
  return (
    <div className="sp-page sp-queue-page">
      <div className="sp-page-heading">
        <p className="sp-kicker">Keep a good thing going</p>
        <h1>
          What’s next<span className="sp-orange">.</span>
        </h1>
        <p>Your queue, with a little room to move.</p>
      </div>
      <div className="sp-queue-path">
        <div className="sp-queue-present">
          <span>Now Playing</span>
          <Artwork
            src={queue[cursor].artwork}
            title={queue[cursor].album}
            eager
          />
          <h2>{queue[cursor].title}</h2>
          <p>{queue[cursor].artist}</p>
        </div>
        <ArrowRight className="sp-queue-path-arrow" size={30} />
        <div className="sp-queue-future">
          {upcoming.slice(0, 3).map((track, index) => (
            <button
              key={track.id}
              onClick={() => onPlay(track.id)}
              style={{ "--queue-order": index } as React.CSSProperties}
              aria-label={`Play ${track.title} next`}
            >
              <Artwork src={track.artwork} title={track.album} />
              <span>
                0{index + 1}
                <strong>{track.title}</strong>
                <small>{track.artist}</small>
              </span>
            </button>
          ))}
        </div>
      </div>
      <div className="sp-section-title">
        <div>
          <p className="sp-kicker">In order, on purpose</p>
          <h2>
            Up next <span className="sp-count">{upcoming.length}</span>
          </h2>
        </div>
        <button
          className="sp-text-action"
          disabled={!upcoming.length}
          onClick={onClear}
        >
          Clear upcoming
        </button>
      </div>
      <p className="sp-queue-help">
        Drag a track to reorder, or use its up and down buttons.
      </p>
      <TrackList
        tracks={upcoming}
        currentId={queue[cursor].id}
        onPlay={onPlay}
        onAlbum={onAlbum}
        onMove={onMove}
        label="Upcoming queue"
      />
      {!upcoming.length && (
        <button className="sp-primary" onClick={() => onAlbum(ALBUMS[0].id)}>
          <Search size={16} />
          Find your next track
        </button>
      )}
    </div>
  );
});

export default function SpatialDesignLab() {
  const [navigation, setNavigation] = useState<{
    destination: Destination;
    history: Destination[];
  }>({ destination: { page: "home" }, history: [] });
  const destination = navigation.destination;
  const scroll = useRef<HTMLElement>(null);
  const [model, setModel] = useState({
    queue: INITIAL_QUEUE,
    cursor: 2,
    playing: false,
    flow: false,
    shuffle: false,
    repeat: 0,
  });
  const [liked, setLiked] = useState(
    () => new Set(["migration-0", "modal-soul-0", "in-rainbows-0"]),
  );
  const [query, setQuery] = useState("");
  const [showInfo, setShowInfo] = useState(false);
  const track = model.queue[model.cursor];
  const navigate = useCallback(
    (next: Destination) =>
      setNavigation((current) => {
        if (
          current.destination.page === next.page &&
          current.destination.id === next.id
        )
          return current;
        return {
          destination: next,
          history: [...current.history, current.destination],
        };
      }),
    [],
  );
  useEffect(() => {
    scroll.current?.scrollTo({ top: 0 });
  }, [destination]);
  const onAlbum = useCallback(
    (id: string) => navigate({ page: "collection", id }),
    [navigate],
  );
  const onPlaylist = useCallback(
    (id: string) => navigate({ page: "collection", id, playlist: true }),
    [navigate],
  );
  const onPlay = useCallback(
    (id: string) =>
      setModel((current) => {
        const index = current.queue.findIndex((item) => item.id === id);
        if (index >= 0) return { ...current, cursor: index, playing: true };
        const selected = TRACKS.find((item) => item.id === id);
        return selected
          ? {
              ...current,
              queue: [...current.queue, selected],
              cursor: current.queue.length,
              playing: true,
            }
          : current;
      }),
    [],
  );
  const onToggle = useCallback(
    () => setModel((current) => ({ ...current, playing: !current.playing })),
    [],
  );
  const onPlaySelection = useCallback((selection: SpatialTrack[]) => {
    if (selection.length)
      setModel((current) => ({
        ...current,
        queue: selection,
        cursor: 0,
        playing: true,
      }));
  }, []);
  const onNext = useCallback(
    () =>
      setModel((current) => ({
        ...current,
        cursor:
          current.repeat === 1
            ? (current.cursor + 1) % current.queue.length
            : Math.min(current.cursor + 1, current.queue.length - 1),
      })),
    [],
  );
  const onPrevious = useCallback(
    () =>
      setModel((current) => ({
        ...current,
        cursor: Math.max(current.cursor - 1, 0),
      })),
    [],
  );
  const onLike = useCallback(
    () =>
      setLiked((current) => {
        const next = new Set(current);
        if (next.has(track.id)) next.delete(track.id);
        else next.add(track.id);
        return next;
      }),
    [track.id],
  );
  const onShuffle = useCallback(() => {
    const eventSeed = Math.floor(Math.random() * 0x7fffffff);
    setModel((current) => {
      if (current.shuffle) return { ...current, shuffle: false };
      const upcoming = current.queue.slice(current.cursor + 1);
      let seed = eventSeed;
      for (let index = upcoming.length - 1; index > 0; index--) {
        seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0;
        const target = seed % (index + 1);
        [upcoming[index], upcoming[target]] = [
          upcoming[target],
          upcoming[index],
        ];
      }
      return {
        ...current,
        shuffle: true,
        queue: [...current.queue.slice(0, current.cursor + 1), ...upcoming],
      };
    });
  }, []);
  const onRepeat = useCallback(
    () =>
      setModel((current) => ({ ...current, repeat: (current.repeat + 1) % 3 })),
    [],
  );
  const onNowPlaying = useCallback(
    () => navigate({ page: "now-playing" }),
    [navigate],
  );
  const onQueue = useCallback(() => navigate({ page: "queue" }), [navigate]);
  const onDiscover = useCallback(
    () => navigate({ page: "discover" }),
    [navigate],
  );
  const onFlow = useCallback(
    () =>
      setModel((current) => ({
        ...current,
        playing: !current.flow,
        flow: !current.flow,
      })),
    [],
  );
  const onSearch = useCallback(
    (value: string) => {
      setQuery(value);
      if (value.trim()) navigate({ page: "library" });
    },
    [navigate],
  );
  const onMove = useCallback(
    (source: string, target: string) =>
      setModel((current) => {
        const from = current.queue.findIndex((item) => item.id === source),
          to = current.queue.findIndex((item) => item.id === target);
        if (from <= current.cursor || to <= current.cursor || from === to)
          return current;
        const queue = [...current.queue],
          [item] = queue.splice(from, 1);
        queue.splice(to, 0, item);
        return { ...current, queue };
      }),
    [],
  );
  const onClear = useCallback(
    () =>
      setModel((current) => ({
        ...current,
        queue: current.queue.slice(0, current.cursor + 1),
      })),
    [],
  );
  useEffect(() => {
    const keyboard = (event: KeyboardEvent) => {
      if (
        event.ctrlKey ||
        event.metaKey ||
        event.altKey ||
        (event.target instanceof HTMLElement &&
          event.target.closest(
            "input, textarea, button, a, [contenteditable=true]",
          ))
      )
        return;
      if (event.key === "/") {
        event.preventDefault();
        document.querySelector<HTMLInputElement>(".sp-search input")?.focus();
      }
      if (event.code === "Space") {
        event.preventDefault();
        onToggle();
      }
    };
    window.addEventListener("keydown", keyboard);
    return () => window.removeEventListener("keydown", keyboard);
  }, [onToggle]);
  return (
    <div className="spatial-kaze">
      <a className="sp-skip" href="#sp-main">
        Skip to music
      </a>
      <Navigation page={destination.page} onNavigate={navigate} />
      <div className="sp-shell">
        <header className="sp-topbar">
          <div className="sp-topbar-left">
            <button
              className="sp-icon"
              aria-label="Go back"
              disabled={!navigation.history.length}
              onClick={() =>
                setNavigation((current) =>
                  current.history.length
                    ? {
                        destination:
                          current.history[current.history.length - 1],
                        history: current.history.slice(0, -1),
                      }
                    : current,
                )
              }
            >
              <ArrowLeft size={19} />
            </button>
            <span className="sp-breadcrumb">
              {destination.page === "collection"
                ? "Library / Collection"
                : NAV.find((item) => item.id === destination.page)?.label}
            </span>
          </div>
          <SearchField onSearch={onSearch} />
          <button
            className="sp-lab-label"
            aria-expanded={showInfo}
            onClick={() => setShowInfo(!showInfo)}
          >
            <SlidersHorizontal size={14} />
            <span>Design lab</span>
          </button>
        </header>
        {showInfo && (
          <div className="sp-lab-info">
            <Music2 size={19} />
            <p>
              <strong>A visual music-player exploration.</strong> Sample music
              and controls are local to this preview. No audio plays; your real
              playback, Flow, library and settings are untouched.
            </p>
            <a href={standardHref()}>
              Return to current Kaze
              <ArrowUpRight size={16} />
            </a>
          </div>
        )}
        <main
          className={`sp-scroll ${destination.page === "discover" || destination.page === "now-playing" ? "sp-scroll-fixed" : ""}`}
          id="sp-main"
          ref={scroll}
          tabIndex={-1}
        >
          <Suspense
            fallback={
              <div className="sp-loading" role="status">
                <span />
                <span />
                <p>Opening your music…</p>
              </div>
            }
          >
            {destination.page === "home" && (
              <Home
                onPlay={onPlay}
                onAlbum={onAlbum}
                onPlaylist={onPlaylist}
                onDiscover={onDiscover}
                flow={model.flow}
                onFlow={onFlow}
                current={track}
              />
            )}
            {destination.page === "discover" && (
              <Discovery
                onPlay={onPlay}
                onAlbum={onAlbum}
                onPlaySelection={onPlaySelection}
              />
            )}
            {destination.page === "library" && (
              <Library
                currentId={track.id}
                onPlay={onPlay}
                onAlbum={onAlbum}
                onPlaylist={onPlaylist}
                query={query}
                liked={liked}
              />
            )}
            {destination.page === "collection" && (
              <Collection
                id={destination.id ?? "migration"}
                playlist={destination.playlist}
                currentId={track.id}
                onPlay={onPlay}
                onAlbum={onAlbum}
                onPlaySelection={onPlaySelection}
              />
            )}
            {destination.page === "now-playing" && (
              <NowPlaying
                track={track}
                playing={model.playing}
                liked={liked.has(track.id)}
                onToggle={onToggle}
                onLike={onLike}
                queue={model.queue}
                cursor={model.cursor}
                onSelect={onPlay}
              />
            )}
            {destination.page === "queue" && (
              <Queue
                queue={model.queue}
                cursor={model.cursor}
                onPlay={onPlay}
                onAlbum={onAlbum}
                onMove={onMove}
                onClear={onClear}
              />
            )}
          </Suspense>
          <div className="sp-page-footnote">
            <span>Sample library</span>
            <span>No audio in this preview</span>
          </div>
        </main>
      </div>
      <Transport
        track={track}
        playing={model.playing}
        liked={liked.has(track.id)}
        onToggle={onToggle}
        onLike={onLike}
        onNext={onNext}
        onPrevious={onPrevious}
        onQueue={onQueue}
        onNowPlaying={onNowPlaying}
        canNext={model.cursor < model.queue.length - 1 || model.repeat === 1}
        canPrevious={model.cursor > 0}
        shuffle={model.shuffle}
        repeat={model.repeat}
        onShuffle={onShuffle}
        onRepeat={onRepeat}
      />
    </div>
  );
}
