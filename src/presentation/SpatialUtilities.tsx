import { memo, useEffect, useRef, useState } from "react";
import {
  Bell,
  Bookmark,
  Disc3,
  Download,
  ListMusic,
  SlidersHorizontal,
  Sparkles,
} from "lucide-react";
import type { ViewType } from "../components/Sidebar";
const destinations = [
  ["albums", "Albums", Disc3],
  ["playlists", "Playlists", ListMusic],
  ["smart_mixes", "Smart Mixes", Sparkles],
  ["downloads", "Downloads", Download],
  ["wishlist", "Wishlist", Bookmark],
  ["notifications", "Notifications", Bell],
] as const;
export const SpatialUtilities = memo(function SpatialUtilities({
  onNavigate,
  unread = 0,
}: {
  onNavigate: (view: ViewType) => void;
  unread?: number;
}) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (!open) return;
    root.current
      ?.querySelector<HTMLButtonElement>('[role="menuitem"]')
      ?.focus({ preventScroll: true });
    const outside = (event: PointerEvent) => {
      if (event.target instanceof Node && !root.current?.contains(event.target))
        setOpen(false);
    };
    document.addEventListener("pointerdown", outside);
    return () => document.removeEventListener("pointerdown", outside);
  }, [open]);
  return (
    <div
      className="spatial-utilities"
      ref={root}
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          event.stopPropagation();
          setOpen(false);
          trigger.current?.focus();
          return;
        }
        if (
          !open ||
          !["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)
        )
          return;
        event.preventDefault();
        const items = Array.from(
          root.current?.querySelectorAll<HTMLButtonElement>(
            '[role="menuitem"]',
          ) || [],
        );
        const index = items.indexOf(
          document.activeElement as HTMLButtonElement,
        );
        items[
          event.key === "Home"
            ? 0
            : event.key === "End"
              ? items.length - 1
              : (index + (event.key === "ArrowDown" ? 1 : -1) + items.length) %
                items.length
        ]?.focus();
      }}
    >
      <button
        ref={trigger}
        className="spatial-utilities-trigger"
        aria-label="Library and more"
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen((value) => !value)}
      >
        <SlidersHorizontal size={15} />
        <span>Your space</span>
        {unread > 0 && <i aria-label={`${unread} unread notifications`} />}
      </button>
      {open && (
        <div
          className="spatial-utilities-menu"
          role="menu"
          aria-label="Library and more"
        >
          {destinations.map(([view, title, Icon]) => (
            <button
              key={view}
              role="menuitem"
              aria-label={title}
              onClick={() => {
                setOpen(false);
                onNavigate(view);
              }}
            >
              <Icon size={17} />
              <span>{title}</span>
              {view === "notifications" && unread > 0 && (
                <small>{unread}</small>
              )}
            </button>
          ))}
        </div>
      )}
    </div>
  );
});

export const SpatialLibraryHeader = memo(function SpatialLibraryHeader({
  view,
  albums,
  onNavigate,
  onLiked,
}: {
  view: ViewType;
  albums: number;
  onNavigate: (view: ViewType) => void;
  onLiked?: () => void;
}) {
  return (
    <header className="spatial-library-header">
      <p className="interface-eyebrow">The music you make yours</p>
      <h1>
        Your library<span>.</span>
      </h1>
      <p>{albums} albums. A thousand places to go.</p>
      <div
        className="spatial-library-tabs"
        role="tablist"
        aria-label="Library categories"
      >
        {(
          [
            ["albums", "Albums"],
            ["library", "Tracks"],
            ["playlists", "Playlists"],
          ] as const
        ).map(([id, title]) => (
          <button
            key={id}
            role="tab"
            aria-selected={view === id}
            onClick={() => onNavigate(id)}
          >
            {title}
          </button>
        ))}
        <button
          role="tab"
          aria-selected={false}
          disabled={!onLiked}
          onClick={onLiked}
        >
          Liked
        </button>
      </div>
    </header>
  );
});
