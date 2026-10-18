import { memo } from "react";
import type { ViewType } from "../components/Sidebar";

/** Production destinations in the preview's horizontal catalogue navigation. */
export const BrutalistNavigation = memo(function BrutalistNavigation({
  view,
  onNavigate,
}: {
  view: ViewType;
  onNavigate: (view: ViewType) => void;
}) {
  const selected = ["albums", "library", "playlists", "smart_mixes"].includes(
    view,
  )
    ? "albums"
    : view === "now_playing"
      ? "home"
      : view;
  return (
    <header className="press-header">
      <button
        className="press-wordmark"
        aria-label="Kaze Home"
        onClick={() => onNavigate("home")}
      >
        KAZE<span lang="ja">音楽を、そのまま。</span>
      </button>
      <nav aria-label="Main navigation">
        {(
          [
            ["home", "Home"],
            ["discovery", "Discover Music"],
            ["albums", "Library"],
            ["queue", "Queue"],
          ] as const
        ).map(([id, label], i) => (
          <button
            key={id}
            aria-label={label}
            aria-current={selected === id ? "page" : undefined}
            onClick={() => onNavigate(id)}
          >
            <span>{String(i + 1).padStart(2, "0")}</span>
            {label}
          </button>
        ))}
      </nav>
      <div className="press-edition">
        <strong>BRUTALIST / KAZE</strong>
        <span>YOUR RECORDS. YOUR RULES.</span>
      </div>
    </header>
  );
});

export const BrutalistLibraryHeader = memo(function BrutalistLibraryHeader({
  view,
  count,
  likedSelected = false,
  onNavigate,
  onLiked,
}: {
  view: ViewType;
  count: number;
  likedSelected?: boolean;
  onNavigate: (view: ViewType) => void;
  onLiked?: () => void;
}) {
  return (
    <header className="press-library-heading">
      <div className="press-view-heading">
        <div>
          <span className="press-micro">YOUR MUSIC / ALL IN ONE PLACE</span>
          <h1>
            THE
            <br />
            ARCHIVE.
          </h1>
        </div>
        <span className="press-big-index">
          {String(count).padStart(2, "0")}
        </span>
      </div>
      <div
        className="press-tabs"
        role="tablist"
        aria-label="Library categories"
        onKeyDown={(event) => {
          if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key))
            return;
          const tabs = Array.from(
            event.currentTarget.querySelectorAll<HTMLButtonElement>(
              '[role="tab"]:not(:disabled)',
            ),
          );
          const current = tabs.indexOf(
            document.activeElement as HTMLButtonElement,
          );
          const next =
            event.key === "Home"
              ? 0
              : event.key === "End"
                ? tabs.length - 1
                : (current +
                    (event.key === "ArrowRight" ? 1 : -1) +
                    tabs.length) %
                  tabs.length;
          event.preventDefault();
          tabs[next]?.focus();
          tabs[next]?.click();
        }}
      >
        {(
          [
            ["albums", "Albums"],
            ["library", "Tracks"],
            ["playlists", "Playlists"],
          ] as const
        ).map(([id, label]) => (
          <button
            key={id}
            role="tab"
            aria-selected={!likedSelected && view === id}
            onClick={() => onNavigate(id)}
          >
            {label}
          </button>
        ))}
        <button
          role="tab"
          aria-selected={likedSelected}
          onClick={onLiked}
          disabled={!onLiked}
        >
          Liked
        </button>
      </div>
    </header>
  );
});
