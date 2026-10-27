import { useCallback, useSyncExternalStore } from "react";
import type { DownloadTask } from "../types";

type Progress = { bytes_downloaded: number; file_size?: number };
const tasksById = new Map<string, DownloadTask>();
const taskListeners = new Set<() => void>();
const trackTasks = new Map<string, string>();
const snapshots = new Map<string, Progress>();
const listeners = new Map<string, Set<() => void>>();

/** Transfer ticks belong to visible transfer rows, never the application shell. */
export const downloadProgress = {
  replace(tasks: readonly DownloadTask[]) {
    const previousTasks = new Map(tasksById);
    tasksById.clear();
    for (const task of tasks) {
      const previous = previousTasks.get(task.id);
      const terminal = previous && ["COMPLETED", "FAILED", "CANCELLED"].includes(previous.status);
      tasksById.set(task.id, terminal && previous && ["DOWNLOADING", "QUEUED"].includes(task.status) ? previous : task);
    }
    const present = new Set(tasks.map((task) => task.id));
    for (const id of snapshots.keys())
      if (!present.has(id)) snapshots.delete(id);
    for (const task of tasksById.values()) {
      const previous = snapshots.get(task.id);
      snapshots.set(task.id, {
        bytes_downloaded: Math.max(task.bytes_downloaded, previous?.bytes_downloaded || 0),
        file_size: task.file_size || previous?.file_size,
      });
      listeners.get(task.id)?.forEach((notify) => notify());
    }
    taskListeners.forEach(notify => notify());
  },
  task(trackId: string, sourceId?: string) {
    const attached = trackTasks.get(trackId);
    const source = sourceId || (trackId.startsWith("jiosaavn:") ? trackId.slice(9) : undefined);
    return (attached ? tasksById.get(attached) : undefined) || [...tasksById.values()].find(task => task.track_ids?.includes(trackId) ||
      (source && task.provider === "jiosaavn" && task.source_track_id === source));
  },
  attach(trackId: string, taskId: string) { trackTasks.set(trackId, taskId); taskListeners.forEach(notify => notify()); },
  update(id: string, bytes: number, total?: number) {
    const previous = snapshots.get(id);
    if (!previous || !Number.isFinite(bytes)) return;
    const next = {
      bytes_downloaded: bytes,
      file_size: total || previous.file_size,
    };
    if (
      previous.bytes_downloaded === next.bytes_downloaded &&
      previous.file_size === next.file_size
    )
      return;
    snapshots.set(id, next);
    listeners.get(id)?.forEach((notify) => notify());
  },
};

export function useDownloadProgress(task: DownloadTask): DownloadTask {
  const subscribe = useCallback(
    (notify: () => void) => {
      let taskListeners = listeners.get(task.id);
      if (!taskListeners) listeners.set(task.id, (taskListeners = new Set()));
      const visible = () => {
        if (!document.hidden) notify();
      };
      taskListeners.add(visible);
      document.addEventListener("visibilitychange", visible);
      return () => {
        taskListeners.delete(visible);
        document.removeEventListener("visibilitychange", visible);
        if (!taskListeners.size) listeners.delete(task.id);
      };
    },
    [task.id],
  );
  const read = useCallback(() => snapshots.get(task.id), [task.id]);
  const progress = useSyncExternalStore(subscribe, read, read);
  return progress ? { ...task, ...progress } : task;
}

/** Each song subscribes to its transfer only; transfer ticks never reach App. */
export function useSongDownload(trackId: string, sourceId?: string) {
  const read = useCallback(() => downloadProgress.task(trackId, sourceId), [trackId, sourceId]);
  return useSyncExternalStore(useCallback((listener: () => void) => { taskListeners.add(listener); return () => { taskListeners.delete(listener); }; }, []), read, read);
}
