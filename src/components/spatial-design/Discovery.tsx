import { memo, useCallback, useEffect, useRef, useState } from "react";
import { ArrowUpRight, Play } from "lucide-react";
import { CHAPTERS, SpatialTrack } from "./data";
import { Artwork, MusicStrip } from "./Primitives";

export const Discovery = memo(function Discovery({
  onPlay,
  onAlbum,
  onPlaySelection,
}: {
  onPlay: (id: string) => void;
  onAlbum: (id: string) => void;
  onPlaySelection: (tracks: SpatialTrack[]) => void;
}) {
  const root = useRef<HTMLDivElement>(null);
  const wheel = useRef({ last: 0, total: 0, consumed: false });
  const [selection, setSelection] = useState({
    chapter: 0,
    trackId: CHAPTERS[0].tracks[0].id,
  });
  const chapter = CHAPTERS[selection.chapter];
  const preview =
    chapter.tracks.find((track) => track.id === selection.trackId) ??
    chapter.tracks[0];
  const changeChapter = useCallback(
    (next: number) =>
      setSelection((current) => {
        const index = Math.max(0, Math.min(CHAPTERS.length - 1, next));
        return index === current.chapter
          ? current
          : { chapter: index, trackId: CHAPTERS[index].tracks[0].id };
      }),
    [],
  );
  const previewTrack = useCallback(
    (id: string) =>
      setSelection((current) =>
        current.trackId === id ? current : { ...current, trackId: id },
      ),
    [],
  );

  useEffect(() => {
    const element = root.current;
    if (!element) return;
    const selectWithWheel = (event: WheelEvent) => {
      if (event.ctrlKey || Math.abs(event.deltaX) >= Math.abs(event.deltaY))
        return;
      // Horizontal browsing remains native and does not change chapters.
      if (event.target instanceof Element && event.target.closest(".sp-strip"))
        return;
      event.preventDefault();
      const now = performance.now(),
        gesture = wheel.current;
      if (now - gesture.last > 180) {
        gesture.total = 0;
        gesture.consumed = false;
      }
      gesture.last = now;
      gesture.total +=
        event.deltaY *
        (event.deltaMode === 1
          ? 16
          : event.deltaMode === 2
            ? element.clientHeight
            : 1);
      if (gesture.consumed || Math.abs(gesture.total) < 40) return;
      gesture.consumed = true;
      const direction = Math.sign(gesture.total);
      setSelection((current) => {
        const index = Math.max(
          0,
          Math.min(CHAPTERS.length - 1, current.chapter + direction),
        );
        return index === current.chapter
          ? current
          : { chapter: index, trackId: CHAPTERS[index].tracks[0].id };
      });
    };
    element.addEventListener("wheel", selectWithWheel, { passive: false });
    return () => element.removeEventListener("wheel", selectWithWheel);
  }, []);

  return (
    <div
      className="sp-discovery-fixed"
      ref={root}
      role="region"
      aria-label="Discover music"
      tabIndex={0}
      onKeyDown={(event) => {
        if (
          event.target instanceof HTMLInputElement ||
          event.target instanceof HTMLTextAreaElement
        )
          return;
        if (event.key === "PageDown" || event.key === "PageUp") {
          event.preventDefault();
          changeChapter(
            selection.chapter + (event.key === "PageDown" ? 1 : -1),
          );
          root.current?.focus({ preventScroll: true });
        }
      }}
    >
      <aside
        className="sp-discovery-artwork"
        aria-label="Album artwork previews"
      >
        <button
          className="sp-discovery-cover"
          onClick={() => onAlbum(preview.albumId)}
          aria-label={`Open ${preview.album} by ${preview.artist}`}
        >
          <Artwork
            key={preview.artwork}
            src={preview.artwork}
            title={`${preview.album} by ${preview.artist}`}
            eager
          />
        </button>
        <div className="sp-discovery-caption">
          <h2 title={preview.title}>{preview.title}</h2>
          <p title={preview.artist}>{preview.artist}</p>
        </div>
      </aside>
      <section className="sp-discovery-panel" aria-label="Discovery selection">
        <div className="sp-discovery-copy" key={chapter.title}>
          <span className="sp-kicker">0{selection.chapter + 1} / 04</span>
          <h1>{chapter.title}</h1>
          <p>{chapter.subtitle}</p>
          <button
            className="sp-text-action"
            onClick={() => onPlaySelection(chapter.tracks)}
          >
            <Play size={14} fill="currentColor" />
            Play this selection
          </button>
        </div>
        <div
          className="sp-discovery-preview"
          aria-live="polite"
          aria-atomic="true"
        >
          <div>
            <button
              className="sp-preview-album"
              onClick={() => onAlbum(preview.albumId)}
            >
              {preview.album}
              <ArrowUpRight size={16} />
            </button>
            <p>{preview.artist}</p>
            <span>{preview.title}</span>
          </div>
        </div>
        <MusicStrip
          key={selection.chapter}
          label={`Explore ${chapter.title.toLocaleLowerCase()}`}
          tracks={chapter.tracks}
          onPlay={onPlay}
          onAlbum={onAlbum}
          onPreview={previewTrack}
        />
      </section>
      <button
        className="sp-discovery-scroll-cue"
        onClick={() => changeChapter((selection.chapter + 1) % CHAPTERS.length)}
        aria-label={
          selection.chapter === CHAPTERS.length - 1
            ? "Back to first discovery chapter"
            : "Next discovery chapter"
        }
      >
        <span>Scroll down</span>
      </button>
    </div>
  );
});
