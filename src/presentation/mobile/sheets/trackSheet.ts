import { useSyncExternalStore } from "react";
import type { DiscoveryRecommendation } from "../../../types";

/** The song a sheet is about. */
export interface SheetTrack {
  id: string;
  title: string;
  artist?: string;
  album?: string;
  cover?: string | null;
  /** Online (JioSaavn) source id: the song can be downloaded. */
  sourceId?: string;
  online?: boolean;
  /** From the user's own folders: takes no like or dislike. */
  localFile?: boolean;
  /** Set for online songs: queueing goes through the recommendation. */
  rec?: DiscoveryRecommendation;
}

export type TrackSheetKind = "actions" | "not_interested";
interface OpenSheet { kind: TrackSheetKind; track: SheetTrack }

let open: OpenSheet | null = null;
const listeners = new Set<() => void>();
const emit = () => listeners.forEach((l) => l());

/** Opens Track Actions (G1) or Not Interested (G3) from any row, with no App state. */
export function openTrackSheet(kind: TrackSheetKind, track: SheetTrack) {
  open = { kind, track };
  emit();
}
export function closeTrackSheet() {
  open = null;
  emit();
}
const subscribe = (l: () => void) => { listeners.add(l); return () => { listeners.delete(l); }; };
const snapshot = () => open;
export const useTrackSheet = () => useSyncExternalStore(subscribe, snapshot, snapshot);
