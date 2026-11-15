import { useCallback, useRef, useState } from "react";
import type { MutableRefObject } from "react";
import { executeQuery } from "../../services/api";
import { playbackProgress } from "../../services/playbackProgress";
import type { PlaybackState, Track } from "../../types";
import { upcomingFromPlayback, withKnownMetadata, type QueueEntry } from "../../presentation/queuePresentation";

const INITIAL_PLAYBACK: PlaybackState = {
  is_playing: false,
  position_secs: 0,
  duration_secs: 0,
  volume: 0.8,
  is_muted: false,
  repeat_mode: "off",
  is_shuffled: false,
};

/** Applies a GetPlaybackState response; the song falls back to a loaded library row. */
function mergePlaybackResponse(prev: PlaybackState, data: any, tracks: Track[]): PlaybackState {
  const updated: PlaybackState = { ...prev, ...data, duration_secs: data.duration_secs ?? 0, current_track: undefined,
    is_shuffled: data.is_shuffle ?? data.is_shuffled ?? prev.is_shuffled,
  };
  if (data.current_track) {
    updated.current_track = data.current_track;
  } else if (data.current_track_id) {
    const found = tracks.find((t) => t.id === data.current_track_id);
    if (found) updated.current_track = found;
  }
  return updated;
}

/** Resolves a pending placeholder for the same song, keeping everything else. */
function resolvePending(prev: PlaybackState, resolved: Track): PlaybackState {
  return prev.current_track?.pending && prev.current_track.id === resolved.id ? { ...prev, current_track: resolved } : prev;
}

/**
 * The song played before the current one (ignoring buffering placeholders).
 * The first song of the queue has none, as Previous has nothing to go back to.
 */
function usePreviousTrack(playbackState: PlaybackState) {
  const previousTrackRef = useRef<Track | undefined>(undefined);
  const observedTrackRef = useRef<Track | undefined>(undefined);
  const current = playbackState.current_track;
  if (!playbackState.is_buffering && current && current.id !== observedTrackRef.current?.id) {
    previousTrackRef.current = observedTrackRef.current;
    observedTrackRef.current = current;
  }
  // Shown separately, so the remembered song survives updates that arrive in either order.
  const shownRef = useRef<Track | undefined>(undefined);
  shownRef.current = (playbackState.current_queue_index ?? 0) > 0 ? previousTrackRef.current : undefined;
  return shownRef;
}

/**
 * The transport state mirrored from the backend: now playing, the loading
 * marker, the queue, and the previous song. Position ticks go through
 * `playbackProgress`, never through this state.
 */
export function usePlaybackState({ libraryAccountRef, tracksRef }: {
  libraryAccountRef: MutableRefObject<string>;
  tracksRef: MutableRefObject<Track[]>;
}) {
  const [playbackState, setPlaybackState] = useState<PlaybackState>(INITIAL_PLAYBACK);
  const [loadingTrackId, setLoadingTrackId] = useState<string | null>(null);
  const [queuedTrackIds, setQueuedTrackIds] = useState<Set<string>>(new Set());
  const [queueEntries, setQueueEntries] = useState<QueueEntry[]>([]);
  const playbackRequestId = useRef(0);
  const playbackSessionId = useRef<string | null>(null);
  const playbackStateRevision = useRef(0);
  const playbackStateQueryId = useRef(0);

  const beginPlaybackLoading = useCallback((trackId: string) => {
    ++playbackStateRevision.current;
    const requestId = ++playbackRequestId.current;
    setLoadingTrackId(trackId);
    return requestId;
  }, []);
  const endPlaybackLoading = useCallback((requestId: number) => {
    if (playbackRequestId.current === requestId) setLoadingTrackId(null);
  }, []);

  const previousTrackRef = usePreviousTrack(playbackState);

  const fetchPlaybackState = useCallback(async () => {
    const account = libraryAccountRef.current;
    const revision = playbackStateRevision.current;
    const queryId = ++playbackStateQueryId.current;
    try {
      const res = await executeQuery({ query: "GetPlaybackState" });
      // Source/catalog work can delay this response beyond a newer transport
      // event. Never reinstate the old song or its buffering/paused state.
      if (account !== libraryAccountRef.current || queryId !== playbackStateQueryId.current) return;
      if (revision !== playbackStateRevision.current) {
        // A newer transport event won: keep its play/pause/buffering state, but
        // still resolve a placeholder for the same song, or it stays pending
        // (greyed-out controls) until the next manual refresh.
        const resolved: Track | undefined = res.data?.current_track;
        if (resolved) setPlaybackState((prev) => resolvePending(prev, resolved));
        return;
      }
      if (res.data) {
        playbackProgress.update(res.data.position_secs ?? 0, res.data.duration_secs ?? 0);
        setPlaybackState((prev) => mergePlaybackResponse(prev, res.data, tracksRef.current));
        if (Array.isArray(res.data.queue_track_ids)) {
          setQueuedTrackIds(new Set(res.data.user_queue_track_ids ?? []));
          setQueueEntries((prev) => withKnownMetadata(upcomingFromPlayback(res.data), prev));
        }
      }
    } catch (err) {
      console.error("Failed to fetch playback state:", err);
    }
  }, []);

  return {
    playbackState, setPlaybackState, loadingTrackId, setLoadingTrackId,
    queuedTrackIds, setQueuedTrackIds, queueEntries, setQueueEntries,
    playbackSessionId, playbackStateRevision, previousTrackRef,
    beginPlaybackLoading, endPlaybackLoading, fetchPlaybackState,
  };
}
