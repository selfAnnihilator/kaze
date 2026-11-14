import { Fragment, memo } from "react";
import { shallowEqual } from "../../shallowEqual";
import type { AppViewsContext } from "./appViews/context";
import { CollectionPane } from "./appViews/CollectionPane";
import { LIBRARY_VIEWS } from "./appViews/libraryViews";
import { OTHER_VIEWS } from "./appViews/otherViews";
import { LyricsView } from "./appViews/lazyViews";

export type { AppViewsContext } from "./appViews/context";

const VIEWS = { ...LIBRARY_VIEWS, ...OTHER_VIEWS };

/**
 * The main content area: lyrics, an open collection, or the current route's view.
 * A route change remounts its view (keyed fragment), as the old inline switch did.
 * App builds `c` afresh on every render, so it re-renders only when a value in
 * it changed. Full screen covers it, so its lyrics are not mounted then (the
 * full-screen view has its own).
 */
export const AppViews = memo(function AppViews({ c }: { c: AppViewsContext }) {
  if (c.isLyricsActive && c.isFullscreen) return null;
  if (c.isLyricsActive) {
    return (
      <div className="lyrics-stage">
        <LyricsView
          trackId={c.activePlayingTrackId}
          artist={c.activePlayingArtist}
          title={c.activePlayingTitle}
          durationSecs={c.activePlayingDuration}
          onSeek={c.handleUnifiedSeek}
          isFullScreen={false}
        />
      </div>
    );
  }
  if (c.activeCollection) return <CollectionPane c={c} collection={c.activeCollection} />;
  return <Fragment key={c.currentView}>{VIEWS[c.currentView]?.(c)}</Fragment>;
}, (previous, next) => shallowEqual(previous.c, next.c));
