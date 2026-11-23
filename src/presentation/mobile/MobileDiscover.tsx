import { memo, useEffect, useRef, useState } from "react";
import { ArrowRight, Search, X } from "lucide-react";
import { useDebouncedValue } from "../useDebouncedValue";
import { MobileTopBar } from "./MobileTopBar";
import { useMobileNav } from "./mobileNav";
import { DiscoverChapters } from "./discover/DiscoverChapters";
import { Offline, useOnline } from "./discover/Offline";
import { SearchResults } from "./discover/SearchResults";
import type { MobileScreenProps } from "./types";

/** Search row: queues an online search 200ms after typing stops; the top bar's Search focuses it. */
function useDiscoverSearch(c: MobileScreenProps["c"]) {
  const [query, setQuery] = useState(c.globalSearchQuery);
  const debounced = useDebouncedValue(query);
  const { searchNonce } = useMobileNav();
  const ref = useRef<HTMLInputElement>(null);
  useEffect(() => { if (searchNonce) ref.current?.focus(); }, [searchNonce]);
  const { globalSearchQuery, setGlobalSearchQuery, handleGlobalOnlineSearch, handleGlobalClearSearch } = c;
  useEffect(() => {
    if (debounced === globalSearchQuery) return;
    setGlobalSearchQuery(debounced);
    if (debounced.trim()) void handleGlobalOnlineSearch(debounced);
    else handleGlobalClearSearch();
  }, [debounced, globalSearchQuery, setGlobalSearchQuery, handleGlobalOnlineSearch, handleGlobalClearSearch]);
  const submit = () => { if (query.trim()) void handleGlobalOnlineSearch(query); };
  /** Clearing skips the debounce, so leaving the tab right after can't bring the old search back. */
  const change = (next: string) => {
    setQuery(next);
    if (!next.trim() && globalSearchQuery) { setGlobalSearchQuery(""); handleGlobalClearSearch(); }
  };
  return { query, setQuery: change, ref, submit };
}

function SearchRow({ s }: { s: ReturnType<typeof useDiscoverSearch> }) {
  return (
    <div className="m-discover-search">
      <label className={`m-field ${s.query ? "is-filled" : ""}`}>
        <Search size={18} strokeWidth={2} className="m-field__icon" />
        <input ref={s.ref} type="search" enterKeyHint="search" className="m-field__input" placeholder="Search online music…" value={s.query}
          onChange={(e) => s.setQuery(e.target.value)} aria-label="Search online music" />
        {s.query && (
          <button type="button" className="m-discover-search__clear" onClick={() => s.setQuery("")} aria-label="Clear search">
            <X size={18} strokeWidth={2} />
          </button>
        )}
      </label>
      <button type="button" className="m-discover-search__go" onClick={s.submit} aria-label="Search">
        <ArrowRight size={20} strokeWidth={2} />
      </button>
    </div>
  );
}

function Hero() {
  return (
    <div className="m-discover-hero">
      <span className="m-kicker">02 // DISCOVER MUSIC</span>
      <h1 className="m-display m-discover-hero__title">A DIFFERENT KIND OF FEELING.</h1>
    </div>
  );
}

/**
 * Discover (Figma C1 Trending and C2 For You): online search, the hero line,
 * then one chapter at a time with refresh and previous/next chapter controls.
 */
export const MobileDiscover = memo(function MobileDiscover({ c }: MobileScreenProps) {
  const search = useDiscoverSearch(c);
  const results = c.currentSearchResults;
  const online = useOnline();
  return (
    <div className="m-screen__container">
      <MobileTopBar num="02" name={results ? "SEARCH" : "DISCOVER"} />
      <div className="m-screen__content">
        {(online || results) && <SearchRow s={search} />}
        {results ? <SearchResults c={c} results={results} query={search.query} onSearch={search.setQuery} onClear={() => search.setQuery("")} />
          : online ? <><Hero /><DiscoverChapters c={c} /></> : <Offline />}
      </div>
    </div>
  );
});
