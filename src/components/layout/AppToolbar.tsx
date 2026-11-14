import { memo } from "react";
import { ArrowLeft, ArrowRight } from "lucide-react";
import { GlobalTopSearchBar } from "./GlobalTopSearchBar";
import { SpatialUtilities, SpatialLibraryHeader } from "../../presentation/SpatialUtilities";
import { BrutalistLibraryHeader } from "../../presentation/BrutalistNavigation";
import { isLibraryView } from "../../hooks/app/useAppNavigation";
import type { ViewType } from "../../viewTypes";

export interface AppToolbarProps {
  style: "spatial" | "brutalist";
  currentView: ViewType;
  /** Title of the open collection, if any. */
  collectionTitle?: string;
  hasCollection: boolean;
  isLyricsActive: boolean;
  isOnline: boolean;
  isArchiveLiked: boolean;
  isArchiveSurface: boolean;
  canGoBack: boolean;
  canGoForward: boolean;
  onBack: () => void;
  onForward: () => void;
  onNavigate: (view: ViewType) => void;
  onLiked?: () => void;
  unread: number;
  search: {
    query: string;
    setQuery: (query: string) => void;
    onSearch: (queryOverride?: string) => void;
    onRefresh: () => void;
    isSearching: boolean;
    isRefreshing: boolean;
  };
  /** Library counts for the headers. */
  counts: { albums: number; tracks: number; archive: number };
}

const VIEW_LABELS: Partial<Record<ViewType, string>> = {
  discovery: "Discover", stats: "Profile", library: "Library", downloads: "Library", albums: "Library", playlists: "Library",
};

function routeLabel(p: AppToolbarProps): string | false {
  if (p.isLyricsActive) return false;
  const label = p.isArchiveLiked ? "Library" : p.collectionTitle;
  return label || VIEW_LABELS[p.currentView] || p.currentView.replace(/_/g, " ");
}

/** Hidden on the profile, with lyrics open, and on Discover while offline. */
function searchHidden(p: AppToolbarProps): boolean {
  const offlineDiscover = !p.hasCollection && p.currentView === "discovery" && !p.isOnline;
  return offlineDiscover || p.isLyricsActive || p.currentView === "stats";
}

/** History arrows, route label, the global search bar and the library headers. */
export const AppToolbar = memo(function AppToolbar(p: AppToolbarProps) {
  const hideSearch = searchHidden(p);
  const spatial = p.style === "spatial";
  const onLibraryPage = !p.hasCollection && !p.isLyricsActive && isLibraryView(p.currentView);
  return (
    <>
      <div className="interface-toolbar">
        <div className="history-controls">
          <button className="player-icon-btn" aria-label="Go back" disabled={!p.canGoBack} onClick={p.onBack}>
            <ArrowLeft size={18} />
          </button>
          <button className="player-icon-btn" aria-label="Go forward" disabled={!p.canGoForward} onClick={p.onForward}>
            <ArrowRight size={18} />
          </button>
        </div>
        <span className="interface-name">{spatial ? "Spatial" : "Brutalist"}</span>
        <span className="interface-route">{routeLabel(p)}</span>
        {hideSearch ? null : (
          <GlobalTopSearchBar
            placeholder={spatial ? "Find your next favourite" : undefined}
            searchQuery={p.search.query}
            setSearchQuery={p.search.setQuery}
            onSearch={p.search.onSearch}
            onRefresh={p.search.onRefresh}
            isSearching={p.search.isSearching}
            isRefreshing={p.search.isRefreshing}
          />
        )}
        {spatial && <SpatialUtilities onNavigate={p.onNavigate} unread={p.unread} />}
      </div>
      {spatial && onLibraryPage && (
        <SpatialLibraryHeader view={p.currentView} albums={p.counts.albums} tracks={p.counts.tracks} onNavigate={p.onNavigate} onLiked={p.onLiked} />
      )}
      {!spatial && p.isArchiveSurface && (
        <BrutalistLibraryHeader view={p.currentView} count={p.counts.archive} likedSelected={p.isArchiveLiked} onNavigate={p.onNavigate} onLiked={p.onLiked} />
      )}
    </>
  );
});
