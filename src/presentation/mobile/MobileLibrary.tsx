import { memo, useMemo, useState } from "react";
import { ChevronLeft } from "lucide-react";
import { WindowedList } from "../WindowedList";
import { useDebouncedValue } from "../useDebouncedValue";
import { MobileChip, MobileChipRow, MobileField, MobileIconButton, EmptyNote, SectionHeader } from "./primitives";
import { Sleeve } from "./Sleeve";
import { TrackRow } from "./TrackRow";
import { joinMeta } from "./format";
import type { Album, Track } from "../../types";
import type { CollectionTrackItem } from "../../components/views/collection/types";
import type { MobileScreenProps } from "./types";

type Section = "albums" | "local";
const ROW_HEIGHT = 72;
const SLEEVE_ROW_HEIGHT = 224;
const MATCH_FIELDS = ["title", "artist_name", "album_title"] as const;

/** Case-insensitive match on title, artist and album. */
export function filterTracks(tracks: readonly Track[], query: string): readonly Track[] {
  const needle = query.trim().toLowerCase();
  if (!needle) return tracks;
  return tracks.filter((t) => MATCH_FIELDS.some((f) => t[f]?.toLowerCase().includes(needle)));
}

/** Two albums per windowed row. */
export function pairAlbums(albums: readonly Album[]): Album[][] {
  const rows: Album[][] = [];
  for (let i = 0; i < albums.length; i += 2) rows.push(albums.slice(i, i + 2));
  return rows;
}

function AlbumGrid({ c }: MobileScreenProps) {
  const rows = useMemo(() => pairAlbums(c.albums), [c.albums]);
  if (!rows.length) return <EmptyNote>No albums yet. Scan your music folder.</EmptyNote>;
  return (
    <WindowedList items={rows} rowHeight={SLEEVE_ROW_HEIGHT} label="Albums" fill onEndReached={c.albumPage.next}
      renderItem={(row) => (
        <div className="m-sleeve-row">
          {row.map((album) => (
            <Sleeve key={album.id} title={album.title} subtitle={album.artist_name}
              trackId={album.first_track_id} onPress={() => c.handleSelectAlbum(album.id)} />
          ))}
        </div>
      )} />
  );
}

function LocalTracks({ c, query }: MobileScreenProps & { query: string }) {
  const source = c.libraryScoped ? c.tracks : c.trackPage.items;
  const items = useMemo(() => filterTracks(source, query), [source, query]);
  if (!items.length) return <EmptyNote>{c.trackPage.loading ? "Loading…" : "No local tracks."}</EmptyNote>;
  return (
    <WindowedList items={items} rowHeight={ROW_HEIGHT} label="Local tracks" fill onEndReached={c.trackPage.next}
      renderItem={(t) => (
        <TrackRow title={t.title} subtitle={joinMeta(t.artist_name, t.album_title)} durationSecs={t.duration_secs}
          trackId={t.id} src={t.cover_art_url} active={t.id === c.activePlayingTrackId}
          onPress={() => void c.handleLibraryRowPlay(t.id)} />
      )} />
  );
}

function CollectionPane({ c }: MobileScreenProps) {
  const collection = c.activeCollection;
  if (!collection) return null;
  const items: CollectionTrackItem[] = collection.tracks ?? [];
  return (
    <div className="m-screen__body m-screen__body--fill">
      <SectionHeader title={collection.title} action={
        <MobileIconButton label="Back" onClick={() => c.navigateBack()}><ChevronLeft size={20} /></MobileIconButton>} />
      {items.length ? (
        <WindowedList items={items} rowHeight={ROW_HEIGHT} label={collection.title} fill
          renderItem={(item) => (
            <TrackRow title={item.title} subtitle={joinMeta(item.artist, item.album)} durationSecs={item.duration_secs}
              src={item.cover_art_url} trackId={item.matched_local_track_id}
              onPress={() => void c.handlePlayCollectionTrack(item)} />
          )} />
      ) : <EmptyNote>{c.isLoadingCollectionTracks ? "Loading…" : "No tracks."}</EmptyNote>}
    </div>
  );
}

/** Library: albums and local tracks; an opened album shows its tracks. */
export const MobileLibrary = memo(function MobileLibrary({ c }: MobileScreenProps) {
  const [section, setSection] = useState<Section>("albums");
  const [query, setQuery] = useState("");
  const debounced = useDebouncedValue(query);
  if (c.activeCollection) return <CollectionPane c={c} />;
  return (
    <div className="m-screen__body m-screen__body--fill">
      <MobileChipRow label="Library section">
        <MobileChip selected={section === "albums"} onSelect={() => setSection("albums")}>Albums</MobileChip>
        <MobileChip selected={section === "local"} onSelect={() => setSection("local")}>Local</MobileChip>
      </MobileChipRow>
      {section === "local" && (
        <MobileField label="Filter" type="search" value={query} placeholder="Title, artist or album"
          onChange={(e) => setQuery(e.target.value)} />
      )}
      {section === "albums" ? <AlbumGrid c={c} /> : <LocalTracks c={c} query={debounced} />}
    </div>
  );
});
