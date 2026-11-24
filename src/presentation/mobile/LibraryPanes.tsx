import { memo, useEffect, useRef, useState } from "react";
import { useSleeveSlide } from "./sleeveSlide";
import { useSideSwipe } from "./useSideSwipe";
import { openTrackSheet, type SheetTrack } from "./sheets/trackSheet";
import { ArrowLeft, ArrowRight, Link, Play, Plus, RotateCw, Search, Shuffle } from "lucide-react";
import { ImportPlaylistSheet } from "./playlists/ImportPlaylistSheet";
import { DownloadButton } from "../../components/DownloadButton";
import { usePromptDialog } from "../../hooks/app/usePromptDialog";
import { WindowedList } from "../WindowedList";
import { Sleeve } from "./Sleeve";
import { TrackRow } from "./TrackRow";
import { joinMeta } from "./format";
import type { Playlist, Track } from "../../types";
import type { MobileScreenProps } from "./types";
import { canRate } from "../../services/ratingRule";
import { playAlbum } from "./playAlbum";

const ROW_HEIGHT = 67;
const pad3 = (n: number) => String(n).padStart(3, "0");

/** Albums record browser (Figma D1): one record at a time; swipe sideways or use Earlier / Later. */
export const AlbumsPane = memo(function AlbumsPane({ c, focusId }: MobileScreenProps & { focusId?: string | null }) {
  const albums = c.albums;
  const [index, setIndex] = useState(0);
  // "Open album" from the Deck: jump to that record.
  useEffect(() => {
    const at = focusId ? albums.findIndex((a) => a.id === focusId) : -1;
    if (at >= 0) setIndex(at);
  }, [focusId, albums]);
  const total = c.albumPage.total || albums.length;
  const at = Math.min(index, Math.max(0, albums.length - 1));
  const album = albums[at];
  const prev = albums[at - 1];
  const next = albums[at + 1];
  const later = () => {
    if (at + 1 >= albums.length - 2) c.albumPage.next();
    setIndex(Math.min(albums.length - 1, at + 1));
  };
  const earlier = () => setIndex(Math.max(0, at - 1));
  const showcase = useRef<HTMLDivElement>(null);
  useSideSwipe(showcase, (step) => (step > 0 ? later() : earlier()));
  const slide = useSleeveSlide(album?.id, prev?.id, next?.id);
  if (!albums.length) return <p className="m-empty-msg">{c.albumPage.loading ? "Loading albums…" : "No albums yet. Add a music folder in Settings › Music folders."}</p>;

  return (
    <div className="m-albums">
      <div className="m-albums__stats">
        <span className="m-kicker">RECORD {pad3(at + 1)} / {pad3(total)}</span>
        <span className="m-kicker m-kicker--ink">SORT: A—Z</span>
      </div>
      <div ref={showcase} className="m-albums__showcase">
        <span className="m-albums__side">{prev && <Sleeve key={prev.id} size="edge" src={prev.cover_art_path} trackId={prev.first_track_id} title={prev.title} onPress={earlier} />}</span>
        <div key={album.id} className={slide}>
          <Sleeve size="album" src={album.cover_art_path} trackId={album.first_track_id} title={album.title}
            onPress={() => void playAlbum(album.id)} />
        </div>
        <span className="m-albums__side">{next && <Sleeve key={next.id} size="edge" src={next.cover_art_path} trackId={next.first_track_id} title={next.title} onPress={later} />}</span>
      </div>
      <div className="m-albums__info">
        <h2 className="m-display m-albums__title">{album.title.toUpperCase()}</h2>
        <span className="m-kicker m-kicker--muted">
          {joinMeta((album.artist_name || "Unknown artist").toUpperCase(), album.year ? String(album.year) : undefined, album.track_count ? `${album.track_count} TRACKS` : undefined)}
        </span>
      </div>
      <div className="m-albums__actions">
        <button type="button" className="m-btn m-btn--outline m-btn--sm m-btn--grow" disabled={at === 0} onClick={earlier}>
          <ArrowLeft size={18} strokeWidth={2} /><span>EARLIER</span>
        </button>
        <button type="button" className="m-btn m-btn--sm m-albums__play" disabled={!album.first_track_id}
          onClick={() => void playAlbum(album.id)}>
          <Play size={18} strokeWidth={2} /><span>PLAY</span>
        </button>
        <button type="button" className="m-btn m-btn--outline m-btn--sm m-btn--grow" disabled={at >= total - 1} onClick={later}>
          <ArrowRight size={18} strokeWidth={2} /><span>LATER</span>
        </button>
      </div>
    </div>
  );
});

type PlayHandlers = Pick<MobileScreenProps["c"], "downloadsListActive" | "libraryShuffle" | "handleLibraryRowPlay" | "handleLibraryPlayAll" | "handleLibraryToggleShuffle">;

/** Play all / pause, shuffle and (Local only) rescan. */
const TracksToolbar = memo(function TracksToolbar({ play, playing, onRescan }: { play: PlayHandlers; playing: boolean; onRescan?: () => void }) {
  return (
    <div className="m-tracks__actions">
      <button type="button" className="m-btn m-btn--sm m-btn--grow" onClick={() => void play.handleLibraryPlayAll()}>
        <Play size={18} strokeWidth={2} /><span>{play.downloadsListActive && playing ? "PAUSE" : "PLAY ALL"}</span>
      </button>
      <button type="button" className={`m-btn m-btn--outline m-btn--sm ${play.libraryShuffle ? "is-on" : ""}`}
        aria-pressed={play.libraryShuffle} onClick={() => void play.handleLibraryToggleShuffle()}>
        <Shuffle size={18} strokeWidth={2} /><span>SHUFFLE</span>
      </button>
      {onRescan && (
        <button type="button" className="m-box-btn" onClick={onRescan} aria-label="Rescan library">
          <RotateCw size={20} strokeWidth={2} />
        </button>
      )}
    </div>
  );
});

/** Right side of a Local row (format over time) or a Downloads row (remove download). */
const sheetTrack = (t: Track): SheetTrack => ({
  id: t.id, title: t.title, artist: t.artist_name, album: t.album_title, cover: t.cover_art_url, localFile: !canRate(t),
});

function trackTrailing(t: Track, downloads: boolean) {
  if (downloads) {
    return <span className="m-tracks__download"><DownloadButton trackId={t.id} title={t.title} downloaded compact /></span>;
  }
  return (
    <span className="m-tracks__meta">
      <span className="m-tracks__format">{(t.format || "").toUpperCase()}</span>
      <span className="m-track-row__time">{formatTime(t.duration_secs)}</span>
    </span>
  );
}

function emptyText(loading: boolean, downloads: boolean): string {
  if (loading) return "Loading…";
  return downloads ? "No downloaded songs yet. Tap the download icon on any online song to keep it here." : "No songs here yet. Add a music folder in Settings › Music folders, then rescan.";
}

/** Local and Downloads (Figma D2/D3): play all, shuffle, rescan, search, then a windowed track list. */
export const TracksPane = memo(function TracksPane({
  c, downloads, query, onQuery,
}: MobileScreenProps & { downloads: boolean; query: string; onQuery: (q: string) => void }) {
  const play: PlayHandlers = downloads ? c.kazeDownloadsPlayback : c;
  const page = downloads ? c.downloadsPage : c.trackPage;
  const tracks: Track[] = downloads ? c.downloadsPage.items : c.tracks;
  const paged = downloads || !c.libraryScoped;

  return (
    <>
      <TracksToolbar play={play} playing={c.playbackState.is_playing} onRescan={downloads ? undefined : () => void c.handleRescanLibrary()} />
      {!downloads && (
        <div className="m-tracks__search">
          <label className="m-field m-field--sm">
            <Search size={18} strokeWidth={2} className="m-field__icon" />
            <input type="text" className="m-field__input" placeholder="Search title, artist, album…" value={query}
              onChange={(e) => onQuery(e.target.value)} aria-label="Search library" />
          </label>
        </div>
      )}
      <div className="m-tracks__list">
        {tracks.length ? (
          <WindowedList
            items={tracks}
            rowHeight={ROW_HEIGHT}
            label={downloads ? "Downloads" : "Local tracks"}
            fill
            onEndReached={paged ? page.next : undefined}
            renderItem={(t: Track, idx: number) => (
              <TrackRow
                key={t.id}
                index={idx + 1}
                title={t.title}
                subtitle={joinMeta(t.artist_name, t.album_title)}
                src={t.cover_art_url}
                trackId={t.id}
                active={t.id === c.activePlayingTrackId}
                onPress={() => void play.handleLibraryRowPlay(t.id)}
                trailing={trackTrailing(t, downloads)}
                onMore={downloads ? undefined : () => openTrackSheet("actions", sheetTrack(t))}
              />
            )}
          />
        ) : (
          <p className="m-empty-msg">{emptyText(page.loading, downloads)}</p>
        )}
      </div>
    </>
  );
});

function formatTime(secs: number | undefined): string {
  if (!secs || !Number.isFinite(secs)) return "";
  const s = Math.floor(secs % 60);
  return `${Math.floor(secs / 60)}:${s < 10 ? "0" : ""}${s}`;
}

/** Playlists (Figma D4): New / Import, then numbered red-index rows. */
export const PlaylistsPane = memo(function PlaylistsPane({ c, onOpen }: MobileScreenProps & { onOpen: (p: Playlist) => void }) {
  const { requestPrompt, promptDialog } = usePromptDialog();
  const [importing, setImporting] = useState(false);
  const create = async () => {
    const name = await requestPrompt("New playlist", "Name it.");
    if (name?.trim()) await c.handleCreatePlaylist(name.trim());
  };
  return (
    <div className="m-playlists">
      <div className="m-tracks__actions">
        <button type="button" className="m-btn m-btn--sm m-btn--grow" onClick={() => void create()}>
          <Plus size={18} strokeWidth={2} /><span>NEW PLAYLIST</span>
        </button>
        <button type="button" className="m-btn m-btn--outline m-btn--sm m-btn--grow" onClick={() => setImporting(true)}>
          <Link size={18} strokeWidth={2} /><span>IMPORT</span>
        </button>
      </div>
      {importing && <ImportPlaylistSheet c={c} onClose={() => setImporting(false)} />}
      {promptDialog}
      {c.playlists.map((p, idx) => (
        <div key={p.id} className="m-playlist-row">
          <button type="button" className="m-playlist-row__main" onClick={() => onOpen(p)} aria-label={`Open ${p.name}`}>
            <span className="m-playlist-row__index">{String(idx + 1).padStart(2, "0")}</span>
            <Sleeve size="playlist" src={p.cover_art_url} trackId={p.first_track_id} title={p.name} />
            <span className="m-playlist-row__text">
              <span className="m-playlist-row__title">{p.name}</span>
              <span className="m-kicker m-kicker--muted">
                {joinMeta(p.is_smart_mix === 1 ? "SMART MIX" : undefined, `${p.track_count ?? 0} TRACKS`)}
              </span>
            </span>
          </button>
          <button type="button" className="m-home-row__play" onClick={() => void c.handlePlayPlaylist(p.id)} aria-label={`Play ${p.name}`}>
            <Play size={18} strokeWidth={2} />
          </button>
        </div>
      ))}
      {!c.playlists.length && <p className="m-empty-msg">No playlists yet. Make one with New Playlist, or import one from a link.</p>}
    </div>
  );
});
