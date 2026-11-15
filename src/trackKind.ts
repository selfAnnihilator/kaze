import type { Track } from "./types";

type TrackLike = Pick<Track, "id" | "file_path" | "format"> & { pending?: boolean };

/**
 * The single rule for "is this an online (not local) track".
 * Returns `undefined` while the track is a pending placeholder, so callers never
 * guess and flicker between "Download" and "Downloaded".
 */
export function isOnlineTrack(track: TrackLike | null | undefined): boolean | undefined {
  if (!track || track.pending) return undefined;
  return (
    track.format === "online" ||
    !!track.file_path?.startsWith("online://") ||
    track.id.startsWith("itunes:") ||
    track.id.startsWith("online:")
  );
}

type RecommendationLike = { external_track_id: string; matched_local_track_id?: string | null };

/** The id a recommendation is queued under: its local copy when there is one, else its online id. */
export function queueIdOf(rec: RecommendationLike): string {
  const local = rec.matched_local_track_id;
  return local && !local.startsWith("online:") && !local.startsWith("itunes:") ? local : rec.external_track_id;
}
