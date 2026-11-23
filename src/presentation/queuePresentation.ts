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

/**
 * A read-only adapter, never a second queue or transport state machine.
 * `order` (the backend's `upcoming_indices`) lists Up Next in play order,
 * which differs from queue order while shuffled; older backends omit it.
 */
export function upcomingFromEvent(
  items: BackendQueueItem[],
  currentIndex: number | null,
  order?: number[],
): QueueEntry[] {
  const start = currentIndex == null ? 0 : currentIndex + 1;
  const indices = order ?? items.map((_, i) => i).slice(start);
  return indices.flatMap((index) => {
    const item = items[index];
    return item ? [{ key: item.queue_id, trackId: item.track_id, index, title: item.title, artist: item.artist }] : [];
  });
}

export function upcomingFromPlayback(state: PlaybackState): QueueEntry[] {
  // A stopped snapshot returns the complete queue even if its old pointer remains.
  const stopped = "current_track_id" in state && state.current_track_id == null;
  const start =
    state.current_queue_index == null || stopped
      ? 0
      : state.current_queue_index + 1;
  const order = state.upcoming_indices;
  return (state.queue_track_ids || []).map((trackId, offset) => {
    const index = order?.[offset] ?? start + offset;
    return { key: `${index}:${trackId}`, trackId, index };
  });
}

/**
 * Playback snapshots carry only track ids; keep titles/artists learned from
 * earlier QueueUpdated events so online/Flow entries never fall back to a
 * placeholder after pause or other state refreshes.
 */
export function withKnownMetadata(
  next: QueueEntry[],
  prev: readonly QueueEntry[],
): QueueEntry[] {
  if (prev.length === 0) return next;
  const known = new Map<string, QueueEntry>();
  for (const entry of prev) {
    if (entry.title) known.set(entry.trackId, entry);
  }
  if (known.size === 0) return next;
  return next.map((entry) => {
    if (entry.title) return entry;
    const hit = known.get(entry.trackId);
    return hit ? { ...entry, title: hit.title, artist: entry.artist || hit.artist } : entry;
  });
}
