import { useMemo, useState } from "react";
import { Download, Link, ListPlus, Search, X } from "lucide-react";
import { WindowedList } from "../../WindowedList";
import { useDebouncedValue } from "../../useDebouncedValue";
import { Sleeve } from "../Sleeve";
import { joinMeta } from "../format";
import type { JioSaavnImportedTrack, JioSaavnPlaylistImport } from "../../../ipcTypes";
import type { MobileScreenProps } from "../types";

type Ctx = MobileScreenProps["c"];
const ROW = 59;
const failText = (e: unknown) => (typeof e === "string" ? e : e instanceof Error ? e.message : "Couldn’t read that playlist. Check the link and try again.");

function useImport(c: Ctx, onDone: () => void) {
  const [link, setLink] = useState("");
  const [result, setResult] = useState<JioSaavnPlaylistImport | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const run = async (work: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    try { await work(); } catch (e) { setError(failText(e)); } finally { setBusy(false); }
  };
  const preview = () => run(async () => {
    const res = await c.handleInspectJioSaavnPlaylist(link.trim());
    if (!res) throw "Couldn’t read that playlist. Check the link and try again.";
    setResult(res);
  });
  const save = () => run(async () => {
    if (!result) return;
    if (!c.currentUser) { c.setIsAuthModalOpen(true); return; }
    const info = result.tracks.map((t) => ({ id: t.track_id, title: t.title, artist: t.artist, album: t.album, duration_secs: t.duration_secs, cover_art_url: t.cover_art_url }));
    await c.handleSaveImportedPlaylist(result.title, info.map((t) => t.id), info);
    onDone();
  });
  return { link, setLink, result, busy, error, preview, save };
}

function ImportRow({ t }: { t: JioSaavnImportedTrack }) {
  const local = t.in_library || t.local_copy;
  return (
    <div className="m-import-row">
      <Sleeve size="row" src={t.cover_art_url} title={t.title} />
      <span className="m-track-row__text"><strong className="m-track-row__title">{t.title}</strong><span className="m-track-row__meta">{joinMeta(t.artist, t.album)}</span></span>
      {local ? <span className="m-top-result__tag">LOCAL</span> : <Download size={18} strokeWidth={2} aria-label="Not in your library" />}
    </div>
  );
}

/** Import Playlist (Figma G5): paste a JioSaavn or Spotify link, preview it, then add it as a playlist. */
export function ImportPlaylistSheet({ c, onClose }: { c: Ctx; onClose: () => void }) {
  const s = useImport(c, onClose);
  const [filter, setFilter] = useState("");
  const needle = useDebouncedValue(filter).trim().toLowerCase();
  const tracks = useMemo(() => (s.result?.tracks ?? []).filter((t) => !needle || `${t.title} ${t.artist}`.toLowerCase().includes(needle)), [s.result, needle]);
  const r = s.result;
  const local = r ? r.tracks.filter((t) => t.in_library || t.local_copy).length : 0;
  return (
    <section className="m-sheet m-import" role="dialog" aria-modal="true" aria-label="Import playlist">
      <button type="button" className="m-backbar m-backbar__back" onClick={onClose}><X size={20} strokeWidth={2} /><span className="m-kicker m-kicker--ink">CLOSE</span></button>
      <div className="m-import__head">
        <span className="m-kicker">JIOSAAVN OR SPOTIFY LINK</span>
        <h1 className="m-display m-import__title">IMPORT PLAYLIST.</h1>
        <form className="m-import__form" onSubmit={(e) => { e.preventDefault(); if (s.link.trim()) void s.preview(); }}>
          <label className="m-field m-field--accent">
            <Link size={18} strokeWidth={2} className="m-field__icon" />
            <input className="m-field__input" value={s.link} onChange={(e) => s.setLink(e.target.value)} placeholder="Paste a playlist link…" aria-label="Playlist link" autoCapitalize="none" />
          </label>
          <button type="submit" className="m-btn m-import__preview" disabled={s.busy || !s.link.trim()}>{s.busy && !r ? "…" : "PREVIEW"}</button>
        </form>
        {r && (
          <div className="m-import__summary">
            <Sleeve size="row" src={r.cover_url} title={r.title} />
            <span className="m-track-row__text"><strong className="m-import__name">{r.title}</strong>
              <span className="m-kicker m-kicker--muted">{r.total_tracks} TRACKS · {local} LOCAL · {r.total_tracks - local} MISSING</span></span>
          </div>
        )}
        {r && (
          <label className="m-field m-field--sm">
            <Search size={18} strokeWidth={2} className="m-field__icon" />
            <input className="m-field__input" value={filter} onChange={(e) => setFilter(e.target.value)} placeholder="Search in playlist…" aria-label="Search in playlist" />
          </label>
        )}
        {s.error && <p className="m-set-note is-error" role="alert">{s.error}</p>}
      </div>
      <div className="m-import__list">
        {r ? (tracks.length ? <WindowedList items={tracks} rowHeight={ROW} label="Playlist songs" fill renderItem={(t: JioSaavnImportedTrack) => <ImportRow key={t.track_id} t={t} />} />
          : <p className="m-empty-msg">No songs match “{filter}”.</p>)
          : <p className="m-empty-msg">Nothing to show yet. Paste a playlist link above and tap Preview.</p>}
      </div>
      {r && (
        <div className="m-import__actions">
          <button type="button" className="m-btn m-btn--grow" disabled={s.busy || !r.tracks.length} onClick={() => void s.save()}>
            <ListPlus size={18} strokeWidth={2} /><span>{s.busy ? "ADDING…" : "ADD AS PLAYLIST"}</span>
          </button>
        </div>
      )}
    </section>
  );
}
