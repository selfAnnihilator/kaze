import { useSyncExternalStore } from "react";
import { canRateFile } from "../downloadsFolder";
import type { Track } from "../types";

/**
 * Songs from the user's own folders take no like or dislike, on PC and phone.
 * Online songs and Kaze's downloads (files in Kaze's folder) do. The backend
 * enforces the same rule; this decides which controls are shown as available.
 */
export const LOCAL_RATING_NOTE = "Songs from your own folders can't be liked or disliked";

type Rateable = Partial<Pick<Track, "file_path">>;

let downloadsDir: string | undefined;
const listeners = new Set<() => void>();

/** Called by App whenever settings report Kaze's download folder. */
export function setRatingDownloadsDir(next: string | undefined) {
  if (next === downloadsDir) return;
  downloadsDir = next;
  listeners.forEach((listener) => listener());
}

const subscribe = (listener: () => void) => {
  listeners.add(listener);
  return () => listeners.delete(listener);
};
const snapshot = () => downloadsDir;

/** Whether a song can be liked or disliked (online or a Kaze download). */
export function canRate(track: Rateable | null | undefined, dir: string | undefined = downloadsDir): boolean {
  return canRateFile(track, dir);
}

/** `canRate`, re-rendering when the download folder becomes known or changes. */
export function useCanRate(track: Rateable | null | undefined): boolean {
  return canRate(track, useSyncExternalStore(subscribe, snapshot, snapshot));
}
