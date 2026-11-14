import { useCallback, useMemo } from "react";
import type { Dispatch, MutableRefObject, SetStateAction } from "react";
import { dispatchCommand } from "../../services/api";
import { viewDestination } from "../../navigationHistory";
import type { NavigationDestination } from "../../navigationHistory";
import type { DiscoveryRecommendation, PlaybackState, Playlist } from "../../types";
import type { CollectionData, CollectionTrackItem } from "../../components/views/CollectionDetailView";
import type { usePlaybackControls } from "./usePlaybackControls";
import { useCollectionTracksLoader } from "./useCollectionTracksLoader";
import { usePlayDiscoveredCollection } from "./usePlayDiscoveredCollection";

type Controls = ReturnType<typeof usePlaybackControls>;

export type Notify = (type: "success" | "error" | "info" | "warning", title: string, message: string) => void;
export type PlayingOrigin = {
  type: "playlist" | "mix" | "chart" | "album" | "discovery" | "flow";
  id?: string;
  name?: string;
  collectionData?: CollectionData;
} | null;

/**
 * Collection detail (mixes, charts, albums, playlists): open, load tracks,
 * play/shuffle/play-all, play one track, play a discovered collection, and
 * jump back to whatever is currently playing.
 */
export function useCollections({
  navigate, destinationRef, activeCollection, setActiveCollection, setPlayingPlaylistId, playlists, playingOrigin, setPlayingOrigin, playbackState, setPlaybackState, fetchPlaybackState, beginPlaybackLoading, endPlaybackLoading, queuedTrackIds, addAppNotification, handlePlayPause, handlePlayOnlineTrack, handleToggleShuffle, handleStopOnlineAudio,
}: {
  navigate: (destination: NavigationDestination) => boolean;
  destinationRef: MutableRefObject<NavigationDestination>;
  activeCollection: CollectionData | null;
  setActiveCollection: (value: SetStateAction<CollectionData | null>) => void;
  setPlayingPlaylistId: Dispatch<SetStateAction<string | null>>;
  playlists: Playlist[];
  playingOrigin: PlayingOrigin;
  setPlayingOrigin: Dispatch<SetStateAction<PlayingOrigin>>;
  playbackState: PlaybackState;
  setPlaybackState: Dispatch<SetStateAction<PlaybackState>>;
  fetchPlaybackState: () => Promise<unknown>;
  beginPlaybackLoading: (trackId: string) => number;
  endPlaybackLoading: (requestId: number) => void;
  queuedTrackIds: Set<string>;
  addAppNotification: Notify;
  handlePlayPause: Controls["handlePlayPause"];
  handlePlayOnlineTrack: Controls["handlePlayOnlineTrack"];
  handleToggleShuffle: Controls["handleToggleShuffle"];
  handleStopOnlineAudio: Controls["handleStopOnlineAudio"];
}) {

  // --- Collection Detail Handlers (Mixes, Top Charts, Playlists) ---
  const handleOpenCollection = useCallback(
    (collection: CollectionData) => {
      navigate({
        ...destinationRef.current,
        view: collection.playlistId ? "playlists" : destinationRef.current.view,
        collection,
        lyrics: false,
      });
      if (collection.playlistId || collection.type === "playlist") {
        setPlayingPlaylistId(collection.playlistId || collection.id);
      }
    },
    [navigate],
  );

  const isLoadingCollectionTracks = useCollectionTracksLoader(activeCollection, setActiveCollection);

  const handlePlayDiscoveredCollection = usePlayDiscoveredCollection({ setPlayingOrigin, fetchPlaybackState, queuedTrackIds, addAppNotification, handleStopOnlineAudio });

  const handleOpenPlayingOrigin = useCallback(() => {
    if (!playingOrigin) return;
    if (playingOrigin.type === "flow") {
      navigate(viewDestination("home"));
      return;
    }
    if (playingOrigin.collectionData) {
      handleOpenCollection(playingOrigin.collectionData);
    } else if (playingOrigin.type === "playlist" && playingOrigin.id) {
      const pl = playlists.find((p) => p.id === playingOrigin.id);
      if (pl) {
        handleOpenCollection({
          id: pl.id,
          title: pl.name,
          type: "playlist",
          tag: "PLAYLIST",
          playlistId: pl.id,
          subtitle: `${pl.track_count || 0} tracks`,
          bgGradient: "linear-gradient(135deg, #4f46e5 0%, #1e1b4b 100%)",
          accentColor: "#6366f1",
        });
      }
    }
  }, [playingOrigin, playlists, handleOpenCollection, navigate]);

  const handlePlayCollectionTrack = useCallback(
    async (item: CollectionTrackItem) => {
      if (activeCollection) {
        setPlayingOrigin({
          type: activeCollection.type,
          id: activeCollection.id,
          name: activeCollection.title,
          collectionData: activeCollection,
        });
      }

      const hasRealLocalMatch =
        item.matched_local_track_id &&
        !item.matched_local_track_id.startsWith("online:") &&
        !item.matched_local_track_id.startsWith("itunes:");

      const isCurrentlyPlaying =
        (playbackState.is_playing || playbackState.is_paused) &&
        playbackState.current_track &&
        ((hasRealLocalMatch &&
          playbackState.current_track.id === item.matched_local_track_id) ||
          playbackState.current_track.id === item.id);

      if (isCurrentlyPlaying) {
        handlePlayPause();
        return;
      }

      const enqueueCollectionRemainder = async () => {
        const collectionTracks = activeCollection?.tracks || [];
        const itemIdx = collectionTracks.findIndex(
          (track) => track.id === item.id,
        );
        const collectionIds = new Set(
          collectionTracks.map(
            (track) => track.matched_local_track_id || track.id,
          ),
        );
        for (let i = itemIdx + 1; i < collectionTracks.length; i++) {
          const queuedTrack = collectionTracks[i];
          const queuedId = queuedTrack.matched_local_track_id || queuedTrack.id;
          const isOnlineQueued =
            !queuedTrack.matched_local_track_id ||
            queuedTrack.matched_local_track_id.startsWith("online:") ||
            queuedTrack.matched_local_track_id.startsWith("itunes:");
          if (isOnlineQueued) {
            try {
              await dispatchCommand({
                command: "EnqueueOnlineTrack",
                payload: {
                  track_id: queuedId,
                  title: queuedTrack.title,
                  artist: queuedTrack.artist,
                  album: queuedTrack.album,
                  duration_secs: queuedTrack.duration_secs,
                  cover_art_url: queuedTrack.cover_art_url,
                  preview_url: queuedTrack.preview_url,
                  play_next: false,
                },
              });
            } catch (err) {
              console.warn("Could not enqueue online collection track:", err);
            }
          } else {
            try {
              await dispatchCommand({
                command: "EnqueueTrack",
                payload: { track_id: queuedId, play_next: false },
              });
            } catch (err) {
              console.warn("Could not enqueue collection track:", err);
            }
          }
        }
        for (const queuedId of queuedTrackIds) {
          if (collectionIds.has(queuedId)) continue;
          try {
            await dispatchCommand({
              command: "EnqueueTrack",
              payload: { track_id: queuedId, play_next: false },
            });
          } catch (err) {
            console.warn(
              "Could not preserve queued track after collection:",
              err,
            );
          }
        }
      };

      if (hasRealLocalMatch) {
        const requestId = beginPlaybackLoading(item.matched_local_track_id!);
        try {
          await dispatchCommand({
            command: "PlayTrack",
            payload: {
              track_id: item.matched_local_track_id!,
              source: "collection",
            },
          });
          await fetchPlaybackState();
        } finally {
          endPlaybackLoading(requestId);
        }
        await enqueueCollectionRemainder();
        return;
      }

      // Offline check: Cannot stream online audio without an internet connection
      if (!item.id.startsWith("jiosaavn:") && typeof navigator !== "undefined" && !navigator.onLine) {
        addAppNotification(
          "warning",
          "No Internet Connection",
          `Cannot play "${item.title}". Connect to the internet to stream online songs.`,
        );
        return;
      }

      if (item.rawRecommendation) {
        await handlePlayOnlineTrack(
          item.rawRecommendation,
          false,
          "collection",
        );
        await enqueueCollectionRemainder();
        return;
      }

      const rec: DiscoveryRecommendation = {
        external_track_id: item.id,
        provider: "online",
        provider_id: item.id,
        title: item.title,
        artist: item.artist,
        album: item.album,
        duration_secs: item.duration_secs,
        cover_art_url: item.cover_art_url,
        preview_url: item.preview_url,
        match_status: hasRealLocalMatch ? "EXACT_MATCH" : "NOT_FOUND",
        matched_local_track_id: hasRealLocalMatch
          ? item.matched_local_track_id
          : undefined,
        recommendation_reason: "From collection",
        in_wishlist: false,
      };
      await handlePlayOnlineTrack(rec, false, "collection");
      await enqueueCollectionRemainder();
    },
    [
      playbackState.is_playing,
      playbackState.is_paused,
      playbackState.current_track,
      activeCollection,
      queuedTrackIds,
      handlePlayPause,
      handlePlayOnlineTrack,
      fetchPlaybackState,
      addAppNotification,
      beginPlaybackLoading,
      endPlaybackLoading,
    ],
  );

  const isPlayingThisCollection = useMemo(() => {
    if (
      !activeCollection ||
      !activeCollection.tracks ||
      activeCollection.tracks.length === 0
    ) {
      return false;
    }
    const currentId = playbackState.is_playing
      ? playbackState.current_track?.id
      : null;
    if (!currentId) return false;

    return activeCollection.tracks.some(
      (t) => t.id === currentId || t.matched_local_track_id === currentId,
    );
  }, [
    activeCollection,
    playbackState.is_playing,
    playbackState.current_track?.id,
  ]);

  const isCollectionSaved = useMemo(() => {
    if (!activeCollection) return false;
    if (activeCollection.type === "playlist" || activeCollection.playlistId)
      return true;
    return playlists.some(
      (p) =>
        p.is_smart_mix !== 1 &&
        (p.id === activeCollection.id ||
          p.id === activeCollection.playlistId ||
          p.name === activeCollection.title.trim()),
    );
  }, [activeCollection, playlists]);

  const handlePlayAllCollection = useCallback(async () => {
    if (
      !activeCollection ||
      !activeCollection.tracks ||
      activeCollection.tracks.length === 0
    ) {
      return;
    }

    if (isPlayingThisCollection) {
      handlePlayPause();
      return;
    }

    await handlePlayCollectionTrack(activeCollection.tracks[0]);
  }, [
    activeCollection,
    isPlayingThisCollection,
    handlePlayPause,
    handlePlayCollectionTrack,
  ]);

  const handleShuffleCollection = useCallback(async () => {
    if (
      !activeCollection ||
      !activeCollection.tracks ||
      activeCollection.tracks.length === 0
    ) {
      return;
    }

    // If shuffle is currently active, clicking it deactivates shuffle and continues with normal queue
    if (playbackState.is_shuffled) {
      await handleToggleShuffle();
      return;
    }

    // If this collection is already playing, simply activate shuffle
    if (isPlayingThisCollection) {
      await handleToggleShuffle();
      return;
    }

    // Otherwise, start playing this collection in shuffle mode
    const tracks = activeCollection.tracks;
    const localTracks = tracks.filter(
      (t) =>
        !!t.matched_local_track_id &&
        !t.matched_local_track_id.startsWith("online:") &&
        !t.matched_local_track_id.startsWith("itunes:"),
    );

    if (localTracks.length > 0) {
      handleStopOnlineAudio();
      // First enqueue tracks in their natural sequential order into the queue
      await dispatchCommand({ command: "ClearQueue" });
      await dispatchCommand({
        command: "PlayTrack",
        payload: {
          track_id: localTracks[0].matched_local_track_id!,
          source: "collection",
        },
      });
      for (let i = 1; i < localTracks.length; i++) {
        const tid = localTracks[i].matched_local_track_id;
        if (tid && !tid.startsWith("online:") && !tid.startsWith("itunes:")) {
          await dispatchCommand({
            command: "EnqueueTrack",
            payload: { track_id: tid, play_next: false },
          });
        }
      }

      // Pick a random track index to start with
      const randIdx = Math.floor(Math.random() * localTracks.length);
      if (randIdx !== 0) {
        await dispatchCommand({
          command: "PlayQueueIndex",
          payload: { index: randIdx },
        });
      }

      // Activate shuffle mode
      await dispatchCommand({
        command: "SetShuffle",
        payload: { enabled: true },
      });
      setPlaybackState((prev) => ({ ...prev, is_shuffled: true }));
      fetchPlaybackState();
    } else {
      // Online collection: pick a random track to start and activate shuffle
      const randIdx = Math.floor(Math.random() * tracks.length);
      const chosenTrack = tracks[randIdx];
      await handlePlayCollectionTrack(chosenTrack);
      await dispatchCommand({
        command: "SetShuffle",
        payload: { enabled: true },
      });
      setPlaybackState((prev) => ({ ...prev, is_shuffled: true }));
    }
  }, [
    activeCollection,
    playbackState.is_shuffled,
    isPlayingThisCollection,
    handleToggleShuffle,
    handleStopOnlineAudio,
    handlePlayCollectionTrack,
    fetchPlaybackState,
  ]);

  return useMemo(() => ({
    isLoadingCollectionTracks, handleOpenCollection, handlePlayDiscoveredCollection, handleOpenPlayingOrigin, handlePlayCollectionTrack, isPlayingThisCollection, isCollectionSaved, handlePlayAllCollection, handleShuffleCollection,
  }), [isLoadingCollectionTracks, handleOpenCollection, handlePlayDiscoveredCollection, handleOpenPlayingOrigin, handlePlayCollectionTrack, isPlayingThisCollection, isCollectionSaved, handlePlayAllCollection, handleShuffleCollection]);
}
