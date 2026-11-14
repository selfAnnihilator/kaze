import { memo, useEffect, useState } from "react";
import { AlertCircle, CheckCircle2, Download, RotateCw } from "lucide-react";
import { dispatchCommand, executeQuery } from "../services/api";
import { confirmAction } from "../services/confirmDialog";
import { downloadProgress, isSavedDownload, useDownloadProgress, useSongDownload } from "../services/downloadProgress";
import type { DownloadTask } from "../types";
const starting = new Map<string, Promise<void>>();
const REMOVE_FAILED = "Couldn’t remove this download.";
const removeFailed = (error: string) => error.startsWith(REMOVE_FAILED);
const fallbackError = "Couldn’t download this song. Click to retry.";
/** Prefer the backend's reason (Tauri rejects with a string) over a generic message. */
function downloadErrorMessage(failure: unknown): string {
  const text = typeof failure === "string" ? failure : failure instanceof Error ? failure.message : "";
  const reason = text.replace(/^[A-Za-z ]*error:\s*/i, "").trim();
  return reason && reason !== "Download did not start" ? `${reason} Click to retry.` : fallbackError;
}
const refreshDownloads = () => executeQuery({ query: "GetDownloads", payload: { limit: 10000 } })
  .then(result => { if (Array.isArray(result.data)) downloadProgress.replace(result.data); });
/** Asks first, then deletes Kaze's file; the song plays online again. Returns false if kept. */
export async function removeDownload(trackId: string, sourceId: string | undefined, title: string): Promise<boolean> {
  const confirmed = await confirmAction({
    title: "Remove from downloads?",
    message: `“${title}” will be deleted from this device. It stays in your library and plays online.`,
    confirmLabel: "Yes, remove",
    cancelLabel: "No",
  });
  if (!confirmed) return false;
  await dispatchCommand({ command: "RemoveDownload", payload: { track_id: trackId, source_id: sourceId } });
  await refreshDownloads();
  return true;
}
const idle: DownloadTask = { id: "", provider: "", title: "", artist: "", filename: "", bytes_downloaded: 0, status: "QUEUED", created_at: 0 };
export const DownloadButton = memo(function DownloadButton({ trackId, sourceId, title, compact = false, downloaded = false, pending = false }: { trackId: string; sourceId?: string; title: string; compact?: boolean; downloaded?: boolean; /** Track details still loading: show a neutral, disabled control. */ pending?: boolean }) {
  const task = useSongDownload(trackId, sourceId);
  const progress = useDownloadProgress(task || idle);
  const [busy, setBusy] = useState(false);
  const [removing, setRemoving] = useState(false);
  const [error, setError] = useState("");
  const complete = !pending && (downloaded || isSavedDownload(task));
  const transferring = task?.status === "DOWNLOADING" || task?.status === "QUEUED";
  const active = removing || (!complete && (busy || transferring));
  const percent = complete ? 100 : active && progress.file_size && progress.file_size > 0 ? Math.min(99, Math.floor(100 * progress.bytes_downloaded / progress.file_size)) : 0;
  const failed = !!error && !active && (!complete || removeFailed(error));
  const label = pending ? `Checking download status of ${title}` : removing ? `Removing ${title} from downloads` : complete ? `Downloaded ${title}. Click to remove` : active ? `Downloading ${title}: ${percent}%` : failed ? `Retry download of ${title}` : `Download ${title}`;
  // A stale failure must never outlive the attempt it describes: drop it once the
  // song finishes (e.g. it was already downloaded) and auto-dismiss it otherwise.
  useEffect(() => {
    if (!error) return;
    if (complete && !removeFailed(error)) { setError(""); return; }
    const timer = window.setTimeout(() => setError(""), 5000);
    return () => window.clearTimeout(timer);
  }, [error, complete]);
  return <span className="song-download-control">
    <button data-track-id={trackId} type="button" className={`song-download-button ${compact ? "is-compact" : ""} ${complete ? "is-complete" : ""} ${failed ? "is-failed" : ""}`} title={failed ? error : label} aria-label={label} disabled={pending || active} aria-busy={pending || removing || undefined} onClick={async event => {
      event.stopPropagation(); setError("");
      if (complete) {
        setRemoving(true);
        try { await removeDownload(trackId, sourceId, title); }
        catch (failure) { setError(`${REMOVE_FAILED} ${downloadErrorMessage(failure).replace(/ Click to retry\.$/, "")}`); void refreshDownloads().catch(() => {}); }
        finally { setRemoving(false); }
        return;
      }
      setBusy(true);
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
      try { await request; } catch (failure) {
        setError(downloadErrorMessage(failure));
        // Resync: the song may already be downloaded, in which case the check replaces the error.
        executeQuery({ query: "GetDownloads", payload: { limit: 10000 } }).then(result => { if (Array.isArray(result.data)) downloadProgress.replace(result.data); }).catch(() => {});
      }
      finally { starting.delete(key); setBusy(false); }
    }}>
      {!complete && !removing && <span className="song-download-fill" style={{ transform: `scaleX(${percent / 100})` }} />}
      <span className="song-download-label">{removing ? <RotateCw size={14} className="spin-animation" /> : complete ? <CheckCircle2 size={16} /> : active ? <RotateCw size={14} className="spin-animation" /> : failed ? <AlertCircle size={16} /> : <Download size={16} />}{active && !removing ? <span className="song-download-percent">{percent}%</span> : !compact && !complete && !pending && <span>{failed ? "Retry" : "Download"}</span>}</span>
    </button>
    {failed && !compact && <span className="song-download-error" role="alert">{error}</span>}
    {failed && compact && <span className="sr-only" role="alert">{error}</span>}
  </span>;
});
