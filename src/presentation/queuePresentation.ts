import type { PlaybackState, Track } from "../types";

export type PlaybackArtworkTrack = Pick<
  Track,
  "id" | "title" | "artist_name" | "album_title" | "cover_art_url"
>;

export interface QueueEntry {
  key: string;
  trackId: string;
  index: number;
  title?: string;
  artist?: string;
}
export interface BackendQueueItem {
  queue_id: string;
  track_id: string;
  title: string;
  artist: string;
}

/** A read-only adapter, never a second queue or transport state machine. */
export function upcomingFromEvent(
  items: BackendQueueItem[],
  currentIndex: number | null,
): QueueEntry[] {
  const start = currentIndex == null ? 0 : currentIndex + 1;
  return items.slice(start).map((item, offset) => ({
    key: item.queue_id,
    trackId: item.track_id,
    index: start + offset,
    title: item.title,
    artist: item.artist,
  }));
}

export function upcomingFromPlayback(state: PlaybackState): QueueEntry[] {
  // A stopped snapshot returns the complete queue even if its old pointer remains.
  const stopped = "current_track_id" in state && state.current_track_id == null;
  const start =
    state.current_queue_index == null || stopped
      ? 0
      : state.current_queue_index + 1;
  return (state.queue_track_ids || []).map((trackId, offset) => ({
    key: `${start + offset}:${trackId}`,
    trackId,
    index: start + offset,
  }));
}

export function queueTrack(
  entry: QueueEntry,
  tracks: readonly Track[],
): Track | undefined {
  return tracks.find((track) => track.id === entry.trackId);
}
