import { memo, useEffect, useRef, useState } from "react";
import { WindowedList } from "../../presentation/WindowedList";
import { DownloadSearchResult } from "../../types";
export interface DownloadModalTrack { title: string; artist: string; album?: string; externalTrackId?: string; }
interface Props {
  isOpen: boolean; track: DownloadModalTrack | null; onClose: () => void;
  onSearchDownloads: (artist: string, title: string, album?: string) => Promise<DownloadSearchResult[]>;
  onStartDownload: (searchResultId: string, track: DownloadModalTrack) => Promise<void>;
}
export const DownloadOptionsModal = memo(function DownloadOptionsModal({ isOpen, track, onClose, onSearchDownloads, onStartDownload }: Props) {
  const search = useRef(onSearchDownloads); search.current = onSearchDownloads;
  const [songs, setSongs] = useState<DownloadSearchResult[]>([]);
  const [busy, setBusy] = useState(false); const [error, setError] = useState("");
  useEffect(() => {
    if (!isOpen || !track) return;
    let cancelled = false; setBusy(true); setError(""); setSongs([]);
    search.current(track.artist, track.title, track.album).then(result => { if (!cancelled) setSongs(result); }).catch(reason => { if (!cancelled) setError(String(reason)); }).finally(() => { if (!cancelled) setBusy(false); });
    return () => { cancelled = true; };
  }, [isOpen, track]);
  if (!isOpen || !track) return null;
  const download = async (id: string) => {
    setBusy(true); setError("");
    try { await onStartDownload(id, track); onClose(); } catch (reason) { setError(String(reason)); } finally { setBusy(false); }
  };
  return <div className="modal-overlay" onClick={onClose}><section className="modal-content" role="dialog" aria-modal="true" aria-label="Download from JioSaavn" onClick={event => event.stopPropagation()}>
    <button className="modal-close" aria-label="Close download options" onClick={onClose}>×</button>
    <h2>Download from JioSaavn</h2><p>{track.title} · {track.artist}</p>
    <p>Choose the recording to save.</p>
    {busy && <p role="status">Loading…</p>}{error && <p role="alert">{error}</p>}
    {!busy && !error && !songs.length && <p>No songs found on JioSaavn.</p>}
    <WindowedList items={songs} rowHeight={76} renderItem={song => <div className="track-row" key={song.id}><span>{song.filename}</span><button disabled={busy} onClick={() => void download(song.id)}>Download</button></div>} />
  </section></div>;
});
