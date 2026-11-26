import { isPersonalPlaylist, playlistCollection } from "./playlistCollection";
import { memo, useCallback, useEffect, useMemo, useState } from "react";
import { ArrowUpRight } from "lucide-react";
import { useDebouncedValue } from "../useDebouncedValue";
import { AlbumsPane, PlaylistsPane, TracksPane } from "./LibraryPanes";
import { MobileTopBar } from "./MobileTopBar";
import { useMobileNav } from "./mobileNav";
import type { Playlist } from "../../types";
import type { MobileScreenProps } from "./types";

type LibraryTab = "albums" | "local" | "downloads" | "playlists" | "liked";
const LIKED_NAME = "Liked Songs";
const TABS: ReadonlyArray<{ id: LibraryTab; label: string }> = [
  { id: "albums", label: "ALBUMS" },
  { id: "local", label: "LOCAL" },
  { id: "downloads", label: "DOWNLOADS" },
  { id: "playlists", label: "PLAYLISTS" },
  { id: "liked", label: "LIKED" },
];

/** Mono counts with thousands separators, as in Figma ("2,481"). */
const count = (n: number) => n.toLocaleString("en-US");

function libraryCounts(c: MobileScreenProps["c"], playlistCount: number): Record<LibraryTab, number> {
  return {
    albums: c.albumPage.total || c.albums.length,
    local: c.libraryScoped ? c.tracks.length : c.trackPage.total,
    downloads: c.downloadsPage.total,
    playlists: playlistCount,
    liked: c.likedTrackIds.size,
  };
}

/** Title with the open tab's count, the five-tab strip and the "More" row (Figma D1). */
const LibraryHeader = memo(function LibraryHeader({
  tab, counts, onSelect,
}: { tab: LibraryTab; counts: Record<LibraryTab, number>; onSelect: (id: LibraryTab) => void }) {
  const nav = useMobileNav();
  return (
    <>
      <div className="m-library-hero">
        <span className="m-kicker">03 // LIBRARY</span>
        <div className="m-library-hero__title-row">
          <h1 className="m-display m-library-hero__title">THE ARCHIVE.</h1>
          <span className="m-display m-library-hero__count">{count(counts[tab])}</span>
        </div>
      </div>

      <div className="m-library-tabs" role="tablist" aria-label="Library sections">
        {TABS.map((t) => (
          <button key={t.id} type="button" role="tab" aria-selected={tab === t.id}
            className={`m-library-tab ${tab === t.id ? "is-active" : ""}`} onClick={() => onSelect(t.id)}>
            <span className="m-library-tab__content">
              <span className="m-library-tab__label">{t.label}</span>
              <span className="m-library-tab__count">{count(counts[t.id])}</span>
            </span>
          </button>
        ))}
      </div>

      <div className="m-library-more">
        <span className="m-kicker m-kicker--muted">MORE:</span>
        <button type="button" className="m-library-more__link" onClick={() => nav.openPage("smart_mixes")}>
          <span>SMART MIXES</span><ArrowUpRight size={12} strokeWidth={2} />
        </button>
      </div>
    </>
  );
});

/**
 * Library (Figma D1–D4, "THE ARCHIVE."): header, then the open tab's pane.
 * Liked opens the Liked Songs collection.
 */
export const MobileLibrary = memo(function MobileLibrary({ c }: MobileScreenProps) {
  const { albumFocus, libraryFocus } = useMobileNav();
  useEffect(() => {
    if (libraryFocus) setTab(libraryFocus.tab);
  }, [libraryFocus]);
  const [tab, setTab] = useState<LibraryTab>("albums");
  useEffect(() => {
    if (albumFocus) setTab("albums");
  }, [albumFocus]);
  const [query, setQuery] = useState(c.destination.libraryQuery ?? "");
  const debounced = useDebouncedValue(query);
  const { handleSearchLibrary } = c;

  useEffect(() => {
    if (tab === "local") handleSearchLibrary(debounced);
  }, [debounced, tab, handleSearchLibrary]);

  const playlists = useMemo(() => c.playlists.filter(isPersonalPlaylist), [c.playlists]);
  const { handleOpenCollection } = c;
  const openPlaylist = useCallback((p: Playlist) => handleOpenCollection(playlistCollection(p)), [handleOpenCollection]);
  const select = useCallback((id: LibraryTab) => {
    if (id !== "liked") return setTab(id);
    const liked = c.playlists.find((p) => p.name === LIKED_NAME);
    if (liked) openPlaylist(liked);
  }, [c.playlists, openPlaylist]);
  const trackTab = tab === "local" || tab === "downloads";
  const listTab = trackTab || tab === "playlists";

  return (
    <div className="m-screen__container">
      <MobileTopBar num="03" name="LIBRARY" />
      <div className={`m-screen__content ${listTab ? "m-screen__content--list m-library--fixed" : ""}`}>
        <LibraryHeader tab={tab} counts={libraryCounts(c, playlists.length)} onSelect={select} />
        {tab === "albums" && <AlbumsPane c={c} focusId={albumFocus} />}
        {trackTab && <TracksPane c={c} downloads={tab === "downloads"} query={query} onQuery={setQuery} />}
        {tab === "playlists" && <PlaylistsPane c={c} playlists={playlists} onOpen={openPlaylist} />}
      </div>
    </div>
  );
});
