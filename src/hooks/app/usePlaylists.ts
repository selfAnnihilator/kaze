import { useCallback, useMemo, useRef, useState } from "react";
import type { Dispatch, MutableRefObject, SetStateAction } from "react";
import { dispatchCommand, executeQuery } from "../../services/api";
import { recommendationPreferences } from "../../services/recommendationPreferences";
import type { JioSaavnPlaylistImport, Playlist, Track, UserProfile } from "../../types";
import type { AddToPlaylistModalTrack } from "../../components/modals/AddToPlaylistModal";
import type { CollectionData } from "../../components/views/CollectionDetailView";

type Notify = (type: "success" | "error" | "info" | "warning", title: string, message: string) => void;

/**
 * Playlists: list + per-track membership, the add-to-playlist modal, the playing
 * playlist id, CRUD, track toggling, and JioSaavn/collection imports.
 */
export function usePlaylists({
  libraryAccountRef, currentUser, setIsAuthModalOpen, activeCollection, setActiveCollection,
  addAppNotification, requestPrompt,
}: {
  libraryAccountRef: MutableRefObject<string>;
  currentUser: UserProfile | null;
  setIsAuthModalOpen: Dispatch<SetStateAction<boolean>>;
  activeCollection: CollectionData | null;
  setActiveCollection: (value: SetStateAction<CollectionData | null>) => void;
  addAppNotification: Notify;
  requestPrompt: (title: string, message: string, defaultValue?: string) => Promise<string | null>;
}) {
  const [isPlaylistModalOpen, setIsPlaylistModalOpen] = useState(false);
  const [playlistModalTrack, setPlaylistModalTrack] = useState<AddToPlaylistModalTrack | null>(null);
  const [trackPlaylistMap, setTrackPlaylistMap] = useState<Record<string, string[]>>({});
  const [playlists, setPlaylists] = useState<Playlist[]>([]);
  const [playingPlaylistId, setPlayingPlaylistId] = useState<string | null>(null,
  );

  const playlistRequest = useRef(0);
  const membershipRequest = useRef(0);
  const fetchPlaylists = useCallback(async () => {
    const account = libraryAccountRef.current, request = ++playlistRequest.current;
    void recommendationPreferences.refresh().catch(console.error);
    try {
      const res = await executeQuery({ query: "GetPlaylists" });
      if (account === libraryAccountRef.current && request === playlistRequest.current && Array.isArray(res.data)) {
        setPlaylists(res.data);
        return true;
      }
    } catch (err) {
      console.error("Failed to fetch playlists:", err);
    }
    return false;
  }, []);

  const handleRefreshPlaylists = useCallback(async () => {
    if (!(await fetchPlaylists())) throw Error("Couldn’t refresh playlists. Please try again.");
  }, [fetchPlaylists]);

  const fetchTrackPlaylistMemberships = useCallback(async () => {
    const account = libraryAccountRef.current, request = ++membershipRequest.current;
    try {
      const res = await executeQuery({ query: "GetTrackPlaylistMemberships" });
      if (account === libraryAccountRef.current && request === membershipRequest.current && res.data && typeof res.data === "object") {
        setTrackPlaylistMap(res.data);
      }
    } catch (err) {
      console.error("Failed to fetch track playlist memberships:", err);
    }
  }, []);

  const handleRenamePlaylist = useCallback(async (playlistId: string, name: string) => {
    await dispatchCommand({
      command: "RenamePlaylist",
      payload: { playlist_id: playlistId, name },
    });
    setActiveCollection((current) =>
      current &&
      (current.playlistId === playlistId || current.id === playlistId)
        ? { ...current, title: name.trim() }
        : current,
    );
    await fetchPlaylists();
  }, [setActiveCollection, fetchPlaylists]);

  const handleDeletePlaylist = useCallback(async (playlistId: string) => {
    await dispatchCommand({
      command: "DeletePlaylist",
      payload: { playlist_id: playlistId },
    });
    await fetchPlaylists();
    if (
      activeCollection?.playlistId === playlistId ||
      activeCollection?.id === playlistId
    ) {
      setActiveCollection(null);
    }
  }, [activeCollection, setActiveCollection, fetchPlaylists]);

  const handleCreatePlaylist = useCallback(async (
    name: string,
    description?: string,
  ): Promise<string | null> => {
    if (!currentUser) {
      setIsAuthModalOpen(true);
      return null;
    }
    let targetName = name.trim();
    while (
      playlists.some((p) => p.is_smart_mix !== 1 && p.name === targetName)
    ) {
      const prompted = window.prompt(
        `A playlist named "${targetName}" already exists.\nPlease enter a new name for this playlist:`,
        `${targetName} (1)`,
      );
      if (prompted === null) {
        return null;
      }
      const trimmed = prompted.trim();
      if (!trimmed) {
        alert("Playlist name cannot be empty.");
        continue;
      }
      targetName = trimmed;
    }

    const account = libraryAccountRef.current;
    const res = await dispatchCommand({
      command: "CreatePlaylist",
      payload: { name: targetName, description },
    });
    if (!res.data || typeof res.data !== "string") return null;
    try {
      const refreshed = await executeQuery({query:"GetPlaylists"});
      if (!Array.isArray(refreshed.data)) throw Error("Invalid playlist response");
      if (account === libraryAccountRef.current) setPlaylists(refreshed.data);
    } catch {
      if (account === libraryAccountRef.current) setPlaylists(previous => previous.some(playlist => playlist.id === res.data) ? previous : [...previous, {id:res.data,name:targetName,description,is_smart_mix:0,track_count:0}]);
      addAppNotification("error", "Playlist created; list refresh failed", "Your playlist was saved. Use Refresh playlists to update the list.");
    }
    if (res.data && typeof res.data === "string") {
      return res.data;
    }
    return null;
  }, [currentUser, setIsAuthModalOpen, playlists, libraryAccountRef, addAppNotification]);

  const handleToggleTrackInPlaylist = useCallback(async (
    playlistId: string,
    isCurrentlyMember: boolean,
  ) => {
    if (!playlistModalTrack) return;
    const trackId = playlistModalTrack.id;
    try {
      if (isCurrentlyMember) {
        await dispatchCommand({
          command: "RemoveTrackFromPlaylist",
          payload: { playlist_id: playlistId, track_id: trackId },
        });
        setTrackPlaylistMap((prev) => {
          const current = prev[trackId] || [];
          return {
            ...prev,
            [trackId]: current.filter((id) => id !== playlistId),
          };
        });
      } else {
        await dispatchCommand({
          command: "AddTrackToPlaylist",
          payload: {
            playlist_id: playlistId,
            track_id: trackId,
            title: playlistModalTrack.title,
            artist: playlistModalTrack.artist,
            album: playlistModalTrack.album,
            cover_art_url: playlistModalTrack.cover_art_url,
          },
        });
        setTrackPlaylistMap((prev) => {
          const current = prev[trackId] || [];
          return { ...prev, [trackId]: [...current, playlistId] };
        });
      }
      await fetchPlaylists();
      await fetchTrackPlaylistMemberships();
    } catch (err) {
      console.error("Failed to toggle track in playlist:", err);
      throw err;
    }
  }, [playlistModalTrack, fetchPlaylists, fetchTrackPlaylistMemberships]);

  const handleOpenAddToPlaylistModal = useCallback((track: {
    id: string;
    title: string;
    artist?: string;
    album?: string;
    cover_art_url?: string;
  }) => {
    setPlaylistModalTrack(track);
    setIsPlaylistModalOpen(true);
  }, []);

  const handleFetchPlaylistTracks = useCallback(async (
    playlistId: string,
  ): Promise<Track[]> => {
    try {
      const res = await executeQuery({
        query: "GetPlaylistTracks",
        payload: { playlist_id: playlistId },
      });
      return (res.data as any) || [];
    } catch (err) {
      console.error("Failed to fetch playlist tracks:", err);
      return [];
    }
  }, []);

  const handleInspectJioSaavnPlaylist = useCallback(async (
    urlOrId: string,
  ): Promise<JioSaavnPlaylistImport | null> => {
    const res = await executeQuery({
      query: "ImportJioSaavnPlaylist",
      payload: { url_or_id: urlOrId },
    });
    return (res.data as JioSaavnPlaylistImport) || null;
  }, []);

  const handleSaveImportedPlaylist = useCallback(
    async (
      name: string,
      trackIds: string[],
      tracksInfo?: Array<{
        id: string;
        title?: string;
        artist?: string;
        album?: string;
        duration_secs?: number;
        cover_art_url?: string;
        preview_url?: string;
      }>,
    ) => {
      let targetName = name.trim();
      while (
        playlists.some(
          (p) =>
            p.is_smart_mix !== 1 &&
            p.name.toLowerCase() === targetName.toLowerCase(),
        )
      ) {
        const prompted = await requestPrompt(
          "Playlist Name Already Taken",
          `A playlist named "${targetName}" already exists in your library. Please enter a new name for this playlist:`,
          `${targetName} (1)`,
        );
        if (prompted === null) {
          return;
        }
        const trimmed = prompted.trim();
        if (!trimmed) {
          addAppNotification(
            "warning",
            "Name Required",
            "Playlist name cannot be empty.",
          );
          continue;
        }
        targetName = trimmed;
      }

      const plRes = await dispatchCommand({
        command: "CreatePlaylist",
        payload: { name: targetName, description: "Saved Collection" },
      });
      const playlistId = (plRes as any)?.data;
      if (playlistId) {
        for (let i = 0; i < trackIds.length; i++) {
          const tid = trackIds[i];
          const info = tracksInfo?.[i];
          await dispatchCommand({
            command: "AddTrackToPlaylist",
            payload: {
              playlist_id: playlistId,
              track_id: tid,
              title: info?.title,
              artist: info?.artist,
              album: info?.album,
              duration_secs: info?.duration_secs,
              cover_art_url: info?.cover_art_url,
              preview_url: info?.preview_url,
            },
          });
        }
        fetchPlaylists();
      }
    },
    [playlists, fetchPlaylists],
  );

  const handleSaveCollectionToPlaylists = useCallback(
    async (collection: CollectionData) => {
      try {
        let targetName = collection.title.trim();
        while (
          playlists.some(
            (p) =>
              p.is_smart_mix !== 1 &&
              p.name.toLowerCase() === targetName.toLowerCase(),
          )
        ) {
          const prompted = await requestPrompt(
            "Playlist Name Already Taken",
            `A playlist named "${targetName}" already exists in your library. Please enter a new name for this playlist:`,
            `${targetName} (1)`,
          );
          if (prompted === null) {
            return;
          }
          const trimmed = prompted.trim();
          if (!trimmed) {
            addAppNotification(
              "warning",
              "Name Required",
              "Playlist name cannot be empty.",
            );
            continue;
          }
          targetName = trimmed;
        }

        const allTracks = collection.tracks || [];
        const trackIds = allTracks.map((t) =>
          t.matched_local_track_id &&
          !t.matched_local_track_id.startsWith("online:") &&
          !t.matched_local_track_id.startsWith("itunes:")
            ? t.matched_local_track_id
            : t.id,
        );

        if (trackIds.length > 0) {
          await handleSaveImportedPlaylist(targetName, trackIds, allTracks);
        } else {
          await handleCreatePlaylist(targetName, collection.subtitle);
          addAppNotification(
            "info",
            "Playlist Created",
            `Playlist "${targetName}" created.`,
          );
        }
      } catch (err) {
        console.error("Failed to save collection to playlists:", err);
      }
    },
    [playlists, handleSaveImportedPlaylist, handleCreatePlaylist],
  );

  return useMemo(() => ({
    playlistRequest, membershipRequest, playlists, setPlaylists, playingPlaylistId, setPlayingPlaylistId, trackPlaylistMap, setTrackPlaylistMap, isPlaylistModalOpen, setIsPlaylistModalOpen, playlistModalTrack, setPlaylistModalTrack, fetchPlaylists, handleRefreshPlaylists, fetchTrackPlaylistMemberships, handleRenamePlaylist, handleDeletePlaylist, handleCreatePlaylist, handleToggleTrackInPlaylist, handleOpenAddToPlaylistModal, handleFetchPlaylistTracks, handleInspectJioSaavnPlaylist, handleSaveImportedPlaylist, handleSaveCollectionToPlaylists,
  }), [playlists, setPlaylists, playingPlaylistId, setPlayingPlaylistId, trackPlaylistMap, setTrackPlaylistMap, isPlaylistModalOpen, setIsPlaylistModalOpen, playlistModalTrack, setPlaylistModalTrack, fetchPlaylists, handleRefreshPlaylists, fetchTrackPlaylistMemberships, handleRenamePlaylist, handleDeletePlaylist, handleCreatePlaylist, handleToggleTrackInPlaylist, handleOpenAddToPlaylistModal, handleFetchPlaylistTracks, handleInspectJioSaavnPlaylist, handleSaveImportedPlaylist, handleSaveCollectionToPlaylists]);
}
