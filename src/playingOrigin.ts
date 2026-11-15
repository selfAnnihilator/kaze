/** Where the playing song came from, and whether the player may link back to it. */
import type { CollectionData } from "./components/views/collection/types";
import { inSection } from "./downloadsFolder";
import { isOnlineTrack } from "./trackKind";
import type { Track } from "./types";

export type PlayingOrigin = {
  type: "playlist" | "mix" | "chart" | "album" | "discovery" | "flow" | "library";
  id?: string;
  name?: string;
  collectionData?: CollectionData;
  /** Library origins: the Local or Downloads list. */
  scope?: "local" | "downloads";
  /** Songs of a playlist started without its collection data. */
  trackIds?: string[];
} | null;

type CurrentTrack = Pick<Track, "id" | "file_path" | "format"> | null | undefined;

/**
 * The origin, when the current song belongs to a collection the user pressed
 * play in. Songs from Flow, search, Discover or the queue get no link, even
 * while an older origin is still remembered.
 */
export function linkableOrigin(origin: PlayingOrigin, track: CurrentTrack, downloadsDir?: string): PlayingOrigin {
  if (!origin || !track) return null;
  if (origin.type === "library") {
    return isOnlineTrack(track) === false && inSection(track, downloadsDir, origin.scope ?? "local") ? origin : null;
  }
  if (origin.type !== "playlist" && origin.type !== "mix" && origin.type !== "chart") return null;
  const inTracks = origin.collectionData?.tracks?.some((item) => item.id === track.id || item.matched_local_track_id === track.id);
  return inTracks || origin.trackIds?.includes(track.id) ? origin : null;
}
