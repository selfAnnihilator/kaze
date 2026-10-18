import {
  memo,
  useCallback,
  useEffect,
  useReducer,
  useRef,
  useState,
} from "react";
import { ArrowLeft, ArrowUpRight } from "lucide-react";
import { Deck } from "./Deck";
import { Cover, SearchBox } from "./Parts";
import {
  Collection,
  Discover,
  Library,
  Listen,
  Queue,
  SearchResults,
} from "./Views";
import {
  Destination,
  Page,
  Track,
  deckReducer,
  initialDeck,
  standardHref,
} from "./model";
import "./brutalist.css";

const destinations: { page: Page; label: string }[] = [
  { page: "listen", label: "Now Playing" },
  { page: "discover", label: "Discover" },
  { page: "library", label: "Library" },
  { page: "queue", label: "Queue" },
];
const Header = memo(function Header({
  page,
  onNavigate,
}: {
  page: Page;
  onNavigate: (next: Destination) => void;
}) {
  return (
    <header className="bk-header">
      <a
        className="bk-wordmark"
        href="?lab=brutalist"
        aria-label="Kaze brutalist preview"
      >
        KAZE<span>音楽を、自由に。</span>
      </a>
      <nav aria-label="Main navigation">
        {destinations.map((item, index) => (
          <button
            key={item.page}
            aria-label={item.label}
            aria-current={page === item.page ? "page" : undefined}
            onClick={() => onNavigate({ page: item.page })}
          >
            <span aria-hidden="true">0{index + 1}</span>
            {item.label}
          </button>
        ))}
      </nav>
      <div className="bk-edition">
        <span>BRUTALIST STUDY</span>
        <a href="?lab=spatial">
          Spatial design <ArrowUpRight size={16} />
        </a>
      </div>
    </header>
  );
});

export default function BrutalistDesignLab() {
  const root = useRef<HTMLDivElement>(null);
  const [state, dispatch] = useReducer(deckReducer, initialDeck);
  const [navigation, setNavigation] = useState<{
    current: Destination;
    history: Destination[];
    key: number;
  }>({ current: { page: "listen" }, history: [], key: 0 });
  const [query, setQuery] = useState("");
  const navigate = useCallback((current: Destination) => {
    setQuery("");
    setNavigation((previous) => ({
      current,
      history: [...previous.history, previous.current],
      key: previous.key + 1,
    }));
  }, []);
  const back = useCallback(() => {
    setQuery("");
    setNavigation((previous) => ({
      current: previous.history[previous.history.length - 1] ?? {
        page: "library",
      },
      history: previous.history.slice(0, -1),
      key: previous.key + 1,
    }));
  }, []);
  const onPlay = useCallback(
    (id: string) => dispatch({ type: "play", id }),
    [],
  );
  const onAlbum = useCallback(
    (id: string) => navigate({ page: "collection", id }),
    [navigate],
  );
  const onSelection = useCallback(
    (tracks: Track[]) => dispatch({ type: "selection", tracks }),
    [],
  );
  const onSearch = useCallback((value: string) => setQuery(value), []);
  const onQueue = useCallback(() => navigate({ page: "queue" }), [navigate]);
  const onDiscover = useCallback(
    () => navigate({ page: "discover" }),
    [navigate],
  );
  const track = state.queue[state.cursor];
  const page = navigation.current.page;
  useEffect(() => {
    root.current?.scrollTo({ top: 0 });
  }, [navigation.key]);
  useEffect(() => {
    const shortcut = (event: KeyboardEvent) => {
      if (
        event.code !== "Space" ||
        event.repeat ||
        event.ctrlKey ||
        event.metaKey ||
        event.altKey ||
        !(event.target instanceof HTMLElement) ||
        event.target.closest(
          "input, textarea, button, a, [contenteditable=true]",
        )
      )
        return;
      event.preventDefault();
      dispatch({ type: "toggle" });
    };
    window.addEventListener("keydown", shortcut);
    return () => window.removeEventListener("keydown", shortcut);
  }, []);
  return (
    <div className="brutalist-kaze" ref={root}>
      <a className="bk-skip" href="#bk-main">
        Skip to music
      </a>
      <Header page={query ? "search" : page} onNavigate={navigate} />
      <div className="bk-content">
        <div className="bk-toolbar">
          <div>
            <a
              className="bk-icon"
              href={standardHref()}
              aria-label="Return to current Kaze"
              title="Return to current Kaze"
            >
              <ArrowLeft size={19} />
            </a>
            <span className="bk-micro">MUSIC, WITHOUT THE NOISE.</span>
          </div>
          <SearchBox key={navigation.key} onSearch={onSearch} />
        </div>
        <main
          id="bk-main"
          tabIndex={-1}
          className={`bk-main ${!query && (page === "listen" || page === "discover") ? `bk-main-fixed ${page === "discover" ? "bk-main-discovery" : ""}` : ""}`}
          key={navigation.key}
        >
          {query ? (
            <SearchResults query={query} currentId={track.id} onPlay={onPlay} />
          ) : (
            <>
              {page === "listen" && (
                <Listen state={state} onPlay={onPlay} onAlbum={onAlbum} />
              )}
              {page === "discover" && (
                <Discover
                  onPlay={onPlay}
                  onAlbum={onAlbum}
                  onSelection={onSelection}
                />
              )}
              {page === "library" && (
                <Library
                  state={state}
                  onPlay={onPlay}
                  onAlbum={onAlbum}
                  onSelection={onSelection}
                />
              )}
              {page === "queue" && (
                <Queue
                  state={state}
                  dispatch={dispatch}
                  onPlay={onPlay}
                  onDiscover={onDiscover}
                />
              )}
              {page === "collection" && (
                <Collection
                  id={navigation.current.id ?? "migration"}
                  state={state}
                  onPlay={onPlay}
                  onSelection={onSelection}
                  onBack={back}
                />
              )}
            </>
          )}
        </main>
      </div>
      <Deck
        state={state}
        dispatch={dispatch}
        onQueue={onQueue}
        onPlay={onPlay}
      />
      <div className="bk-mobile-track">
        <Cover key={track.artwork} track={track} />
        <span>
          {track.title}
          <small>{track.artist}</small>
        </span>
        <button
          onClick={() => {
            document
              .querySelector(".bk-deck")
              ?.scrollIntoView({ block: "start" });
          }}
          aria-label="Show playback controls"
        >
          THE DECK <ArrowUpRight size={19} />
        </button>
      </div>
    </div>
  );
}
