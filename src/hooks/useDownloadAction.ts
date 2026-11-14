import { useEffect, useState } from "react";
import { downloadErrorMessage, isRemoveError, refreshDownloads, removeDownload, removeErrorMessage, startDownload } from "../services/downloadActions";
import { isSavedDownload, useDownloadProgress, useSongDownload } from "../services/downloadProgress";
import type { DownloadTask } from "../types";

/** What a download control shows, in priority order. */
export type DownloadControlStatus = "pending" | "removing" | "complete" | "active" | "failed" | "idle";

interface Options { trackId: string; sourceId?: string; title: string; downloaded?: boolean; pending?: boolean }

interface Facts { pending: boolean; removing: boolean; busy: boolean; error: string; downloaded: boolean; task?: DownloadTask; bytes: number; size?: number }

interface Description { status: DownloadControlStatus; complete: boolean; failed: boolean; percent: number }

const idle: DownloadTask = { id: "", provider: "", title: "", artist: "", filename: "", bytes_downloaded: 0, status: "QUEUED", created_at: 0 };

function percentOf(bytes: number, size: number | undefined) {
  return size && size > 0 ? Math.min(99, Math.floor(100 * bytes / size)) : 0;
}

/** What the control shows, from the song's task and this control's own activity. */
export function describeDownload({ pending, removing, busy, error, downloaded, task, bytes, size }: Facts): Description {
  const complete = !pending && (downloaded || isSavedDownload(task));
  const transferring = task?.status === "DOWNLOADING" || task?.status === "QUEUED";
  const downloading = !complete && (busy || transferring);
  const failed = !!error && !removing && !downloading && (!complete || isRemoveError(error));
  const order: [DownloadControlStatus, boolean][] = [["pending", pending], ["removing", removing], ["complete", complete], ["active", downloading], ["failed", failed]];
  const status = order.find(([, on]) => on)?.[0] ?? "idle";
  const percent = complete ? 100 : downloading ? percentOf(bytes, size) : 0;
  return { status, complete, failed, percent };
}

/** State and click behaviour of one song's download control. */
export function useDownloadAction({ trackId, sourceId, title, downloaded = false, pending = false }: Options) {
  const task = useSongDownload(trackId, sourceId);
  const progress = useDownloadProgress(task || idle);
  const [busy, setBusy] = useState(false);
  const [removing, setRemoving] = useState(false);
  const [error, setError] = useState("");
  const { status, complete, failed, percent } = describeDownload({
    pending, removing, busy, error, downloaded, task, bytes: progress.bytes_downloaded, size: progress.file_size,
  });

  // A stale failure must never outlive the attempt it describes: drop it once the
  // song finishes (e.g. it was already downloaded) and auto-dismiss it otherwise.
  useEffect(() => {
    if (!error) return;
    if (complete && !isRemoveError(error)) { setError(""); return; }
    const timer = window.setTimeout(() => setError(""), 5000);
    return () => window.clearTimeout(timer);
  }, [error, complete]);

  const remove = async () => {
    setRemoving(true);
    try { await removeDownload(trackId, sourceId, title); }
    catch (failure) { setError(removeErrorMessage(failure)); void refreshDownloads().catch(() => {}); }
    finally { setRemoving(false); }
  };
  const download = async () => {
    setBusy(true);
    try { await startDownload(trackId, sourceId); }
    catch (failure) {
      setError(downloadErrorMessage(failure));
      // Resync: the song may already be downloaded, in which case the check replaces the error.
      void refreshDownloads().catch(() => {});
    }
    finally { setBusy(false); }
  };
  const run = () => { setError(""); return complete ? remove() : download(); };

  return { status, failed, error, percent, run };
}
