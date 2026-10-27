import { memo, useState } from "react";
import { CheckCircle2, Download, RotateCw } from "lucide-react";
import { dispatchCommand, executeQuery } from "../services/api";
import { downloadProgress, useDownloadProgress, useSongDownload } from "../services/downloadProgress";
import type { DownloadTask } from "../types";
const starting = new Map<string, Promise<void>>();
const idle: DownloadTask = { id: "", provider: "", title: "", artist: "", filename: "", bytes_downloaded: 0, status: "QUEUED", created_at: 0 };
export const DownloadButton = memo(function DownloadButton({ trackId, sourceId, title, compact = false, downloaded = false }: { trackId: string; sourceId?: string; title: string; compact?: boolean; downloaded?: boolean }) {
  const task = useSongDownload(trackId, sourceId);
  const progress = useDownloadProgress(task || idle);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const complete = downloaded || task?.status === "COMPLETED";
  const transferring = task?.status === "DOWNLOADING" || task?.status === "QUEUED";
  const active = busy || transferring;
  const percent = complete ? 100 : active && progress.file_size && progress.file_size > 0 ? Math.min(99, Math.floor(100 * progress.bytes_downloaded / progress.file_size)) : 0;
  const label = complete ? `Downloaded ${title}` : active ? `Downloading ${title}: ${percent}%` : `Download ${title}`;
  return <span className="song-download-control">
    <button data-track-id={trackId} type="button" className={`song-download-button ${compact ? "is-compact" : ""}`} title={error || label} aria-label={label} disabled={active || complete} onClick={async event => {
      event.stopPropagation(); setError(""); setBusy(true);
      const key = sourceId || trackId;
      let request = starting.get(key);
      if (!request) {
        request = (async () => {
          const response = await dispatchCommand({ command: "DownloadTrack", payload: { track_id: trackId } });
          if (!response?.data?.task_id) throw new Error("Download did not start");
          downloadProgress.attach(trackId, response.data.task_id);
          const result = await executeQuery({ query: "GetDownloads", payload: { limit: 10000 } });
          if (Array.isArray(result.data)) downloadProgress.replace(result.data);
        })();
        starting.set(key, request);
      }
      try { await request; } catch { setError("Couldn’t download this song. Click to retry."); }
      finally { starting.delete(key); setBusy(false); }
    }}>
      <span className="song-download-fill" style={{ transform: `scaleX(${percent / 100})` }} />
      <span className="song-download-label">{complete ? <CheckCircle2 size={16} /> : active ? <RotateCw size={14} className="spin-animation" /> : <Download size={16} />}{active ? <span>{percent}%</span> : !compact && <span>{complete ? "Downloaded" : "Download"}</span>}</span>
    </button>
    {error && <span className="song-download-error" role="alert">{error}</span>}
  </span>;
});
