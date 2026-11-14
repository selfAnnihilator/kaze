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
