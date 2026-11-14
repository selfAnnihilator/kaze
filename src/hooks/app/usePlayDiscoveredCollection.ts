import { useCallback } from "react";
import type { Dispatch, SetStateAction } from "react";
import { dispatchCommand, executeQuery } from "../../services/api";
import type { DiscoveryRecommendation } from "../../types";
import type { CollectionData } from "../../components/views/CollectionDetailView";
import type { usePlaybackControls } from "./usePlaybackControls";
import type { Notify, PlayingOrigin } from "./useCollections";

/** Plays a chart or searched collection found in Discover: resolve its songs, then queue them. */
export function usePlayDiscoveredCollection({ setPlayingOrigin, fetchPlaybackState, queuedTrackIds, addAppNotification, handleStopOnlineAudio }: {
  setPlayingOrigin: Dispatch<SetStateAction<PlayingOrigin>>;
  fetchPlaybackState: () => Promise<unknown>;
  queuedTrackIds: Set<string>;
  addAppNotification: Notify;
  handleStopOnlineAudio: ReturnType<typeof usePlaybackControls>["handleStopOnlineAudio"];
}) {
  return useCallback(
    async (collection: CollectionData) => {
      if (collection.type !== "chart" && !collection.searchQuery) return;
      if (typeof navigator !== "undefined" && !navigator.onLine) {
        addAppNotification(
          "warning",
          "No Internet Connection",
          `Cannot load ${collection.title}. Connect to the internet to stream this collection.`,
        );
        return;
      }

      try {
        const response = await executeQuery(
          collection.type === "chart"
            ? {
                query: "GetChartSongs",
                payload: { chart_id: collection.id, limit: 50 },
              }
            : {
                query: "SearchOnlineMusic",
                payload: { query: collection.searchQuery!, limit: 50 },
              },
        );
        const results = (
          (response?.data as DiscoveryRecommendation[]) || []
        ).filter(
          (track, index, all) =>
            all.findIndex(
              (candidate) =>
                candidate.external_track_id === track.external_track_id,
            ) === index,
        );
        if (results.length === 0) {
          addAppNotification(
            "warning",
            "No Songs Found",
            `${collection.title} did not return any playable songs.`,
          );
          return;
        }

        const localIdFor = (track: DiscoveryRecommendation) => {
          const localId = track.matched_local_track_id;
          return localId &&
            !localId.startsWith("online:") &&
            !localId.startsWith("itunes:")
            ? localId
            : null;
        };
        const resolvedId = (track: DiscoveryRecommendation) =>
          localIdFor(track) || track.external_track_id;
        const isLocal = (track: DiscoveryRecommendation) =>
          localIdFor(track) !== null;
        const collectionIds = new Set(results.map(resolvedId));
        const preservedQueue = Array.from(queuedTrackIds).filter(
          (trackId) => !collectionIds.has(trackId),
        );
        const first = results[0];
        const firstId = resolvedId(first);

        setPlayingOrigin({
          type: collection.type === "chart" ? "chart" : "mix",
          id: collection.id,
          name: collection.title,
          collectionData: {
            ...collection,
            tracks: results.map((track) => ({
              id: track.external_track_id,
              title: track.title,
              artist: track.artist,
              album: track.album,
              duration_secs: track.duration_secs ?? 0,
              cover_art_url: track.cover_art_url,
              preview_url: track.preview_url,
              is_downloaded: isLocal(track),
              matched_local_track_id: isLocal(track)
                ? resolvedId(track)
                : undefined,
              rawRecommendation: track,
            })),
          },
        });

        if (!isLocal(first)) {
          try {
            await dispatchCommand({
              command: "EnqueueOnlineTrack",
              payload: {
                track_id: firstId,
                title: first.title,
                artist: first.artist,
                album: first.album,
                duration_secs: first.duration_secs,
                cover_art_url: first.cover_art_url,
                preview_url: first.preview_url,
                play_next: false,
              },
            });
          } catch (error: any) {
            if (
              !String(error?.message || error)
                .toLowerCase()
                .includes("already in the queue")
            ) {
              throw error;
            }
          }
        }

        await dispatchCommand({
          command: "PlayTrack",
          payload: { track_id: firstId, source: "collection" },
        });

        for (const track of results.slice(1)) {
          const trackId = resolvedId(track);
          try {
            if (isLocal(track)) {
              await dispatchCommand({
                command: "EnqueueTrack",
                payload: { track_id: trackId, play_next: false },
              });
            } else {
              await dispatchCommand({
                command: "EnqueueOnlineTrack",
                payload: {
                  track_id: trackId,
                  title: track.title,
                  artist: track.artist,
                  album: track.album,
                  duration_secs: track.duration_secs,
                  cover_art_url: track.cover_art_url,
                  preview_url: track.preview_url,
                  play_next: false,
                },
              });
            }
          } catch (error) {
            console.warn("Could not enqueue chart track:", error);
          }
        }

        for (const trackId of preservedQueue) {
          try {
            await dispatchCommand({
              command: "EnqueueTrack",
              payload: { track_id: trackId, play_next: false },
            });
          } catch (error) {
            console.warn("Could not preserve queued track after chart:", error);
          }
        }

        fetchPlaybackState();
      } catch (error: any) {
        console.error("Failed to play discovered collection:", error);
        addAppNotification(
          "error",
          "Collection Playback Failed",
          error?.message || `Could not play ${collection.title}.`,
        );
      }
    },
    [
      queuedTrackIds,
      addAppNotification,
      handleStopOnlineAudio,
      fetchPlaybackState,
    ],
  );
}
