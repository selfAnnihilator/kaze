import { isOnlineTrack } from "../../trackKind";
import { useCallback, useEffect, useMemo, useState } from "react";
import type { Dispatch, MutableRefObject, SetStateAction } from "react";
import { dispatchCommand, executeQuery } from "../../services/api";
import { viewDestination } from "../../navigationHistory";
import type { NavigationDestination } from "../../navigationHistory";
import type { DiscoveryRecommendation, PlaybackState, Playlist, Track } from "../../types";
import type { CollectionData, CollectionTrackItem } from "../../components/views/CollectionDetailView";
import type { usePlaybackControls } from "./usePlaybackControls";

type Controls = ReturnType<typeof usePlaybackControls>;

type Notify = (type: "success" | "error" | "info" | "warning", title: string, message: string) => void;
type PlayingOrigin = {
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
  const [isLoadingCollectionTracks, setIsLoadingCollectionTracks] = useState<boolean>(false);

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

  useEffect(() => {
    const collection = activeCollection;
    if (!collection || collection.tracks !== undefined) {
      setIsLoadingCollectionTracks(false);
      return;
    }
    let cancelled = false;
    const load = async () => {
      setIsLoadingCollectionTracks(true);
      try {
        if (collection.playlistId) {
          const res = await executeQuery({
            query: "GetPlaylistTracks",
            payload: { playlist_id: collection.playlistId },
          });
          const plTracks: Track[] = (res.data as any) || [];
          const items: CollectionTrackItem[] = plTracks.map((t) => {
            const isOnline = !!isOnlineTrack(t);
            return {
              id: t.id,
              title: t.title,
              artist: t.artist_name || "Unknown Artist",
              album: t.album_title,
              duration_secs: t.duration_secs ?? 0,
              cover_art_url: t.cover_art_url,
              preview_url: t.preview_url,
              is_downloaded: !isOnline,
              matched_local_track_id: isOnline ? undefined : t.id,
              rawLocalTrack: isOnline ? undefined : t,
              rawRecommendation: isOnline
                ? {
                    external_track_id: t.id,
                    provider: t.playback_source?.provider || "online",
                    provider_id: t.playback_source?.track_id || t.id,
                    playback_source: t.playback_source,
                    title: t.title,
                    artist: t.artist_name || "Unknown Artist",
                    album: t.album_title,
                    duration_secs: t.duration_secs ?? 0,
                    cover_art_url: t.cover_art_url,
                    preview_url: t.preview_url,
                    match_status: "NOT_FOUND",
                    recommendation_reason: "",
                    in_wishlist: false,
                  }
                : undefined,
            };
          });
          setActiveCollection((prev) =>
            !cancelled && prev === collection
              ? { ...prev, tracks: items }
              : prev,
          );
        } else if (collection.type === "chart" || collection.searchQuery) {
          const res = await executeQuery(
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
          const onlineRecs: DiscoveryRecommendation[] = (res.data as any) || [];
          const items: CollectionTrackItem[] = onlineRecs.map((r) => {
            const validLocalId =
              r.matched_local_track_id &&
              !r.matched_local_track_id.startsWith("online:") &&
              !r.matched_local_track_id.startsWith("itunes:")
                ? r.matched_local_track_id
                : undefined;
            return {
              id: r.external_track_id,
              title: r.title,
              artist: r.artist,
              album: r.album,
              duration_secs: r.duration_secs ?? 0,
              cover_art_url: r.cover_art_url,
              preview_url: r.preview_url,
              is_downloaded: !!validLocalId,
              matched_local_track_id: validLocalId,
              rawRecommendation: r,
            };
          });
          setActiveCollection((prev) =>
            !cancelled && prev === collection
              ? { ...prev, tracks: items }
              : prev,
          );
        }
      } catch (err) {
        console.error("Failed to load collection tracks:", err);
      } finally {
        if (!cancelled) setIsLoadingCollectionTracks(false);
      }
    };
    void load();
    return () => {
      cancelled = true;
    };
  }, [activeCollection, setActiveCollection]);

  const handlePlayDiscoveredCollection = useCallback(
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
