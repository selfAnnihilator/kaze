import {
  Children,
  memo,
  createContext,
  useCallback,
  useContext,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import {
  ArrowLeft,
  ArrowRight,
  ArrowUpRight,
  Pause,
  Play,
  RefreshCw,
} from "lucide-react";
import { Artwork } from "./Artwork";
import { useInterfaceStyle } from "./InterfaceContext";
import { useDiscoveryMemory } from "./ViewMemory";

const chapters = ["Trending Songs", "For You", "Mixes For You", "Top Charts"];

/** Overflow keeps ordinary scrolling, even at either end of the scroll range. */
function overflowingRegion(root: HTMLElement): HTMLElement | undefined {
  return [root, root.querySelector<HTMLElement>(".chapter-stage")].find(
    (region): region is HTMLElement =>
      !!region && region.scrollHeight > region.clientHeight + 1 &&
      /^(auto|scroll)$/.test(getComputedStyle(region).overflowY),
  );
}

/** Real discovery chapters in each theme's independent stage composition. */
export type DiscoveryPreview = {
  id: string;
  title: string;
  artist: string;
  album?: string;
  cover?: string | null;
  artworkLabel?: string;
  loading?: boolean;
  playing?: boolean;
  onPlay: () => void;
  onOpen?: () => void;
};
export type DiscoveryChapter = {
  title: string;
  subtitle: string;
  previews: DiscoveryPreview[];
  onRefresh?: () => Promise<void>;
  isRefreshing?: boolean;
};
const PreviewContext = createContext<
  ((preview: DiscoveryPreview) => void) | null
>(null);
export function DiscoveryPreviewTarget({
  preview,
  children,
  className = "",
}: {
  preview: DiscoveryPreview;
  children: ReactNode;
  className?: string;
}) {
  const select = useContext(PreviewContext);
  return (
    <div
      className={`discovery-preview-target ${className}`}
      onMouseEnter={() => select?.(preview)}
      onFocus={() => select?.(preview)}
    >
      {children}
    </div>
  );
}

export const DiscoveryChapters = memo(function DiscoveryChapters({
  children,
  selections,
}: {
  children: ReactNode;
  selections: DiscoveryChapter[];
}) {
  const [refreshing, setRefreshing] = useState<number | null>(null);
  const [refreshError, setRefreshError] = useState("");
  const style = useInterfaceStyle();
  const memory = useDiscoveryMemory();
  const [active, setActive] = useState(memory.chapter);
  const root = useRef<HTMLDivElement>(null);
  const shelfPositions = memory.shelves;
  const wheelGate = useRef({ last: 0, total: 0, consumed: false });
  const [hovered, setHovered] = useState<{
    chapter: number;
    id: string;
  } | null>(memory.hovered);
  useLayoutEffect(() => {
    memory.chapter = active;
    memory.hovered = hovered;
  }, [active, hovered, memory]);
  const preview =
    (hovered?.chapter === active
      ? selections[active].previews.find((item) => item.id === hovered.id)
      : undefined) || selections[active].previews[0];
  const previewTrack = useCallback(
    (value: DiscoveryPreview) =>
      setHovered((current) =>
        current?.chapter === active && current.id === value.id
          ? current
          : { chapter: active, id: value.id },
      ),
    [active],
  );
  useLayoutEffect(() => {
    if (root.current) {
      root.current.scrollTop = 0;
      const shelf = root.current.querySelector<HTMLElement>(".chapter-stage");
      if (shelf) shelf.scrollTop = 0;
    }
    // Windowed shelves restore themselves after measuring their real width.
    root.current?.querySelector<HTMLElement>(
      ".discovery-chapter .horizontal-scroll-row:not(.windowed-strip)",
    )?.scrollTo({ left: shelfPositions.get(active) || 0, behavior: "instant" });
  }, [active, style, shelfPositions, selections[active].previews.length]);
  useEffect(() => {
    const el = root.current;
    if (!el) return;
    const wheel = (e: WheelEvent) => {
      if (
        e.ctrlKey ||
        Math.abs(e.deltaX) >= Math.abs(e.deltaY) ||
        (style === "brutalist" && e.shiftKey)
      )
        return;
      if (e.target instanceof Element &&
        e.target.closest("input, select, textarea, [contenteditable=true]"))
        return;
      const scrollRegion = overflowingRegion(el);
      if (scrollRegion) {
        wheelGate.current = { last: 0, total: 0, consumed: false };
        // A wheel over the heading cannot natively reach a sibling shelf.
        if (e.target instanceof Node && !scrollRegion.contains(e.target)) {
          e.preventDefault();
          scrollRegion.scrollBy({
            top: e.deltaY * (e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? scrollRegion.clientHeight : 1),
            behavior: "instant",
          });
        }
        return;
      }
      if (
        e.target instanceof Element &&
        style === "spatial" && e.target.closest(".horizontal-scroll-row")
      )
        return;
      e.preventDefault();
      const now = performance.now(),
        gesture = wheelGate.current;
      if (now - gesture.last > 180) {
        gesture.total = 0;
        gesture.consumed = false;
      }
      gesture.last = now;
      gesture.total +=
        e.deltaY *
        (e.deltaMode === 1 ? 16 : e.deltaMode === 2 ? el.clientHeight : 1);
      if (gesture.consumed || Math.abs(gesture.total) < 40) return;
      gesture.consumed = true;
      setActive((current) =>
        Math.max(
          0,
          Math.min(chapters.length - 1, current + Math.sign(gesture.total)),
        ),
      );
    };
    el.addEventListener("wheel", wheel, { passive: false });
    return () => el.removeEventListener("wheel", wheel);
  }, [style]);
  const select = (index: number) => {
    setActive(index);
  };
  const chapterNav = (
    <nav className="chapter-nav" aria-label="Discovery chapters">
      {chapters.map((chapter, index) => (
        <button
          key={chapter}
          aria-current={active === index ? "true" : undefined}
          aria-controls={`discovery-chapter-${index}`}
          onClick={() => select(index)}
        >
          <small>{String(index + 1).padStart(2, "0")}</small>
          <span>{chapter}</span>
        </button>
      ))}
    </nav>
  );
  const isActiveRefreshing = refreshing === active || !!selections[active].isRefreshing;
  const refreshControl = <span className="discovery-refresh-control">
    <button type="button" className="scroll-arrow-btn" aria-label={`Refresh ${selections[active].title}`} title={`Refresh ${selections[active].title}`} disabled={refreshing !== null || isActiveRefreshing} aria-busy={isActiveRefreshing} onClick={async () => {
      const index = active; setRefreshing(index); setRefreshError("");
      try { await selections[index].onRefresh?.(); }
      catch { setRefreshError("Couldn’t refresh. Try again."); }
      finally { setRefreshing(null); }
    }}><RefreshCw size={16} className={isActiveRefreshing ? "spin-animation" : ""} /></button>
    {refreshError && <span role="alert">{refreshError}</span>}
  </span>;
  const content = Children.toArray(children).flatMap((child, index) =>
    active !== index
      ? []
      : [
          <section
            className={`discovery-chapter${style === "spatial" ? " spatial-chapter-content" : ""}`}
            id={`discovery-chapter-${index}`}
            data-chapter-index={index}
            aria-label={chapters[index]}
            key={chapters[index]}
            hidden={style === "spatial" && active !== index}
          >
            {child}
          </section>,
        ],
  );
  return (
    <PreviewContext.Provider value={style === "spatial" ? previewTrack : null}>
      <div
        className={`discovery-chapters${style === "spatial" ? " spatial-discovery-fixed" : " press-discover"}`}
        role="region"
        aria-label="Discover music"
        tabIndex={0}
        onKeyDown={(event) => {
          if (
            event.target instanceof HTMLElement &&
            event.target.closest("input, textarea, select")
          )
            return;
          if (event.key !== "PageDown" && event.key !== "PageUp") return;
          const scrollRegion = root.current && overflowingRegion(root.current);
          if (scrollRegion) {
            event.preventDefault();
            scrollRegion.scrollBy({
              top: scrollRegion.clientHeight * (event.key === "PageDown" ? 0.9 : -0.9),
              behavior: "instant",
            });
            return;
          }
          event.preventDefault();
          select(
            Math.max(
              0,
              Math.min(
                chapters.length - 1,
                active + (event.key === "PageDown" ? 1 : -1),
              ),
            ),
          );
          root.current?.focus({ preventScroll: true });
        }}
        ref={root}
        onScrollCapture={(event) => {
          const el = event.target as HTMLElement;
          if (el.classList.contains("horizontal-scroll-row")) {
            const chapter = el.closest<HTMLElement>("[data-chapter-index]");
            if (chapter)
              shelfPositions.set(
                Number(chapter.dataset.chapterIndex),
                el.scrollLeft,
              );
          }
        }}
      >
        {style === "spatial" ? (
          <>
            <aside
              className="spatial-discovery-artwork"
              aria-label="Album artwork preview"
            >
              <button
                className="spatial-discovery-cover"
                disabled={!preview || (preview.loading && !preview.onOpen)}
                onClick={preview?.onOpen || preview?.onPlay}
                aria-label={
                  preview
                    ? `${preview.onOpen ? "Open" : preview.playing ? "Pause" : "Play"} ${preview.title} by ${preview.artist}`
                    : "No artwork available"
                }
              >
                {preview?.artworkLabel && !preview.cover ? (
                  <span className="spatial-chart-art">
                    {preview.artworkLabel}
                  </span>
                ) : (
                  <Artwork
                    src={preview?.cover}
                    alt={
                      preview
                        ? `${preview.album || preview.title} by ${preview.artist}`
                        : "Discover your next favourite"
                    }
                  />
                )}
              </button>
              <div className="spatial-discovery-caption">
                <h2 title={preview?.title}>
                  {preview?.title || "Your next discovery"}
                </h2>
                <p title={preview?.artist}>
                  {preview?.artist || "Choose another chapter to explore."}
                </p>
              </div>
            </aside>
            <div className="spatial-discovery-panel">
              {chapterNav}
              <div className="spatial-discovery-copy">
                <span className="interface-eyebrow">
                  {String(active + 1).padStart(2, "0")} / 04
                </span>
                <h1>{selections[active].title}</h1>
                <p>{selections[active].subtitle}</p>
                {refreshControl}
              </div>
              <div
                className="spatial-discovery-detail"
                aria-live="polite"
                aria-atomic="true"
              >
                <div>
                  {preview?.onOpen ? (
                    <button
                      className="spatial-preview-title"
                      onClick={preview.onOpen}
                    >
                      {preview.album || preview.title}
                      <ArrowUpRight size={16} />
                    </button>
                  ) : (
                    <h2 title={preview?.album || preview?.title}>
                      {preview?.album || preview?.title || "More music awaits"}
                    </h2>
                  )}
                  <p>
                    {preview?.artist ||
                      "Recommendations grow with your listening."}
                  </p>
                </div>
                {preview && (
                  <button
                    className="spatial-preview-play"
                    onClick={preview.onPlay}
                    aria-label={`${preview.loading ? "Loading" : preview.playing ? "Pause" : "Play"} ${preview.title}`}
                    title={`${preview.loading ? "Loading" : preview.playing ? "Pause" : "Play"} ${preview.title}`}
                    disabled={preview.loading}
                  >
                    {preview.loading ? (
                      <RefreshCw size={19} className="spin-animation" />
                    ) : preview.playing ? (
                      <Pause size={19} fill="currentColor" />
                    ) : (
                      <Play size={19} fill="currentColor" />
                    )}
                  </button>
                )}
              </div>
              <div className="chapter-stage">{content}</div>
            </div>
            <button
              className="spatial-discovery-scroll-cue"
              onClick={() => select((active + 1) % chapters.length)}
              aria-label="Next discovery chapter"
            >
              <span>Scroll down</span>
            </button>
          </>
        ) : (
          <>
            <header className="press-view-heading">
              <div>
                <span className="press-micro">
                  DISCOVER / TAKE A DIFFERENT TURN
                </span>
                <h1>
                  OFF THE
                  <br />
                  BEATEN TRACK.
                </h1>
              </div>
              <span className="press-big-index">
                {String(active + 1).padStart(2, "0")}
              </span>
            </header>
            <div className="press-bin-heading">
              <div>
                <h2>{selections[active].title}</h2>
                <p>{selections[active].subtitle}</p>
                {refreshControl}
              </div>
              <div className="press-bin-controls">
                <button
                  className="press-icon"
                  aria-label="Previous discovery chapter"
                  onClick={() => select((active + 3) % chapters.length)}
                >
                  <ArrowLeft size={20} />
                </button>
                <select
                  aria-label="Discovery selection"
                  value={active}
                  onChange={(e) => select(Number(e.target.value))}
                >
                  {chapters.map((title, i) => (
                    <option key={title} value={i}>
                      {i + 1} / 4 · {title}
                    </option>
                  ))}
                </select>
                <button
                  className="press-icon"
                  aria-label="Next discovery chapter"
                  onClick={() => select((active + 1) % chapters.length)}
                >
                  <ArrowRight size={20} />
                </button>
              </div>
            </div>
            <div className="chapter-stage">{content}</div>
            <p className="press-wall-note">
              Browse sideways. Drag a sleeve wall, or use its arrows.
            </p>
          </>
        )}
      </div>
    </PreviewContext.Provider>
  );
});
