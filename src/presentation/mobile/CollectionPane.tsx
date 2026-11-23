import { memo, useMemo, useState } from "react";
import { ArrowLeft, BookmarkCheck, BookmarkPlus, Heart, Pause, Play, Search, Shuffle } from "lucide-react";
import { WindowedList } from "../WindowedList";
import { useDebouncedValue } from "../useDebouncedValue";
import { Sleeve } from "./Sleeve";
import { CollectionRow } from "./CollectionRow";
import { PlaylistOptions } from "./playlists/PlaylistOptions";
import { joinMeta } from "./format";
import { useMobileNav } from "./mobileNav";
import type { CollectionTrackItem } from "../../components/views/collection/types";
import type { MobileScreenProps } from "./types";

const ROW_HEIGHT = 67;

/** Open playlist, mix or Liked Songs (Figma D6/D7): back bar, hero, toolbar, search and windowed rows. */
export const CollectionPane = memo(function CollectionPane({ c }: MobileScreenProps) {
  const collection = c.activeCollection;
  const nav = useMobileNav();
  const [query, setQuery] = useState("");
  const needle = useDebouncedValue(query).trim().toLowerCase();
  const all: CollectionTrackItem[] = collection?.tracks ?? [];
  const items = useMemo(() => (needle
    ? all.filter((t) => `${t.title} ${t.artist ?? ""} ${t.album ?? ""}`.toLowerCase().includes(needle))
    : all), [all, needle]);
  if (!collection) return null;

  const liked = collection.title === "Liked Songs";
  const playing = c.isPlayingThisCollection && c.playbackState.is_playing;
  const saved = c.isCollectionSaved;

  return (
    <div className="m-screen__container">
      <div className="m-backbar">
        <button type="button" className="m-backbar__btn" onClick={nav.back} aria-label="Back">
          <ArrowLeft size={20} strokeWidth={2} />
        </button>
        <span className="m-kicker m-kicker--ink">BACK</span>
        {collection.type === "playlist" && collection.playlistId && <PlaylistOptions c={c} playlistId={collection.playlistId} name={collection.title} />}
      </div>

      <div className="m-collection-hero">
        {liked ? (
          <span className="m-collection-hero__liked"><Heart size={56} strokeWidth={2} /></span>
        ) : (
          <Sleeve size="collection" src={collection.cover_art_url} title={collection.title} />
        )}
        <div className="m-collection-hero__aside">
          <span className="m-collection-hero__tag">{liked ? "YOUR COLLECTION" : collection.tag}</span>
          <span className="m-kicker m-kicker--muted m-collection-hero__meta">
            {joinMeta(collection.curator ? `BY ${collection.curator.toUpperCase()}` : undefined, `${all.length} TRACKS`)}
          </span>
        </div>
      </div>
      <div className="m-collection-copy">
        <h1 className="m-display m-collection-copy__title">{collection.title.toUpperCase()}.</h1>
        {collection.subtitle && <p className="m-collection-copy__sub">{collection.subtitle}</p>}
      </div>
      <div className="m-collection-toolbar">
        <button type="button" className="m-btn m-btn--grow m-collection-toolbar__play" onClick={() => void c.handlePlayAllCollection()}>
          {playing ? <Pause size={18} strokeWidth={2} /> : <Play size={18} strokeWidth={2} />}
          <span>{playing ? "PAUSE" : "PLAY"}</span>
        </button>
        <button type="button" className="m-box-btn m-collection-toolbar__box" onClick={() => void c.handleShuffleCollection()} aria-label="Shuffle">
          <Shuffle size={20} strokeWidth={2} />
        </button>
        {!liked && (
          <button type="button" className={`m-box-btn m-collection-toolbar__box ${saved ? "m-box-btn--accent" : ""}`}
            disabled={saved} onClick={() => void c.handleSaveCollectionToPlaylists(collection)}
            aria-label={saved ? "Already in collection" : "Save to library"}>
            {saved ? <BookmarkCheck size={20} strokeWidth={2} /> : <BookmarkPlus size={20} strokeWidth={2} />}
          </button>
        )}
      </div>
      <div className="m-collection-search">
        <label className="m-field m-field--sm">
          <Search size={18} strokeWidth={2} className="m-field__icon" />
          <input type="text" className="m-field__input" placeholder="Search in playlist…" value={query}
            onChange={(e) => setQuery(e.target.value)} aria-label="Search in playlist" />
        </label>
      </div>

      <div className="m-collection-list">
        {items.length ? (
          <WindowedList
            items={items}
            rowHeight={ROW_HEIGHT}
            label={collection.title}
            fill
            renderItem={(item, idx) => (
              <CollectionRow key={item.id || item.matched_local_track_id || idx} item={item} c={c} />
            )}
          />
        ) : (
          <p className="m-empty-msg">{c.isLoadingCollectionTracks ? "Loading…" : "No songs in here yet. Add some from a song’s ⋮ menu."}</p>
        )}
      </div>
    </div>
  );
});
