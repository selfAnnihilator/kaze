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
}
export const DownloadsView = React.memo(function DownloadsView({ downloads, onSearchDownloads, onStartDownload: _onStartDownload, onCancelDownload, onRefreshDownloads, initialSearch }: DownloadsViewProps) {
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

  return <section className="downloads-view">
    <div className="view-header"><h1>Downloads</h1><button className="btn btn-secondary" onClick={() => void onRefreshDownloads()}>Refresh downloads</button></div>
    <p>Save songs to your library as M4A audio.</p>
    <form onSubmit={search} style={{ display: "flex", gap: 12, flexWrap: "wrap", marginBottom: 24 }}>
      <input aria-label="Download artist" placeholder="Artist" value={artist} onChange={event => setArtist(event.target.value)} />
      <input aria-label="Download song" placeholder="Song" value={title} onChange={event => setTitle(event.target.value)} />
      <button className="btn btn-primary" disabled={busy || (!artist.trim() && !title.trim())}>{busy ? "Searching…" : "Search music"}</button>
    </form>
    {message && <p role="status">{message}</p>}
    {!!results.length && <WindowedTable items={results} renderItem={song => <tr key={song.id}><td>{song.filename}</td><td>{song.username}</td><td><DownloadButton key={song.id} trackId={song.id} sourceId={song.id.startsWith("jiosaavn:") ? song.id.slice(9) : undefined} title={song.filename} /></td></tr>}><thead><tr><th>Recording</th><th>Artist</th><th>Save</th></tr></thead></WindowedTable>}
    <h2>Saved downloads</h2>
    {!downloads.length && <p>No downloads yet.</p>}
    <WindowedTable items={downloads} renderItem={task => <LiveDownloadTask task={task}>{live => <tr key={live.id}><td>{live.title}</td><td>{live.artist}</td><td>{live.status}</td><td>{live.bytes_downloaded.toLocaleString()} bytes</td><td>{["DOWNLOADING", "QUEUED"].includes(live.status) && <button onClick={() => void onCancelDownload(live.id)}>Cancel</button>}</td></tr>}</LiveDownloadTask>}><thead><tr><th>Song</th><th>Artist</th><th>Status</th><th>Saved</th><th>Action</th></tr></thead></WindowedTable>
  </section>;
});
