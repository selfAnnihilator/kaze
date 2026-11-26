import { useCallback, useEffect, useMemo, useState } from "react";
import type { SetStateAction } from "react";
import { executeQuery } from "../../services/api";
import { recommendationPreferences } from "../../services/recommendationPreferences";
import { attachLocalSearchMatches } from "../../localTrackMatch";
import { viewDestination, type CatalogPlaylist, type NavigationDestination } from "../../navigationHistory";
import type { DiscoveryRecommendation, Track } from "../../types";

/** Debounce before an online search runs (performance guardrail: 150–250 ms). */
const SEARCH_DEBOUNCE_MS = 200;

type Notify = (type: "success" | "error" | "info" | "warning", title: string, message: string) => void;

/** Keeps results that match the query, or its words, in title, artist or album. */
export function relevantSearchResults(query: string, incoming: DiscoveryRecommendation[]): DiscoveryRecommendation[] {
  const qLower = query.toLowerCase();
  const qTokens = qLower.split(/\s+/).filter((t) => t.length > 1);
  const relevant = incoming.filter((r) => {
    const titleLower = r.title.toLowerCase();
    const artistLower = r.artist.toLowerCase();
    const albumLower = r.album ? r.album.toLowerCase() : "";
    if (titleLower.includes(qLower) || artistLower.includes(qLower) || albumLower.includes(qLower)) return true;
    return qTokens.some((tok) => titleLower.includes(tok) || artistLower.includes(tok));
  });
  return relevant.length > 0 ? relevant : incoming;
}

async function searchOnline(q: string): Promise<DiscoveryRecommendation[]> {
  const res = await executeQuery({ query: "SearchOnlineMusic", payload: { query: q, limit: 35, provider: "jiosaavn" } });
  return relevantSearchResults(q, res && Array.isArray(res.data) ? res.data : []);
}

const searchFailure = (err: unknown) => (typeof err === "string" ? err : "Couldn’t reach online music. Please try again.");

interface SearchRun {
  cancelled: () => boolean;
  show: (results: DiscoveryRecommendation[], playlists?: CatalogPlaylist[]) => void;
  mobile: boolean;
  notify: Notify;
  done: () => void;
}

/** One online search; a cancelled run only clears or keeps the entry it belongs to. */
async function runSearch(q: string, run: SearchRun) {
  try {
    const result = run.mobile ? await executeQuery({ query: "SearchMobileCatalog", payload: { query: q, limit: 35 } }) : undefined;
    const songs = result ? (result.data?.songs ?? []) : await searchOnline(q);
    if (!run.cancelled()) void recommendationPreferences.refresh().catch(console.error);
    run.show(songs, result?.data?.playlists);
  } catch (err) {
    console.error("Global online music search failed:", err);
    if (!run.cancelled()) run.notify("error", "Online search unavailable", searchFailure(err));
    run.show([]);
  } finally {
    if (!run.cancelled()) run.done();
  }
}

/** Runs the destination's online search after a short debounce; results go into the entry. */
function useDebouncedOnlineSearch({ destination, updateDestination, addAppNotification, revision, mobile }: {
  destination: NavigationDestination;
  updateDestination: (update: (current: NavigationDestination) => NavigationDestination) => void;
  addAppNotification: Notify;
  /** Bumped to re-run the same query. */
  revision: number;
  mobile: boolean;
}) {
  const [isSearching, setIsSearching] = useState(false);
  const q = destination.searchQuery;
  const active = !!q && destination.view === "discovery" && !destination.collection && !destination.lyrics;
  useEffect(() => {
    if (!active) {
      setIsSearching(false);
      return;
    }
    let cancelled = false;
    setIsSearching(true);
    const show: SearchRun["show"] = (results, catalogPlaylists) => updateDestination((current) =>
      !cancelled && current.view === "discovery" && current.searchQuery === q ? { ...current, searchResults: results, catalogPlaylists } : current);
    const run: SearchRun = { cancelled: () => cancelled, show, mobile, notify: addAppNotification, done: () => setIsSearching(false) };
    const timer = setTimeout(() => void runSearch(q, run), SEARCH_DEBOUNCE_MS);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [active, q, revision, updateDestination, addAppNotification, mobile]);
  return isSearching;
}

/**
 * The persistent top search bar: the typed query, the debounced online search
 * (results live in the navigation entry), clear and the global refresh.
 */
export function useGlobalSearch({ destination, navigate, updateDestination, tracks, addAppNotification, refreshAll, mobile }: {
  destination: NavigationDestination;
  navigate: (destination: NavigationDestination) => void;
  updateDestination: (update: (current: NavigationDestination) => NavigationDestination) => void;
  tracks: Track[];
  addAppNotification: Notify;
  /** Reloads discovery, home and library after a manual refresh. */
  refreshAll: () => Promise<unknown>;
  mobile?: boolean;
}) {
  const [globalSearchQuery, setGlobalSearchQuery] = useState("");
  const [searchRefreshRevision, setSearchRefreshRevision] = useState(0);
  const [isGlobalRefreshing, setIsGlobalRefreshing] = useState(false);
  const globalSearchResults = destination.searchResults;

  const setGlobalSearchResults = useCallback((results: SetStateAction<DiscoveryRecommendation[] | null>) => {
    updateDestination((current) => ({ ...current, searchResults: typeof results === "function" ? results(current.searchResults) : results }));
  }, [updateDestination]);
  useEffect(() => { setGlobalSearchQuery(destination.searchQuery); }, [destination.searchQuery]);
  const currentSearchResults = useMemo(
    () => attachLocalSearchMatches(globalSearchResults, tracks),
    [globalSearchResults, tracks],
  );

  // Metadata-only legacy wishlist entries open explicit search, never select a recording silently.
  const handleInitiateDirectDownloadSearch = useCallback((artist: string, title: string) => {
    navigate({ ...viewDestination("discovery"), searchQuery: `${title} ${artist}`, searchResults: [] });
    setSearchRefreshRevision((revision) => revision + 1);
  }, [navigate]);

  const handleGlobalOnlineSearch = useCallback(async (queryOverride?: string) => {
    const q = (typeof queryOverride === "string" ? queryOverride : globalSearchQuery).trim();
    navigate({ ...viewDestination("discovery"), searchQuery: q, searchResults: q ? [] : null });
    setSearchRefreshRevision((revision) => revision + 1);
  }, [globalSearchQuery, navigate]);

  const isGlobalSearching = useDebouncedOnlineSearch({ destination, updateDestination, addAppNotification, revision: searchRefreshRevision, mobile: !!mobile });

  const handleGlobalClearSearch = useCallback(() => navigate(viewDestination("discovery")), [navigate]);

  const handleGlobalRefresh = useCallback(async () => {
    setIsGlobalRefreshing(true);
    try {
      if (globalSearchQuery.trim()) await handleGlobalOnlineSearch(globalSearchQuery.trim());
      await refreshAll();
    } finally {
      setIsGlobalRefreshing(false);
    }
  }, [globalSearchQuery, handleGlobalOnlineSearch, refreshAll]);

  return {
    globalSearchQuery, setGlobalSearchQuery, isGlobalSearching, isGlobalRefreshing,
    currentSearchResults, setGlobalSearchResults,
    handleInitiateDirectDownloadSearch, handleGlobalOnlineSearch, handleGlobalClearSearch, handleGlobalRefresh,
  };
}
