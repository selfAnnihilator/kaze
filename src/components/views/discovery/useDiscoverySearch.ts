import { useState } from "react";
import type { DiscoveryRecommendation } from "../../../types";
import { executeQuery } from "../../../services/api";

export interface DiscoverySearchProps {
  searchQuery?: string;
  setSearchQuery?: (q: string) => void;
  searchResults?: DiscoveryRecommendation[] | null;
  setSearchResults?: (r: DiscoveryRecommendation[] | null) => void;
  isSearchingOnline?: boolean;
  onSearchOnline?: (queryOverride?: string) => Promise<void> | void;
  onClearSearch?: () => void;
}

/** Results that match the query (whole or any token); all results when none do. */
export function relevantResults(q: string, incoming: DiscoveryRecommendation[]) {
  const qLower = q.toLowerCase();
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

/** Searches online and keeps the results relevant to `q`; empty on failure. */
async function searchOnline(q: string): Promise<DiscoveryRecommendation[]> {
  try {
    const res = await executeQuery({ query: "SearchOnlineMusic", payload: { query: q, limit: 35 } });
    return relevantResults(q, res && Array.isArray(res.data) ? res.data : []);
  } catch (err) {
    console.error("Online music search failed:", err);
    return [];
  }
}

/**
 * Online search for Discovery: uses the parent's state and handlers when given,
 * otherwise keeps its own.
 */
export function useDiscoverySearch({
  searchQuery: propSearchQuery,
  setSearchQuery: propSetSearchQuery,
  searchResults: propSearchResults,
  setSearchResults: propSetSearchResults,
  isSearchingOnline: propIsSearchingOnline,
  onSearchOnline: propOnSearchOnline,
  onClearSearch: propOnClearSearch,
}: DiscoverySearchProps) {
  const [internalSearchQuery, setInternalSearchQuery] = useState("");
  const [internalIsSearching, setInternalIsSearching] = useState(false);
  const [internalSearchResults, setInternalSearchResults] = useState<DiscoveryRecommendation[] | null>(null);

  const searchQuery = propSearchQuery !== undefined ? propSearchQuery : internalSearchQuery;
  const setSearchQuery = propSetSearchQuery || setInternalSearchQuery;
  const searchResults = propSearchResults !== undefined ? propSearchResults : internalSearchResults;
  const setSearchResults = propSetSearchResults || setInternalSearchResults;
  const searching = propIsSearchingOnline ?? internalIsSearching;
  const searchActive = searchResults !== null || searching;

  const handleSearchOnline = async (queryOverride?: string) => {
    if (propOnSearchOnline) {
      await propOnSearchOnline(queryOverride);
      return;
    }
    const q = (typeof queryOverride === "string" ? queryOverride : searchQuery).trim();
    if (!q) {
      setSearchResults(null);
      return;
    }
    setSearchResults([]);
    const ownsSearching = propIsSearchingOnline === undefined;
    if (ownsSearching) setInternalIsSearching(true);
    setSearchResults(await searchOnline(q));
    if (ownsSearching) setInternalIsSearching(false);
  };

  const handleClearSearch = () => {
    if (propOnClearSearch) {
      propOnClearSearch();
      return;
    }
    setSearchQuery("");
    setSearchResults(null);
  };

  return { searchQuery, setSearchQuery, searchResults, searching, searchActive, handleSearchOnline, handleClearSearch };
}
