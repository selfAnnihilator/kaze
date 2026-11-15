import { isOnlineTrack } from "../../trackKind";
import { useCallback, useMemo } from "react";
import type { Dispatch, SetStateAction } from "react";
import { dispatchCommand, executeQuery } from "../../services/api";
import { playbackProgress } from "../../services/playbackProgress";
import { showEmptyPlaybackNotice } from "../../components/PlaybackNotice";
import type { DiscoveryRecommendation, PlaybackState, Playlist, Track } from "../../types";

type Notify = (type: "success" | "error" | "info" | "warning", title: string, message: string) => void;
import type { PlayingOrigin } from "../../playingOrigin";

/**
 * Playback commands sent to the Rust PlaybackService: play local/online tracks,
 * play/pause, next/previous, seek (writes the progress store, never React state),
 * volume, mute, repeat, shuffle, and playing a whole playlist.
 */
export function usePlaybackControls({
  playbackState, setPlaybackState, fetchPlaybackState, beginPlaybackLoading, endPlaybackLoading, tracks,
  addAppNotification, playlists, setPlayingPlaylistId, setPlayingOrigin, queuedTrackIds,
}: {
  playbackState: PlaybackState;
  setPlaybackState: Dispatch<SetStateAction<PlaybackState>>;
  fetchPlaybackState: () => Promise<unknown>;
  beginPlaybackLoading: (trackId: string) => number;
  endPlaybackLoading: (requestId: number) => void;
  tracks: Track[];
  addAppNotification: Notify;
  playlists: Playlist[];
  setPlayingPlaylistId: Dispatch<SetStateAction<string | null>>;
  setPlayingOrigin: Dispatch<SetStateAction<PlayingOrigin>>;
  queuedTrackIds: Set<string>;
}) {
  const handleStopOnlineAudio = useCallback(() => {}, []);

  const handleStopTrack = useCallback(async () => {
    if (playbackState.is_playing) {
      await dispatchCommand({ command: "Pause" });
      setPlaybackState((prev) => ({ ...prev, is_playing: false }));
    }
  }, [playbackState.is_playing]);

  const handlePlayTrack = useCallback(
    async (trackId: string) => {
      if (playbackState.current_track?.id === trackId && (playbackState.is_playing || playbackState.is_paused)) {
        await dispatchCommand({
          command: playbackState.is_playing ? "Pause" : "Resume",
        });
        await fetchPlaybackState();
        return;
      }
      const requestId = beginPlaybackLoading(trackId);
      try {
        const found = tracks.find((t) => t.id === trackId);
        if (found) {
          setPlaybackState((prev) => ({
            ...prev,
            current_track: found,
            is_playing: true,
            position_secs: 0,
            duration_secs: found.duration_secs ?? 0,
          }));
        }
        await dispatchCommand({
          command: "PlayTrack",
          payload: { track_id: trackId },
        });
        await fetchPlaybackState();
      } catch (err) {
        await fetchPlaybackState();
        throw err;
      } finally {
        endPlaybackLoading(requestId);
      }
    },
    [
      tracks,
      playbackState.current_track?.id,
      playbackState.is_playing,
      playbackState.is_paused,
      fetchPlaybackState,
      beginPlaybackLoading,
      endPlaybackLoading,
    ],
  );

  const handlePlayOnlineTrack = useCallback(
    async (
      rec: DiscoveryRecommendation,
      _fromBackendQueue = false,
      playbackSource: "online" | "collection" | "playlist" = "online",
    ) => {
      const exactProvider = !!rec.playback_source || rec.provider === "jiosaavn" || rec.external_track_id.startsWith("jiosaavn:");
      const hasRealLocalMatch =
        !exactProvider && !!rec.matched_local_track_id &&
        !rec.matched_local_track_id.startsWith("itunes:") &&
        !rec.matched_local_track_id.startsWith("online:");

      // 1. Check if this track is currently playing
      const isCurrentlyPlaying =
        (playbackState.is_playing || playbackState.is_paused) &&
        playbackState.current_track &&
        ((hasRealLocalMatch &&
          playbackState.current_track.id === rec.matched_local_track_id) ||
          rec.external_track_id === playbackState.current_track.id ||
          (!!rec.playback_source && !!playbackState.current_track.playback_source &&
            rec.playback_source.track_id === playbackState.current_track.playback_source.track_id));

      if (isCurrentlyPlaying) {
        await dispatchCommand({
          command: playbackState.is_playing ? "Pause" : "Resume",
        });
        setPlaybackState((prev) => ({ ...prev, is_playing: !playbackState.is_playing }));
        return;
      }

      // 2. If this track is in local library, play it locally
      if (hasRealLocalMatch) {
        await handlePlayTrack(rec.matched_local_track_id!);
        return;
      }

      // Dispatch PlayOnlineTrack to unified Rust PlaybackService
      const requestId = beginPlaybackLoading(rec.external_track_id);
      try {
        await dispatchCommand({
          command: "PlayOnlineTrack",
          payload: {
            track_id: rec.external_track_id,
            title: rec.title,
            artist: rec.artist,
            album: rec.album || undefined,
            duration_secs: rec.duration_secs || undefined,
            cover_art_url: rec.cover_art_url || undefined,
            preview_url: rec.preview_url || undefined,
            source: playbackSource,
          },
        });
        await fetchPlaybackState();
      } catch (err) {
        console.error("Failed to play online track via Rust engine:", err);
        addAppNotification(
          "error",
          "Playback Failed",
          `Could not stream "${rec.title}": ${String(err)}`,
        );
      } finally {
        endPlaybackLoading(requestId);
      }
    },
    [
      playbackState.is_playing,
      playbackState.is_paused,
      playbackState.current_track,
      handlePlayTrack,
      addAppNotification,
      fetchPlaybackState,
      beginPlaybackLoading,
      endPlaybackLoading,
    ],
  );

  const handlePlayPause = useCallback(async () => {
    if (!playbackState.current_track && !playbackState.current_track_id && !playbackState.queue_track_ids?.length) {
      showEmptyPlaybackNotice(); return;
    }
    try {
      await dispatchCommand({ command: "TogglePlayPause" });
      await fetchPlaybackState();
    } catch (error) {
      if (String(error).includes("Select a song to play")) showEmptyPlaybackNotice();
      else addAppNotification("error", "Playback unavailable", String(error));
    }
  }, [playbackState.current_track, playbackState.current_track_id, playbackState.queue_track_ids?.length, fetchPlaybackState, addAppNotification]);

  const handleNextTrack = useCallback(async () => {
    await dispatchCommand({ command: "NextTrack" });
  }, []);

  const handlePreviousTrack = useCallback(async () => {
    await dispatchCommand({ command: "PreviousTrack" });
  }, []);

  const handleSeek = useCallback(async (position_secs: number) => {
    playbackProgress.update(position_secs, playbackProgress.getDuration());
    await dispatchCommand({
      command: "Seek",
      payload: { position_secs },
    });
  }, []);

  const handleVolumeChange = useCallback(async (volume: number) => {
    await dispatchCommand({
      command: "SetVolume",
      payload: { volume },
    });
    setPlaybackState((prev) => ({ ...prev, volume }));
  }, []);

  const handleToggleMute = useCallback(async () => {
    await dispatchCommand({ command: "ToggleMute" });
    setPlaybackState((prev) => ({ ...prev, is_muted: !prev.is_muted }));
  }, []);

  const handleToggleRepeat = useCallback(async () => {
    const nextMode = playbackState.repeat_mode === "one" ? "off" : "one";

    await dispatchCommand({
      command: "SetRepeatMode",
      payload: { mode: nextMode },
    });
    setPlaybackState((prev) => ({ ...prev, repeat_mode: nextMode }));
  }, [playbackState.repeat_mode]);

  const handleToggleShuffle = useCallback(async () => {
    const next = !playbackState.is_shuffled;
    await dispatchCommand({
      command: "SetShuffle",
      payload: { enabled: next },
    });
    setPlaybackState((prev) => ({ ...prev, is_shuffled: next }));
  }, [playbackState.is_shuffled]);

  const handlePlayPlaylist = useCallback(async (playlistId: string) => {
    setPlayingPlaylistId(playlistId);
    const pl = playlists.find((p) => p.id === playlistId);
    setPlayingOrigin({
      type: "playlist",
      id: playlistId,
      name: pl?.name || "Playlist",
    });
    try {
      const res = await executeQuery({
        query: "GetPlaylistTracks",
        payload: { playlist_id: playlistId },
      });
      const plTracks: Track[] = (res.data as any) || [];
      if (plTracks.length > 0) {
        const playlistTrackIds = new Set(plTracks.map((track) => track.id));
        setPlayingOrigin({ type: "playlist", id: playlistId, name: pl?.name || "Playlist", trackIds: [...playlistTrackIds] });
        const preservedQueue = Array.from(queuedTrackIds).filter(
          (trackId) => !playlistTrackIds.has(trackId),
        );
        const first = plTracks[0];
        const isOnline = !!isOnlineTrack(first);

        if (isOnline) {
          if (!first.id.startsWith("jiosaavn:") && typeof navigator !== "undefined" && !navigator.onLine) {
            addAppNotification(
              "warning",
              "No Internet Connection",
              `Cannot play "${first.title}". Connect to the internet to stream online songs.`,
            );
            return;
          }

          const rec: DiscoveryRecommendation = {
            external_track_id: first.id,
            provider: "online",
            provider_id: first.id,
            title: first.title,
            artist: first.artist_name || "Unknown Artist",
            album: first.album_title,
            duration_secs: first.duration_secs ?? 0,
            cover_art_url: first.cover_art_url,
            preview_url: first.preview_url,
            match_status: "NOT_FOUND",
            recommendation_reason: "",
            in_wishlist: false,
          };
          await handlePlayOnlineTrack(rec, false, "playlist");
          for (let i = 1; i < plTracks.length; i++) {
            const nextTrack = plTracks[i];
            if (
              nextTrack.format === "online" ||
              nextTrack.file_path?.startsWith("online://")
            ) {
              await dispatchCommand({
                command: "EnqueueOnlineTrack",
                payload: {
                  track_id: nextTrack.id,
                  title: nextTrack.title,
                  artist: nextTrack.artist_name || "Unknown Artist",
                  album: nextTrack.album_title,
                  duration_secs: nextTrack.duration_secs,
                  cover_art_url: nextTrack.cover_art_url,
                  preview_url: nextTrack.preview_url,
                  play_next: false,
                },
              });
            } else {
              await dispatchCommand({
                command: "EnqueueTrack",
                payload: { track_id: nextTrack.id, play_next: false },
              });
            }
          }
        } else {
          await dispatchCommand({
            command: "PlayTrack",
            payload: { track_id: first.id, source: "playlist" },
          });
          for (let i = 1; i < plTracks.length; i++) {
            const nextTrack = plTracks[i];
            const nextIsOnline = !!isOnlineTrack(nextTrack);
            if (nextIsOnline) {
              await dispatchCommand({
                command: "EnqueueOnlineTrack",
                payload: {
                  track_id: nextTrack.id,
                  title: nextTrack.title,
                  artist: nextTrack.artist_name || "Unknown Artist",
                  album: nextTrack.album_title,
                  duration_secs: nextTrack.duration_secs,
                  cover_art_url: nextTrack.cover_art_url,
                  preview_url: nextTrack.preview_url,
                  play_next: false,
                },
              });
            } else {
              await dispatchCommand({
                command: "EnqueueTrack",
                payload: { track_id: nextTrack.id, play_next: false },
              });
            }
          }
          fetchPlaybackState();
        }
        for (const queuedId of preservedQueue) {
          try {
            await dispatchCommand({
              command: "EnqueueTrack",
              payload: { track_id: queuedId, play_next: false },
            });
          } catch (err) {
            console.warn(
              "Could not preserve queued track after playlist:",
              err,
            );
          }
        }
      }
    } catch (err) {
      console.error("Failed to play playlist:", err);
    }
  }, [setPlayingPlaylistId, playlists, setPlayingOrigin, queuedTrackIds, addAppNotification, handlePlayOnlineTrack, fetchPlaybackState]);

  return useMemo(() => ({
    handleStopOnlineAudio, handleStopTrack, handlePlayTrack, handlePlayOnlineTrack, handlePlayPause, handleNextTrack, handlePreviousTrack, handleSeek, handleVolumeChange, handleToggleMute, handleToggleRepeat, handleToggleShuffle, handlePlayPlaylist,
  }), [handleStopOnlineAudio, handleStopTrack, handlePlayTrack, handlePlayOnlineTrack, handlePlayPause, handleNextTrack, handlePreviousTrack, handleSeek, handleVolumeChange, handleToggleMute, handleToggleRepeat, handleToggleShuffle, handlePlayPlaylist]);
}
