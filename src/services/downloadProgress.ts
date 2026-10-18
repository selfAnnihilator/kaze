import { useCallback, useSyncExternalStore } from "react";
import type { DownloadTask } from "../types";

type Progress = { bytes_downloaded: number; file_size?: number };
const snapshots = new Map<string, Progress>();
const listeners = new Map<string, Set<() => void>>();

/** Transfer ticks belong to visible transfer rows, never the application shell. */
export const downloadProgress = {
  replace(tasks: readonly DownloadTask[]) {
    const present = new Set(tasks.map((task) => task.id));
    for (const id of snapshots.keys())
      if (!present.has(id)) snapshots.delete(id);
    for (const task of tasks) {
      snapshots.set(task.id, {
        bytes_downloaded: task.bytes_downloaded,
        file_size: task.file_size,
      });
      listeners.get(task.id)?.forEach((notify) => notify());
    }
  },
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
