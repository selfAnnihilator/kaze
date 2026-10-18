import { Dispatch, memo, useMemo, useRef, useState } from "react";
import {
  ArrowLeft,
  ArrowRight,
  ArrowUpRight,
  Heart,
  ListMusic,
  Play,
  X,
} from "lucide-react";
import {
  ALBUMS,
  Action,
  CHAPTERS,
  DeckState,
  PLAYLISTS,
  TRACKS,
  Track,
} from "./model";
import { Cover, Empty, SleeveButton, Ticket, TrackRows } from "./Parts";

interface MusicActions {
  onPlay: (id: string) => void;
  onAlbum: (id: string) => void;
}

export const Listen = memo(function Listen({
  state,
  onPlay,
  onAlbum,
}: MusicActions & { state: DeckState }) {
  const track = state.queue[state.cursor];
  const upcoming = state.queue.slice(state.cursor + 1, state.cursor + 4);
  return (
    <section className="bk-listen" aria-label="Now Playing">
      <div className="bk-poster-title">
        <span className="bk-micro">
          {state.flow ? "FLOW / CURRENT SELECTION" : "NOW PLAYING"}
        </span>
        <h1 title={track.title}>{track.title}</h1>
      </div>
      <div className="bk-poster-body">
        <button
          className="bk-poster-cover"
          onClick={() => onAlbum(track.albumId)}
          aria-label={`Open ${track.album} by ${track.artist}`}
        >
          <Cover key={track.artwork} track={track} eager />
          <span className="bk-cover-open">
            OPEN RECORD <ArrowUpRight size={20} />
          </span>
        </button>
        <div className="bk-poster-annotation">
          <div className="bk-poster-count" aria-hidden="true">
            {String(state.cursor + 1).padStart(2, "0")}
          </div>
          <span className="bk-micro">
            OF {String(state.queue.length).padStart(2, "0")} TRACKS IN THE QUEUE
          </span>
          <h2>{track.artist}</h2>
          <button
            className="bk-text-button"
            onClick={() => onAlbum(track.albumId)}
          >
            {track.album}
            <ArrowUpRight size={23} />
          </button>
          <span className="bk-genre">{track.genre}</span>
        </div>
      </div>
      <div className="bk-poster-future">
        <div className="bk-line-label">
          <span>NEXT ON DECK</span>
          <span>{upcoming.length ? "PICK YOUR NEXT MOVE" : "END OF SIDE"}</span>
        </div>
        <div className="bk-ticket-row">
          {upcoming.length ? (
            upcoming.map((item, index) => (
              <Ticket
                key={item.id}
                track={item}
                index={index}
                onPlay={onPlay}
              />
            ))
          ) : (
            <p className="bk-side-end">
              Nothing after this. Leave a little room for a new record.
            </p>
          )}
        </div>
      </div>
    </section>
  );
});

export const Discover = memo(function Discover({
  onPlay,
  onAlbum,
  onSelection,
}: MusicActions & { onSelection: (tracks: Track[]) => void }) {
  const [bin, setBin] = useState(0);
  const wall = useRef<HTMLDivElement>(null);
  const drag = useRef({ down: false, moved: false, x: 0, scroll: 0 });
  const chapter = CHAPTERS[bin];
  const changeBin = (offset: number) => {
    setBin((current) => (current + offset + CHAPTERS.length) % CHAPTERS.length);
    wall.current?.scrollTo({ left: 0 });
  };
  const browse = (offset: number) =>
    wall.current?.scrollBy({
      left: offset * (wall.current.clientWidth * 0.75),
      behavior: matchMedia("(prefers-reduced-motion: reduce)").matches
        ? "auto"
        : "smooth",
    });
  return (
    <section className="bk-discover">
      <header className="bk-view-heading">
        <div>
          <span className="bk-micro">DISCOVER / TAKE A DIFFERENT TURN</span>
          <h1>
            OFF THE
            <br />
            BEATEN TRACK.
          </h1>
        </div>
        <span className="bk-big-index" aria-hidden="true">
          {String(bin + 1).padStart(2, "0")}
        </span>
      </header>
      <div className="bk-bin-heading">
        <div>
          <h2>{chapter.title}</h2>
          <p>{chapter.subtitle}</p>
        </div>
        <div className="bk-bin-controls">
          <button
            className="bk-icon"
            onClick={() => changeBin(-1)}
            aria-label="Previous discovery selection"
          >
            <ArrowLeft size={21} />
          </button>
          <span className="bk-micro">
            {bin + 1} / {CHAPTERS.length}
          </span>
          <button
            className="bk-icon"
            onClick={() => changeBin(1)}
            aria-label="Next discovery selection"
          >
            <ArrowRight size={21} />
          </button>
        </div>
      </div>
      <div className="bk-wall-controls">
        <button
          className="bk-text-button"
          onClick={() => onSelection(chapter.tracks)}
        >
          <Play size={15} fill="currentColor" /> Play this selection
        </button>
        <div>
          <button
            className="bk-icon"
            onClick={() => browse(-1)}
            aria-label="Browse earlier records"
          >
            <ArrowLeft size={20} />
          </button>
          <button
            className="bk-icon"
            onClick={() => browse(1)}
            aria-label="Browse later records"
          >
            <ArrowRight size={20} />
          </button>
        </div>
      </div>
      <div
        className="bk-sleeve-wall"
        ref={wall}
        tabIndex={0}
        aria-label="Discovery records"
        onKeyDown={(event) => {
          drag.current.moved = false;
          if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
            event.preventDefault();
            browse(event.key === "ArrowLeft" ? -1 : 1);
          }
        }}
        onPointerDown={(event) => {
          if (event.pointerType === "mouse" && event.button === 0)
            drag.current = {
              down: true,
              moved: false,
              x: event.clientX,
              scroll: event.currentTarget.scrollLeft,
            };
        }}
        onPointerMove={(event) => {
          const gesture = drag.current;
          if (!gesture.down) return;
          const distance = event.clientX - gesture.x;
          if (Math.abs(distance) > 6) {
            gesture.moved = true;
            event.currentTarget.setPointerCapture(event.pointerId);
          }
          if (gesture.moved)
            event.currentTarget.scrollLeft = gesture.scroll - distance;
        }}
        onPointerUp={(event) => {
          drag.current.down = false;
          if (event.currentTarget.hasPointerCapture(event.pointerId))
            event.currentTarget.releasePointerCapture(event.pointerId);
        }}
        onPointerCancel={() => {
          drag.current.down = false;
          drag.current.moved = false;
        }}
        onClickCapture={(event) => {
          if (drag.current.moved) {
            event.preventDefault();
            event.stopPropagation();
            drag.current.moved = false;
          }
        }}
      >
        {chapter.tracks.map((track, index) => (
          <SleeveButton
            key={track.id}
            track={track}
            index={index}
            onPlay={onPlay}
            onAlbum={onAlbum}
          />
        ))}
      </div>
      <p className="bk-wall-note">Six records. Follow one somewhere new.</p>
    </section>
  );
});

export const Library = memo(function Library({
  state,
  onPlay,
  onAlbum,
  onSelection,
}: MusicActions & {
  state: DeckState;
  onSelection: (tracks: Track[]) => void;
}) {
  const [tab, setTab] = useState<"albums" | "tracks" | "playlists" | "liked">(
    "albums",
  );
  const tracks = useMemo(
    () =>
      tab === "liked"
        ? TRACKS.filter((track) => state.liked.includes(track.id))
        : TRACKS,
    [tab, state.liked],
  );
  return (
    <section className="bk-library">
      <header className="bk-view-heading">
        <div>
          <span className="bk-micro">YOUR MUSIC / ALL IN ONE PLACE</span>
          <h1>
            THE
            <br />
            ARCHIVE.
          </h1>
        </div>
        <span className="bk-big-index" aria-hidden="true">
          12
        </span>
      </header>
      <div
        className="bk-tabs"
        role="tablist"
        aria-label="Library sections"
        onKeyDown={(event) => {
          const tabs = ["albums", "tracks", "playlists", "liked"] as const;
          const current = tabs.indexOf(tab);
          const next =
            event.key === "Home"
              ? 0
              : event.key === "End"
                ? tabs.length - 1
                : event.key === "ArrowRight"
                  ? (current + 1) % tabs.length
                  : event.key === "ArrowLeft"
                    ? (current + tabs.length - 1) % tabs.length
                    : -1;
          if (next < 0) return;
          event.preventDefault();
          setTab(tabs[next]);
          event.currentTarget
            .querySelectorAll<HTMLButtonElement>('[role="tab"]')
            [next]?.focus();
        }}
      >
        {(["albums", "tracks", "playlists", "liked"] as const).map((item) => (
          <button
            key={item}
            role="tab"
            tabIndex={item === tab ? 0 : -1}
            aria-selected={item === tab}
            aria-controls="bk-library-panel"
            id={`bk-tab-${item}`}
            onClick={() => setTab(item)}
          >
            {item === "liked" ? <Heart size={15} /> : null}
            {item}{" "}
            <span>
              {item === "albums"
                ? ALBUMS.length
                : item === "tracks"
                  ? TRACKS.length
                  : item === "playlists"
                    ? PLAYLISTS.length
                    : state.liked.length}
            </span>
          </button>
        ))}
      </div>
      <div
        className={`bk-library-panel bk-library-${tab}`}
        role="tabpanel"
        id="bk-library-panel"
        aria-labelledby={`bk-tab-${tab}`}
      >
        {tab === "albums" && (
          <>
            <div className="bk-record-spines" aria-label="Album shelf">
              {ALBUMS.map((album, index) => (
                <button
                  key={album.id}
                  className="bk-record-spine"
                  onClick={() => onAlbum(album.id)}
                  aria-label={`Open ${album.title} by ${album.artist}`}
                >
                  <span>{String(index + 1).padStart(2, "0")}</span>
                  <Cover track={album} />
                  <strong>{album.title}</strong>
                  <small>{album.artist}</small>
                </button>
              ))}
            </div>
            <div className="bk-archive-footer">
              <span>12 ALBUMS. NO WRONG PLACE TO START.</span>
              <button
                className="bk-text-button"
                onClick={() => setTab("tracks")}
              >
                See every track <ArrowUpRight size={22} />
              </button>
            </div>
          </>
        )}
        {(tab === "tracks" || tab === "liked") &&
          (tracks.length ? (
            <TrackRows
              tracks={tracks}
              currentId={state.queue[state.cursor].id}
              onPlay={onPlay}
            />
          ) : (
            <Empty
              title="Keep the good ones."
              text="Like a track from the deck and it will live here."
            />
          ))}
        {tab === "playlists" && (
          <div className="bk-playlist-list">
            {PLAYLISTS.map((playlist, index) => (
              <button
                key={playlist.id}
                onClick={() => onSelection(playlist.tracks)}
                aria-label={`Play playlist ${playlist.name}`}
              >
                <span className="bk-playlist-number">0{index + 1}</span>
                <Cover
                  track={{ artwork: playlist.cover, title: playlist.name }}
                />
                <span>
                  <strong>{playlist.name}</strong>
                  <small>
                    {playlist.description} / {playlist.tracks.length} tracks
                  </small>
                </span>
                <Play size={24} fill="currentColor" />
              </button>
            ))}
          </div>
        )}
      </div>
    </section>
  );
});

export const Collection = memo(function Collection({
  id,
  state,
  onPlay,
  onSelection,
  onBack,
}: {
  id: string;
  state: DeckState;
  onPlay: (id: string) => void;
  onSelection: (tracks: Track[]) => void;
  onBack: () => void;
}) {
  const album = ALBUMS.find((item) => item.id === id) ?? ALBUMS[0];
  const tracks = useMemo(
    () => TRACKS.filter((track) => track.albumId === album.id),
    [album.id],
  );
  return (
    <section className="bk-collection">
      <button className="bk-text-button bk-collection-back" onClick={onBack}>
        <ArrowLeft size={18} /> BACK TO THE MUSIC
      </button>
      <div className="bk-collection-hero">
        <Cover key={album.id} track={album} eager />
        <div>
          <span className="bk-micro">
            ALBUM / {album.year} / {tracks.length} SAMPLE TRACKS
          </span>
          <h1>{album.title}</h1>
          <p>{album.artist}</p>
          <button
            className="bk-solid-button"
            onClick={() => onSelection(tracks)}
          >
            <Play size={20} fill="currentColor" /> PLAY RECORD
          </button>
        </div>
      </div>
      <TrackRows
        tracks={tracks}
        currentId={state.queue[state.cursor].id}
        onPlay={onPlay}
      />
    </section>
  );
});

export const Queue = memo(function Queue({
  state,
  dispatch,
  onPlay,
  onDiscover,
}: {
  state: DeckState;
  dispatch: Dispatch<Action>;
  onPlay: (id: string) => void;
  onDiscover: () => void;
}) {
  const tracks = useMemo(
    () => state.queue.slice(state.cursor + 1),
    [state.queue, state.cursor],
  );
  const onMove = useMemo(
    () => (id: string, offset: number) =>
      dispatch({ type: "move", id, offset }),
    [dispatch],
  );
  const onRemove = useMemo(
    () => (id: string) => dispatch({ type: "remove", id }),
    [dispatch],
  );
  return (
    <section className="bk-queue">
      <header className="bk-view-heading">
        <div>
          <span className="bk-micro">THE QUEUE / IN YOUR ORDER</span>
          <h1>
            WHAT
            <br />
            COMES NEXT.
          </h1>
        </div>
        <span className="bk-big-index">
          {String(tracks.length).padStart(2, "0")}
        </span>
      </header>
      <div className="bk-queue-heading">
        <span>
          <ListMusic size={18} /> UPCOMING TRACKS
        </span>
        <button
          className="bk-text-button"
          disabled={!tracks.length}
          onClick={() => dispatch({ type: "clear" })}
        >
          Clear upcoming <X size={16} />
        </button>
      </div>
      {tracks.length ? (
        <TrackRows
          tracks={tracks}
          currentId={state.queue[state.cursor].id}
          onPlay={onPlay}
          onMove={onMove}
          onRemove={onRemove}
        />
      ) : (
        <Empty
          title="An open ending."
          text="Your current record stays on the deck. Find something to follow it."
          onDiscover={onDiscover}
        />
      )}
    </section>
  );
});

export const SearchResults = memo(function SearchResults({
  query,
  currentId,
  onPlay,
}: {
  query: string;
  currentId: string;
  onPlay: (id: string) => void;
}) {
  const tracks = useMemo(() => {
    const term = query.toLocaleLowerCase();
    return TRACKS.filter((track) =>
      `${track.title} ${track.artist} ${track.album}`
        .toLocaleLowerCase()
        .includes(term),
    );
  }, [query]);
  return (
    <section className="bk-search-results">
      <header className="bk-view-heading">
        <div>
          <span className="bk-micro">SEARCH / {tracks.length} MATCHES</span>
          <h1>
            FOUND
            <br />
            IN THE STACK.
          </h1>
          <p className="bk-search-query">Results for “{query}”</p>
        </div>
      </header>
      {tracks.length ? (
        <TrackRows tracks={tracks} currentId={currentId} onPlay={onPlay} />
      ) : (
        <Empty
          title="No record of that."
          text="Try a different song, artist, or album from the sample library."
        />
      )}
    </section>
  );
});
