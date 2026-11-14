import { DownloadButton } from "../DownloadButton";
import React, { useState } from "react";
import { DownloadTask, DownloadSearchResult } from "../../types";
import { WindowedTable } from "../../presentation/WindowedTable";
import { LiveDownloadTask } from "../../presentation/LiveDownloadTask";

interface DownloadsViewProps {
  downloads: DownloadTask[];
  onSearchDownloads: (artist: string, title: string, album?: string) => Promise<DownloadSearchResult[]>;
  onStartDownload: (searchResultId: string, wishlistId?: string) => Promise<void>;
  onCancelDownload: (taskId: string) => Promise<void>;
  onRefreshDownloads: () => Promise<void>;
  initialSearch?: { artist: string; title: string; album?: string } | null;
  /** The playable list of songs saved in Kaze's download folder. */
  songs?: React.ReactNode;
  /** Kaze's download folder, shown so users know where files live. */
  folder?: string;
}

const statusLabel = (task: DownloadTask) =>
  task.status === "MISSING" ? "File moved or deleted"
    : task.status === "FAILED" ? "Failed"
    : task.status === "CANCELLED" ? "Cancelled"
    : task.status === "QUEUED" ? "Waiting"
    : "Downloading";

/** Downloads = songs Kaze downloaded into its own folder (your library folders are separate). */
export const DownloadsView = React.memo(function DownloadsView({ downloads, onSearchDownloads, onStartDownload: _onStartDownload, onCancelDownload, onRefreshDownloads, initialSearch, songs, folder }: DownloadsViewProps) {
  const [artist, setArtist] = useState(initialSearch?.artist || "");
  const [title, setTitle] = useState(initialSearch?.title || "");
  const [results, setResults] = useState<DownloadSearchResult[]>([]);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const search = async (event: React.FormEvent) => {
    event.preventDefault(); setBusy(true); setMessage("");
    try { const songs = await onSearchDownloads(artist, title); setResults(songs); if (!songs.length) setMessage("No songs found."); }
    catch (error) { setMessage(String(error)); } finally { setBusy(false); }
  };
  // Completed downloads appear in the song list; only transfers needing attention show here.
  const transfers = downloads.filter(task => task.status !== "COMPLETED");

  return <section className="downloads-view">
    <div className="view-header"><h1>Downloads</h1><button className="btn btn-secondary" onClick={() => void onRefreshDownloads()}>Refresh downloads</button></div>
    <p>Songs Kaze saved as M4A{folder ? <> in <code>{folder}</code></> : null}. Your own music folders are in Library.</p>
    {!!transfers.length && <>
      <h2>Transfers</h2>
      <WindowedTable items={transfers} renderItem={task => <LiveDownloadTask task={task}>{live => <tr key={live.id}><td>{live.title}</td><td>{live.artist}</td><td title={live.error_message}>{statusLabel(live)}</td><td>{live.bytes_downloaded.toLocaleString()} bytes</td><td>{["DOWNLOADING", "QUEUED"].includes(live.status) ? <button onClick={() => void onCancelDownload(live.id)}>Cancel</button> : live.source_track_id ? <DownloadButton trackId={`jiosaavn:${live.source_track_id}`} sourceId={live.source_track_id} title={live.title} compact /> : null}</td></tr>}</LiveDownloadTask>}><thead><tr><th>Song</th><th>Artist</th><th>Status</th><th>Saved</th><th>Action</th></tr></thead></WindowedTable>
    </>}
    <h2>Saved songs</h2>
    <div className="downloads-songs" style={{ minHeight: 0 }}>{songs}</div>
    <h2>Find a song to download</h2>
    <form onSubmit={search} style={{ display: "flex", gap: 12, flexWrap: "wrap", marginBottom: 24 }}>
      <input aria-label="Download artist" placeholder="Artist" value={artist} onChange={event => setArtist(event.target.value)} />
      <input aria-label="Download song" placeholder="Song" value={title} onChange={event => setTitle(event.target.value)} />
      <button className="btn btn-primary" disabled={busy || (!artist.trim() && !title.trim())}>{busy ? "Searching…" : "Search music"}</button>
    </form>
    {message && <p role="status">{message}</p>}
    {!!results.length && <WindowedTable items={results} renderItem={song => <tr key={song.id}><td>{song.filename}</td><td>{song.username}</td><td><DownloadButton key={song.id} trackId={song.id} sourceId={song.id.startsWith("jiosaavn:") ? song.id.slice(9) : undefined} title={song.filename} /></td></tr>}><thead><tr><th>Recording</th><th>Artist</th><th>Save</th></tr></thead></WindowedTable>}
  </section>;
});
