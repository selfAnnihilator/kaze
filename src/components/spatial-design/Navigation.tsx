import { memo, useLayoutEffect, useRef, useState } from "react";
import {
  ArrowLeft,
  Disc3,
  Home,
  Library,
  ListMusic,
  Radio,
} from "lucide-react";
import { standardHref } from "./data";

export type Destination = {
  page:
    "home" | "discover" | "library" | "now-playing" | "queue" | "collection";
  id?: string;
  playlist?: boolean;
};

export const NAV = [
  { id: "home", label: "Home", icon: Home },
  { id: "discover", label: "Discover", icon: Radio },
  { id: "library", label: "Library", icon: Library },
  { id: "now-playing", label: "Now Playing", icon: Disc3 },
  { id: "queue", label: "Queue", icon: ListMusic },
] as const;

const MIN_WIDTH = 56;
const MAX_WIDTH = 160;
const STORAGE_KEY = "kaze_spatial_rail_width";
const clamp = (value: number) =>
  Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, value));

export const Navigation = memo(function Navigation({
  page,
  onNavigate,
}: {
  page: string;
  onNavigate: (destination: Destination) => void;
}) {
  const rail = useRef<HTMLElement>(null);
  const gesture = useRef({ startX: 0, startWidth: 72, width: 72 });
  const [width, setWidth] = useState(() => {
    try {
      const saved = Number(localStorage.getItem(STORAGE_KEY));
      return saved > 0 ? clamp(saved) : 72;
    } catch {
      return 72;
    }
  });
  useLayoutEffect(() => {
    rail.current
      ?.closest<HTMLElement>(".spatial-kaze")
      ?.style.setProperty("--sp-rail-width", `${width}px`);
    gesture.current.width = width;
  }, [width]);
  const commit = (next: number) => {
    const value = clamp(next);
    setWidth(value);
    try {
      localStorage.setItem(STORAGE_KEY, String(value));
    } catch {
      /* Optional preview preference. */
    }
  };
  return (
    <aside className="sp-rail" ref={rail}>
      <button
        className="sp-logo"
        onClick={() => onNavigate({ page: "home" })}
        aria-label="Kaze design home"
        title="Kaze home"
      >
        <img src="/kaze-icon.png" alt="" width="34" height="34" />
      </button>
      <nav aria-label="Music navigation">
        {NAV.map((item) => (
          <button
            key={item.id}
            aria-label={item.label}
            title={item.label}
            aria-current={
              page === item.id ||
              (page === "collection" && item.id === "library")
                ? "page"
                : undefined
            }
            onClick={() => onNavigate({ page: item.id })}
          >
            <item.icon size={22} strokeWidth={1.6} aria-hidden="true" />
          </button>
        ))}
      </nav>
      <div className="sp-rail-bottom">
        <a href={standardHref()} title="Current Kaze" aria-label="Current Kaze">
          <ArrowLeft size={19} aria-hidden="true" />
        </a>
      </div>
      <div
        className="sp-rail-resizer"
        role="separator"
        aria-label="Resize navigation"
        aria-orientation="vertical"
        aria-valuemin={MIN_WIDTH}
        aria-valuemax={MAX_WIDTH}
        aria-valuenow={width}
        tabIndex={0}
        onPointerDown={(event) => {
          if (event.button !== 0) return;
          event.preventDefault();
          event.currentTarget.setPointerCapture(event.pointerId);
          gesture.current = { startX: event.clientX, startWidth: width, width };
          if (rail.current) rail.current.dataset.resizing = "true";
        }}
        onPointerMove={(event) => {
          if (!event.currentTarget.hasPointerCapture(event.pointerId)) return;
          const next = clamp(
            gesture.current.startWidth + event.clientX - gesture.current.startX,
          );
          gesture.current.width = next;
          // Resize the grid directly; dragging never drives shell React state.
          rail.current
            ?.closest<HTMLElement>(".spatial-kaze")
            ?.style.setProperty("--sp-rail-width", `${next}px`);
          event.currentTarget.setAttribute("aria-valuenow", String(next));
        }}
        onPointerUp={(event) => {
          if (!event.currentTarget.hasPointerCapture(event.pointerId)) return;
          event.currentTarget.releasePointerCapture(event.pointerId);
          if (rail.current) delete rail.current.dataset.resizing;
          commit(gesture.current.width);
        }}
        onPointerCancel={(event) => {
          if (event.currentTarget.hasPointerCapture(event.pointerId))
            event.currentTarget.releasePointerCapture(event.pointerId);
          if (rail.current) delete rail.current.dataset.resizing;
          commit(gesture.current.width);
        }}
        onKeyDown={(event) => {
          const next =
            event.key === "ArrowLeft"
              ? width - 8
              : event.key === "ArrowRight"
                ? width + 8
                : event.key === "Home"
                  ? MIN_WIDTH
                  : event.key === "End"
                    ? MAX_WIDTH
                    : null;
          if (next !== null) {
            event.preventDefault();
            commit(next);
          }
        }}
      />
    </aside>
  );
});
