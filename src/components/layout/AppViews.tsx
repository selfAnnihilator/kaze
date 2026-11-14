import { Fragment } from "react";
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
 */
export function AppViews({ c }: { c: AppViewsContext }) {
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
}
