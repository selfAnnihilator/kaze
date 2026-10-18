import { memo, useLayoutEffect, useRef, useState } from "react";
import { Home, Library, ListMusic, Radio, Settings } from "lucide-react";
import type { SidebarProps, ViewType } from "../components/Sidebar";
import { ProfileNavigation } from "./ProfileNavigation";

const primary = [
  ["home", "Home", Home],
  ["discovery", "Discover Music", Radio],
  ["albums", "Library", Library],
  ["queue", "Queue", ListMusic],
] as const;
const clamp = (width: number) => Math.min(160, Math.max(56, width));

/** The accepted icon rail, with production destinations and leaf-only resizing. */
export const SpatialNavigation = memo(function SpatialNavigation({
  currentView,
  currentUser,
  onSelectView,
}: Pick<
  SidebarProps,
  "currentView" | "currentUser" | "onSelectView" | "unreadNotificationsCount"
>) {
  const rail = useRef<HTMLElement>(null);
  const [width, setWidth] = useState(72);
  const gesture = useRef({ x: 0, start: 72, width: 72 });
  const setGridWidth = (value: number) =>
    rail.current
      ?.closest<HTMLElement>(".kaze-interface")
      ?.style.setProperty("--spatial-rail-width", `${value}px`);
  useLayoutEffect(() => {
    setGridWidth(width);
  }, [width]);
  const navigate = (view: ViewType) => onSelectView(view);
  const button = ([view, label, Icon]: (typeof primary)[number]) => (
    <button
      key={view}
      className={`nav-button${currentView === view || (view === "albums" && ["library", "playlists", "smart_mixes"].includes(currentView)) ? " active" : ""}`}
      aria-current={
        currentView === view ||
        (view === "albums" &&
          ["library", "playlists", "smart_mixes"].includes(currentView))
          ? "page"
          : undefined
      }
      aria-label={label}
      title={label}
      onClick={() => navigate(view)}
    >
      <Icon size={22} strokeWidth={1.6} aria-hidden="true" />
      <span className="spatial-sr-only">{label}</span>
    </button>
  );
  const commit = () => {
    setWidth(gesture.current.width);
    if (rail.current) delete rail.current.dataset.resizing;
  };
  return (
    <aside
      ref={rail}
      className="sidebar spatial-navigation"
      aria-label="Main navigation"
    >
      <button
        className="spatial-logo"
        aria-label="Kaze home"
        title="Kaze home"
        onClick={() => navigate("home")}
      >
        <img src="/kaze-icon.png" alt="" width={30} height={30} />
      </button>
      <div className="spatial-navigation-scroll">
        <nav aria-label="Music navigation">{primary.map(button)}</nav>
      </div>
      <div className="spatial-navigation-footer">
        <button
          className={`nav-button spatial-settings${currentView === "settings" ? " active" : ""}`}
          aria-current={currentView === "settings" ? "page" : undefined}
          aria-label="Settings"
          title="Settings"
          onClick={() => navigate("settings")}
        >
          <Settings size={20} strokeWidth={1.6} />
          <span className="spatial-sr-only">Settings</span>
        </button>
        <ProfileNavigation
          user={currentUser}
          active={currentView === "stats"}
          onSelectView={onSelectView}
        />
      </div>
      <div
        className="spatial-rail-resizer"
        role="separator"
        aria-label="Resize navigation"
        aria-orientation="vertical"
        aria-valuemin={56}
        aria-valuemax={160}
        aria-valuenow={width}
        tabIndex={0}
        onPointerDown={(event) => {
          if (event.button !== 0) return;
          event.preventDefault();
          event.currentTarget.setPointerCapture(event.pointerId);
          gesture.current = { x: event.clientX, start: width, width };
          if (rail.current) rail.current.dataset.resizing = "true";
        }}
        onPointerMove={(event) => {
          if (!event.currentTarget.hasPointerCapture(event.pointerId)) return;
          const next = clamp(
            gesture.current.start + event.clientX - gesture.current.x,
          );
          gesture.current.width = next;
          setGridWidth(next);
          event.currentTarget.setAttribute("aria-valuenow", String(next));
        }}
        onPointerUp={(event) => {
          if (!event.currentTarget.hasPointerCapture(event.pointerId)) return;
          event.currentTarget.releasePointerCapture(event.pointerId);
          commit();
        }}
        onPointerCancel={() => commit()}
        onKeyDown={(event) => {
          const next =
            event.key === "ArrowLeft"
              ? width - 8
              : event.key === "ArrowRight"
                ? width + 8
                : event.key === "Home"
                  ? 56
                  : event.key === "End"
                    ? 160
                    : null;
          if (next === null) return;
          event.preventDefault();
          setWidth(clamp(next));
        }}
      />
    </aside>
  );
});
