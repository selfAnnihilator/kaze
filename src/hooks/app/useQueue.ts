import { useCallback, useMemo } from "react";
import type { Dispatch, SetStateAction } from "react";
import { dispatchCommand } from "../../services/api";
import type { DiscoveryRecommendation } from "../../types";
import { queueIdOf } from "../../trackKind";

type Notify = (type: "success" | "error" | "info" | "warning", title: string, message: string) => void;

const ALREADY = ["Already in Queue", "This song is already waiting in the queue."] as const;

/** Queue commands: enqueue local/online tracks (dedup via queuedTrackIds), play index, dequeue, clear. */
export function useQueue({ queuedTrackIds, setQueuedTrackIds, addAppNotification }: {
  queuedTrackIds: Set<string>;
  setQueuedTrackIds: Dispatch<SetStateAction<Set<string>>>;
  addAppNotification: Notify;
}) {
  /** Sends one enqueue command; true when the song was added. */
  const enqueue = useCallback(async (trackId: string, command: Parameters<typeof dispatchCommand>[0]): Promise<boolean> => {
    if (queuedTrackIds.has(trackId)) {
      addAppNotification("info", ...ALREADY);
      return false;
    }
    try {
      await dispatchCommand(command);
      setQueuedTrackIds((prev) => new Set(prev).add(trackId));
      return true;
    } catch (err: any) {
      const message = String(err?.message || err);
      if (message.toLowerCase().includes("already in the queue")) {
        setQueuedTrackIds((prev) => new Set(prev).add(trackId));
        addAppNotification("info", ...ALREADY);
      } else {
        addAppNotification("error", "Queue Failed", err?.message || "Could not add this song to the queue.");
      }
      return false;
    }
  }, [queuedTrackIds, addAppNotification, setQueuedTrackIds]);

  const handleEnqueueTrack = useCallback((trackId: string, playNext = false) =>
    enqueue(trackId, { command: "EnqueueTrack", payload: { track_id: trackId, play_next: playNext, manual: true } }), [enqueue]);

  const handleEnqueueRecommendation = useCallback((rec: DiscoveryRecommendation) => {
    const trackId = queueIdOf(rec);
    if (trackId === rec.matched_local_track_id) return handleEnqueueTrack(trackId);
    return enqueue(trackId, {
      command: "EnqueueOnlineTrack",
      payload: {
        track_id: trackId, title: rec.title, artist: rec.artist, album: rec.album, duration_secs: rec.duration_secs,
        cover_art_url: rec.cover_art_url, preview_url: rec.preview_url, play_next: false, manual: true,
      },
    });
  }, [enqueue, handleEnqueueTrack]);

  const runQueueCommand = useCallback(
    async (command: Parameters<typeof dispatchCommand>[0]) => {
      try {
        await dispatchCommand(command);
      } catch (error) {
        addAppNotification("error", "Queue action failed", String(error));
      }
    },
    [addAppNotification],
  );
  const handlePlayQueueIndex = useCallback(
    (index: number) => {
      void runQueueCommand({ command: "PlayQueueIndex", payload: { index } });
    },
    [runQueueCommand],
  );
  const handleDequeueTrack = useCallback(
    (track_id: string) => {
      void runQueueCommand({ command: "DequeueTrack", payload: { track_id } });
    },
    [runQueueCommand],
  );
  const handleClearQueue = useCallback(() => {
    void runQueueCommand({ command: "ClearQueue" });
  }, [runQueueCommand]);

  return useMemo(() => ({
    handleEnqueueTrack, handleEnqueueRecommendation, handlePlayQueueIndex, handleDequeueTrack, handleClearQueue,
  }), [handleEnqueueTrack, handleEnqueueRecommendation, handlePlayQueueIndex, handleDequeueTrack, handleClearQueue]);
}
