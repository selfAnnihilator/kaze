import type { Dispatch, SetStateAction } from "react";
import type { CollectionData, CollectionTrackItem } from "../../components/views/CollectionDetailView";
import { dispatchCommand, executeQuery } from "../../services/api";
import { isOnlineTrack } from "../../trackKind";
import type { PlaybackState, Playlist, Track } from "../../types";

interface LikesDeps {
  activeCollection: CollectionData | null;
  setActiveCollection: (value: SetStateAction<CollectionData | null>) => void;
  addAppNotification: (type: "success" | "error" | "info" | "warning", title: string, message: string) => void;
  fetchHomeOverview: () => unknown;
  fetchPlaylists: () => unknown;
  fetchTrackPlaylistMemberships: () => unknown;
  setPlaybackState: Dispatch<SetStateAction<PlaybackState>>;
  setTrackPlaylistMap: Dispatch<SetStateAction<Record<string, string[]>>>;
  setTracks: (update: SetStateAction<Track[]>) => void;
  trackPlaylistMap: Record<string, string[]>;
  tracks: Track[];
  playlists: Playlist[];
}

/**
 * Liking and un-liking songs: local tracks, online songs and whole collections.
 * Handlers are recreated each render, as they were inline in App.
 */
export function useLikes({ activeCollection, setActiveCollection, addAppNotification, fetchHomeOverview, fetchPlaylists, fetchTrackPlaylistMemberships, setPlaybackState, setTrackPlaylistMap, setTracks, trackPlaylistMap, tracks, playlists }: LikesDeps) {
  const likedPlaylistId = playlists.find(
    (playlist) => playlist.name === "Liked Songs",
  )?.id;

  const handleLike = async (trackId: string) => {
    setTracks((prev) =>
      prev.map((t) => (t.id === trackId ? { ...t, manual_like: 1 } : t)),
    );
    setPlaybackState((prev) =>
      prev.current_track?.id === trackId
        ? { ...prev, current_track: { ...prev.current_track, manual_like: 1 } }
        : prev,
    );
    try {
      await dispatchCommand({
        command: "LikeTrack",
        payload: { track_id: trackId },
      });
      await Promise.all([
        fetchPlaylists(),
        fetchTrackPlaylistMemberships(),
        fetchHomeOverview(),
      ]);
      // If Liked Songs is the active collection, refresh its tracks
      if (
        activeCollection &&
        (activeCollection.title === "Liked Songs" ||
          (likedPlaylistId &&
            (activeCollection.playlistId === likedPlaylistId ||
              activeCollection.id === likedPlaylistId)))
      ) {
        const plId =
          activeCollection.playlistId || activeCollection.id || likedPlaylistId;
        if (plId) {
          const res = await executeQuery({
            query: "GetPlaylistTracks",
            payload: { playlist_id: plId },
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
            };
          });
          setActiveCollection((prev) =>
            prev
              ? { ...prev, tracks: items, subtitle: `${items.length} tracks` }
              : prev,
          );
        }
      }
    } catch (err: any) {
      console.error("Failed to like track:", err);
      setTracks((prev) =>
        prev.map((t) => (t.id === trackId ? { ...t, manual_like: 0 } : t)),
      );
      setPlaybackState((prev) =>
        prev.current_track?.id === trackId
          ? {
              ...prev,
              current_track: { ...prev.current_track, manual_like: 0 },
            }
          : prev,
      );
      addAppNotification(
        "error",
        "Like Failed",
        err?.message || "Could not like track.",
      );
    }
  };

  const handleRemoveFeedback = async (trackId: string) => {
    setTracks((prev) =>
      prev.map((t) => (t.id === trackId ? { ...t, manual_like: 0 } : t)),
    );
    setPlaybackState((prev) =>
      prev.current_track?.id === trackId
        ? { ...prev, current_track: { ...prev.current_track, manual_like: 0 } }
        : prev,
    );
    try {
      await dispatchCommand({
        command: "RemoveTrackFeedback",
        payload: { track_id: trackId },
      });
      await Promise.all([
        fetchPlaylists(),
        fetchTrackPlaylistMemberships(),
        fetchHomeOverview(),
      ]);
      // If Liked Songs is the active collection, remove the unliked track immediately
      setActiveCollection((prev) => {
        if (!prev || !prev.tracks) return prev;
        const isLikedSongs =
          prev.title === "Liked Songs" ||
          (likedPlaylistId &&
            (prev.playlistId === likedPlaylistId ||
              prev.id === likedPlaylistId));
        if (!isLikedSongs) return prev;
        const updatedTracks = prev.tracks.filter(
          (t) => t.id !== trackId && t.matched_local_track_id !== trackId,
        );
        return {
          ...prev,
          tracks: updatedTracks,
          subtitle: `${updatedTracks.length} tracks`,
        };
      });
    } catch (err: any) {
      console.error("Failed to remove feedback:", err);
      setTracks((prev) =>
        prev.map((t) => (t.id === trackId ? { ...t, manual_like: 1 } : t)),
      );
      setPlaybackState((prev) =>
        prev.current_track?.id === trackId
          ? {
              ...prev,
              current_track: { ...prev.current_track, manual_like: 1 },
            }
          : prev,
      );
      addAppNotification(
        "error",
        "Action Failed",
        err?.message || "Could not update track rating.",
      );
    }
  };

  const handleLikeOnline = async (track: {
    id: string;
    title: string;
    artist: string;
    album?: string;
    cover_art_url?: string;
    preview_url?: string;
    duration_secs?: number;
  }) => {
    try {
      const res = await dispatchCommand({
        command: "EnsureLikedSongsPlaylist",
      });
      const likedId = (res as any)?.data;
      if (likedId) {
        const isLiked = (trackPlaylistMap[track.id] || []).includes(likedId);
        if (isLiked) {
          await dispatchCommand({
            command: "RemoveTrackFromPlaylist",
            payload: { playlist_id: likedId, track_id: track.id },
          });
          setTrackPlaylistMap((prev) => {
            const next = { ...prev };
            if (next[track.id]) {
              next[track.id] = next[track.id].filter((id) => id !== likedId);
            }
            return next;
          });
          setActiveCollection((prev) => {
            if (!prev || !prev.tracks) return prev;
            const isLikedSongs =
              prev.title === "Liked Songs" ||
              prev.playlistId === likedId ||
              prev.id === likedId;
            if (!isLikedSongs) return prev;
            const updatedTracks = prev.tracks.filter((t) => t.id !== track.id);
            return {
              ...prev,
              tracks: updatedTracks,
              subtitle: `${updatedTracks.length} tracks`,
            };
          });
        } else {
          await dispatchCommand({
            command: "AddTrackToPlaylist",
            payload: {
              playlist_id: likedId,
              track_id: track.id,
              title: track.title,
              artist: track.artist,
              album: track.album,
              duration_secs: track.duration_secs,
              cover_art_url: track.cover_art_url,
              preview_url: track.preview_url,
            },
          });
          setTrackPlaylistMap((prev) => {
            const next = { ...prev };
            const existing = next[track.id] || [];
            if (!existing.includes(likedId)) {
              next[track.id] = [...existing, likedId];
            }
            return next;
          });
        }
        await fetchPlaylists();
        await fetchTrackPlaylistMemberships();
      }
    } catch (err: any) {
      console.error("Failed to toggle online like:", err);
      addAppNotification(
        "error",
        "Like Failed",
        err?.message || "Could not update online track like.",
      );
    }
  };

  const handleToggleCollectionLike = async (
    track: CollectionTrackItem,
    isLiked: boolean,
  ) => {
    const localId =
      (track.matched_local_track_id &&
      !track.matched_local_track_id.startsWith("online:") &&
      !track.matched_local_track_id.startsWith("itunes:")
        ? track.matched_local_track_id
        : undefined) ||
      (!track.id.startsWith("online:") &&
      !track.id.startsWith("itunes:") &&
      tracks.some((t) => t.id === track.id)
        ? track.id
        : undefined);

    const isLikedSongsView =
      activeCollection?.title === "Liked Songs" ||
      (likedPlaylistId &&
        (activeCollection?.playlistId === likedPlaylistId ||
          activeCollection?.id === likedPlaylistId));

    try {
      if (localId) {
        if (isLiked) {
          await handleRemoveFeedback(localId);
        } else {
          await handleLike(localId);
        }
      } else {
        await handleLikeOnline({
          id: track.id,
          title: track.title,
          artist: track.artist,
          album: track.album,
          cover_art_url: track.cover_art_url,
          preview_url: track.preview_url,
          duration_secs: track.duration_secs,
        });
      }

      if (isLiked && isLikedSongsView) {
        setActiveCollection((prev) => {
          if (!prev || !prev.tracks) return prev;
          const updatedTracks = prev.tracks.filter(
            (t) =>
              t.id !== track.id &&
              (!localId ||
                (t.matched_local_track_id !== localId && t.id !== localId)),
          );
          return {
            ...prev,
            tracks: updatedTracks,
            subtitle: `${updatedTracks.length} tracks`,
          };
        });
      }
    } catch (err: any) {
      console.error("Failed to toggle collection like:", err);
      addAppNotification(
        "error",
        "Like Failed",
        err?.message || "Could not update track like state.",
      );
    }
  };

  return { likedPlaylistId, handleLike, handleRemoveFeedback, handleLikeOnline, handleToggleCollectionLike };
}
