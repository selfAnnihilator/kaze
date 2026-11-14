import { useCallback, useMemo } from "react";
import type { Dispatch, SetStateAction } from "react";
import { dispatchCommand } from "../../services/api";
import type { DiscoveryRecommendation } from "../../types";

type Notify = (type: "success" | "error" | "info" | "warning", title: string, message: string) => void;

/** Queue commands: enqueue local/online tracks (dedup via queuedTrackIds), play index, dequeue, clear. */
export function useQueue({ queuedTrackIds, setQueuedTrackIds, addAppNotification }: {
  queuedTrackIds: Set<string>;
  setQueuedTrackIds: Dispatch<SetStateAction<Set<string>>>;
  addAppNotification: Notify;
}) {
  const handleEnqueueTrack = useCallback(async (trackId: string) => {
    if (queuedTrackIds.has(trackId)) {
      addAppNotification(
        "info",
        "Already in Queue",
        "This song is already waiting in the queue.",
      );
      return;
    }
    try {
      await dispatchCommand({
        command: "EnqueueTrack",
        payload: { track_id: trackId, play_next: false },
      });
      setQueuedTrackIds((prev) => new Set(prev).add(trackId));
    } catch (err: any) {
      if (
        String(err?.message || err)
          .toLowerCase()
          .includes("already in the queue")
      ) {
        setQueuedTrackIds((prev) => new Set(prev).add(trackId));
        addAppNotification(
          "info",
          "Already in Queue",
          "This song is already waiting in the queue.",
        );
        return;
      }
      addAppNotification(
        "error",
        "Queue Failed",
        err?.message || "Could not add this song to the queue.",
      );
    }
  }, [queuedTrackIds, addAppNotification, setQueuedTrackIds]);

  const handleEnqueueRecommendation = useCallback(async (rec: DiscoveryRecommendation) => {
    const hasLocalMatch =
      !!rec.matched_local_track_id &&
      !rec.matched_local_track_id.startsWith("online:") &&
      !rec.matched_local_track_id.startsWith("itunes:");
    const trackId = hasLocalMatch
      ? rec.matched_local_track_id!
      : rec.external_track_id;
    if (!hasLocalMatch) {
      if (queuedTrackIds.has(trackId)) {
        addAppNotification(
          "info",
          "Already in Queue",
          "This song is already waiting in the queue.",
        );
        return;
      }
      try {
        await dispatchCommand({
          command: "EnqueueOnlineTrack",
          payload: {
            track_id: trackId,
            title: rec.title,
            artist: rec.artist,
            album: rec.album,
            duration_secs: rec.duration_secs,
            cover_art_url: rec.cover_art_url,
            preview_url: rec.preview_url,
            play_next: false,
          },
        });
        setQueuedTrackIds((prev) => new Set(prev).add(trackId));
      } catch (err: any) {
        if (
          String(err?.message || err)
            .toLowerCase()
            .includes("already in the queue")
        ) {
          setQueuedTrackIds((prev) => new Set(prev).add(trackId));
          addAppNotification(
            "info",
            "Already in Queue",
            "This song is already waiting in the queue.",
          );
        } else {
          addAppNotification(
            "error",
            "Queue Failed",
            err?.message || "Could not add this song to the queue.",
          );
        }
      }
      return;
    }
    await handleEnqueueTrack(trackId);
  }, [queuedTrackIds, addAppNotification, setQueuedTrackIds, handleEnqueueTrack]);

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
