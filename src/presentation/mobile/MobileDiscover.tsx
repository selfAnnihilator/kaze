import { memo, useEffect, useState } from "react";
import { ListPlus } from "lucide-react";
import { WindowedList } from "../WindowedList";
import { useDebouncedValue } from "../useDebouncedValue";
import { MobileChip, MobileChipRow, MobileField, MobileIconButton, EmptyNote } from "./primitives";
import { TrackRow } from "./TrackRow";
import { joinMeta } from "./format";
import type { DiscoveryRecommendation } from "../../types";
import type { MobileScreenProps } from "./types";

type Feed = "for_you" | "trending";
const ROW_HEIGHT = 72;

/** Discover: search online, or browse recommendations and trending. */
export const MobileDiscover = memo(function MobileDiscover({ c }: MobileScreenProps) {
  const [feed, setFeed] = useState<Feed>("for_you");
  const [query, setQuery] = useState(c.globalSearchQuery);
  const debounced = useDebouncedValue(query);
  const { globalSearchQuery, setGlobalSearchQuery, handleGlobalOnlineSearch, handleGlobalClearSearch } = c;
  useEffect(() => {
    if (debounced === globalSearchQuery) return;
    setGlobalSearchQuery(debounced);
    if (debounced.trim()) void handleGlobalOnlineSearch(debounced);
    else handleGlobalClearSearch();
  }, [debounced, globalSearchQuery, setGlobalSearchQuery, handleGlobalOnlineSearch, handleGlobalClearSearch]);
  const items = c.currentSearchResults ?? (feed === "for_you" ? c.discoveryRecs : c.worldTrending);
  const renderRec = (rec: DiscoveryRecommendation) => (
    <TrackRow title={rec.title} subtitle={joinMeta(rec.artist, rec.album)} durationSecs={rec.duration_secs}
      src={rec.cover_art_url} onPress={() => void c.handlePlayOnlineTrack(rec)}
      trailing={<MobileIconButton label={`Queue ${rec.title}`}
        onClick={() => void c.handleEnqueueRecommendation(rec)}><ListPlus size={18} /></MobileIconButton>} />
  );
  return (
    <div className="m-screen__body m-screen__body--fill">
      <MobileField label="Search" type="search" value={query} placeholder="Songs, artists, albums"
        onChange={(e) => setQuery(e.target.value)} />
      {!c.currentSearchResults && (
        <MobileChipRow label="Feed">
          <MobileChip selected={feed === "for_you"} onSelect={() => setFeed("for_you")}>For you</MobileChip>
          <MobileChip selected={feed === "trending"} onSelect={() => setFeed("trending")}>Trending</MobileChip>
        </MobileChipRow>
      )}
      {items.length
        ? <WindowedList items={items} renderItem={renderRec} rowHeight={ROW_HEIGHT} label="Discover" fill />
        : <EmptyNote>{c.isGlobalSearching || c.isTrendingLoading ? "Searching…" : "No results."}</EmptyNote>}
    </div>
  );
});
