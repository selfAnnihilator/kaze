import { dispatchCommand, executeQuery } from "./api";
import { confirmAction } from "./confirmDialog";
import { downloadProgress } from "./downloadProgress";

/** One request per recording, however many buttons show it. */
const starting = new Map<string, Promise<void>>();
const REMOVE_FAILED = "Couldn’t remove this download.";
const fallbackError = "Couldn’t download this song. Click to retry.";

export const isRemoveError = (error: string) => error.startsWith(REMOVE_FAILED);

function reasonOf(failure: unknown): string {
  const text = typeof failure === "string" ? failure : failure instanceof Error ? failure.message : "";
  return text.replace(/^[A-Za-z ]*error:\s*/i, "").trim();
}

/** Prefer the backend's reason (Tauri rejects with a string) over a generic message. */
export function downloadErrorMessage(failure: unknown): string {
  const reason = reasonOf(failure);
  return reason && reason !== "Download did not start" ? `${reason} Click to retry.` : fallbackError;
}

export function removeErrorMessage(failure: unknown): string {
  const reason = reasonOf(failure);
  return reason ? `${REMOVE_FAILED} ${reason}` : REMOVE_FAILED;
}

/** Reloads every download record into the live store. */
export const refreshDownloads = () => executeQuery({ query: "GetDownloads", payload: { limit: 10000 } })
  .then(result => { if (Array.isArray(result.data)) downloadProgress.replace(result.data); });

/** Starts (or joins) the download of this recording. Rejects with the backend's reason. */
export function startDownload(trackId: string, sourceId?: string): Promise<void> {
  const key = sourceId || trackId;
  let request = starting.get(key);
  if (!request) {
    request = (async () => {
      const response = await dispatchCommand({ command: "DownloadTrack", payload: { track_id: trackId } });
      if (!response?.data?.task_id) throw new Error("Download did not start");
      downloadProgress.attach(trackId, response.data.task_id);
      await refreshDownloads();
    })().finally(() => starting.delete(key));
    starting.set(key, request);
  }
  return request;
}

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
