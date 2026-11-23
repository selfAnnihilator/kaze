import { useMemo, useState, type ReactElement } from "react";
import { Play, Search, X } from "lucide-react";
import { WindowedList } from "../../WindowedList";
import { playlistCollection } from "../playlistCollection";
import { Sleeve } from "../Sleeve";
import { ForYouRow } from "./DiscoverChapters";
import type { DiscoveryRecommendation, Playlist } from "../../../types";
import type { MobileScreenProps } from "../types";

type Ctx = MobileScreenProps["c"];
type Tab = "all" | "songs" | "artists" | "albums" | "playlists";
const TABS: Array<{ id: Tab; label: string }> = [
  { id: "all", label: "ALL" }, { id: "songs", label: "SONGS" }, { id: "artists", label: "ARTISTS" },
  { id: "albums", label: "ALBUMS" }, { id: "playlists", label: "PLAYLISTS" },
];
const ROW = 67;
const key = (s: string) => s.toLowerCase().trim();

/** Artists and albums found among the songs, and your playlists that match (same as the PC). */
function useGroups(results: DiscoveryRecommendation[], playlists: Playlist[], query: string) {
  return useMemo(() => {
    const artists = new Map<string, { name: string; cover?: string; count: number }>();
    const albums = new Map<string, { name: string; artist: string; cover?: string }>();
    for (const r of results) {
      for (const name of r.artist.split(/\s*,\s*|\s+&\s+|\s+feat\.?\s+/i).filter(Boolean)) {
        const a = artists.get(key(name));
        if (a) a.count += 1; else artists.set(key(name), { name, cover: r.cover_art_url, count: 1 });
      }
      if (r.album?.trim() && !albums.has(`${key(r.artist)}:${key(r.album)}`)) albums.set(`${key(r.artist)}:${key(r.album)}`, { name: r.album, artist: r.artist, cover: r.cover_art_url });
    }
    const q = key(query);
    const lists = q ? playlists.filter((p) => key(p.name).includes(q)) : [];
    return { artists: [...artists.values()].sort((x, y) => y.count - x.count), albums: [...albums.values()], lists };
  }, [results, playlists, query]);
}

function TopResult({ rec, c, onClear }: { rec: DiscoveryRecommendation; c: Ctx; onClear: () => void }) {
  return (
    <div className="m-top-result">
      <div className="m-top-result__head">
        <span className="m-kicker">TOP RESULT</span>
        <button type="button" className="m-top-result__clear" onClick={onClear}><X size={12} strokeWidth={2} /><span>CLEAR SEARCH</span></button>
      </div>
      <div className="m-top-result__card">
        <Sleeve size="toprow" src={rec.cover_art_url} trackId={rec.matched_local_track_id} title={rec.title} />
        <span className="m-top-result__text">
          <strong>{rec.title}</strong>
          <span className="m-kicker m-kicker--muted">SONG · {rec.artist.toUpperCase()}</span>
          {rec.matched_local_track_id && <span className="m-top-result__tag">IN YOUR LIBRARY</span>}
        </span>
        <button type="button" className="m-top-result__play" onClick={() => void c.handlePlayOnlineTrack(rec)} aria-label={`Play ${rec.title}`}><Play size={20} strokeWidth={2} /></button>
      </div>
      <span className="m-kicker m-kicker--ink">SONGS</span>
    </div>
  );
}

function ArtistRows({ artists, onPick }: { artists: Groups["artists"]; onPick: (q: string) => void }) {
  return <>{artists.map((a) => (
    <button key={a.name} type="button" className="m-search-artist" onClick={() => onPick(a.name)}>
      <Sleeve size="edge" src={a.cover} title={a.name} />
      <span className="m-search-artist__text"><strong>{a.name}</strong><span className="m-kicker m-kicker--muted">{a.count} {a.count === 1 ? "SONG" : "SONGS"} FOUND</span></span>
      <Search size={18} strokeWidth={2} />
    </button>
  ))}</>;
}

function AlbumCards({ albums, onPick }: { albums: Groups["albums"]; onPick: (q: string) => void }) {
  return (
    <div className="m-search-albums">
      {albums.map((a) => (
        <button key={`${a.artist}:${a.name}`} type="button" className="m-search-album" onClick={() => onPick(`${a.artist} ${a.name}`)}>
          <Sleeve size="albumcard" src={a.cover} title={a.name} />
          <strong>{a.name}</strong><span className="m-kicker m-kicker--muted">{a.artist.toUpperCase()}</span>
        </button>
      ))}
    </div>
  );
}

type Groups = ReturnType<typeof useGroups>;
interface TabProps { c: Ctx; results: DiscoveryRecommendation[]; query: string; g: Groups; onSearch: (q: string) => void; onClear: () => void }

const NoMatch = ({ query }: { query: string }) => <p className="m-empty-msg">Nothing found for “{query}”. Try another spelling or the artist’s name.</p>;

function ArtistsTab({ g, query, onSearch }: TabProps) {
  if (!g.artists.length) return <NoMatch query={query} />;
  return (
    <>
      <ArtistRows artists={g.artists} onPick={onSearch} />
      {g.albums.length > 0 && <><span className="m-kicker m-kicker--ink m-search-label">ALBUMS</span><AlbumCards albums={g.albums} onPick={onSearch} /></>}
    </>
  );
}

const AlbumsTab = ({ g, query, onSearch }: TabProps) => (g.albums.length ? <AlbumCards albums={g.albums} onPick={onSearch} /> : <NoMatch query={query} />);

function PlaylistsTab({ c, g, query }: TabProps) {
  if (!g.lists.length) return <p className="m-empty-msg">None of your playlists match “{query}”.</p>;
  return <>{g.lists.map((p) => (
    <button key={p.id} type="button" className="m-search-artist" onClick={() => c.handleOpenCollection(playlistCollection(p))}>
      <Sleeve size="edge" src={p.cover_art_url} trackId={p.first_track_id} title={p.name} />
      <span className="m-search-artist__text"><strong>{p.name}</strong><span className="m-kicker m-kicker--muted">{p.track_count ?? 0} TRACKS</span></span>
    </button>
  ))}</>;
}

function SongsTab({ c, results, query }: TabProps) {
  if (!results.length) return <NoMatch query={query} />;
  return <WindowedList items={results} rowHeight={ROW} label="Songs" fill renderItem={(rec: DiscoveryRecommendation, i: number) => <ForYouRow key={rec.external_track_id || i} rec={rec} c={c} />} />;
}

function AllTab({ c, results, query, onClear }: TabProps) {
  if (!results.length) return <NoMatch query={query} />;
  return <><TopResult rec={results[0]} c={c} onClear={onClear} />{results.slice(1, 8).map((rec, i) => <ForYouRow key={rec.external_track_id || i} rec={rec} c={c} />)}</>;
}

const BODIES: Record<Tab, (p: TabProps) => ReactElement> = { all: AllTab, songs: SongsTab, artists: ArtistsTab, albums: AlbumsTab, playlists: PlaylistsTab };

/** Search results (Figma C4/C5): All, Songs, Artists, Albums and Playlists. */
export function SearchResults({ c, results, query, onSearch, onClear }: {
  c: Ctx; results: DiscoveryRecommendation[]; query: string; onSearch: (q: string) => void; onClear: () => void;
}) {
  const [tab, setTab] = useState<Tab>("all");
  const g = useGroups(results, c.playlists, query);
  const Body = BODIES[tab];
  return (
    <div className="m-discover-results">
      <div className="m-library-tabs m-search-tabs" role="tablist" aria-label="Search results">
        {TABS.map((t) => (
          <button key={t.id} type="button" role="tab" aria-selected={tab === t.id} className={`m-library-tab ${tab === t.id ? "is-active" : ""}`} onClick={() => setTab(t.id)}>
            <span className="m-library-tab__content"><span className="m-library-tab__label">{t.label}</span></span>
          </button>
        ))}
      </div>
      <div className={`m-search-body ${tab === "songs" ? "is-list" : ""}`}><Body c={c} results={results} query={query} g={g} onSearch={onSearch} onClear={onClear} /></div>
    </div>
  );
}
