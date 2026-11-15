// Playback, queue and transport events.
import { upcomingFromEvent } from "../../../presentation/queuePresentation";
import { playbackProgress } from "../../../services/playbackProgress";
import type { Track } from "../../../types";
import type { BackendEvent, BackendEventContext } from "./context";

/** Handles one playback event; returns false when the event belongs elsewhere. */
export function handlePlaybackEvent(event: BackendEvent, ctx: BackendEventContext): boolean {
  const { addAppNotification, fetchFlowSession, fetchPlaybackState, playbackSessionId, playbackStateRevision, setLoadingTrackId, setPlaybackState, setQueueEntries, setQueuedTrackIds, tracksRef } = ctx;
  switch (event.event) {
    case "PlaybackStarted": {
      ++playbackStateRevision.current;
      playbackSessionId.current = event.payload?.session_id || null;
      playbackProgress.update(0, event.payload?.duration_secs || 0);
      const trackId = event.payload?.track_id;
      setLoadingTrackId((pending) => pending === trackId ? null : pending);
      const found = tracksRef.current.find((t) => t.id === trackId);
      const currentTrackDetail = event.payload?.current_track;
      const track: Track =
        found ||
        (currentTrackDetail
          ? {
              id: currentTrackDetail.id,
              file_path: currentTrackDetail.file_path,
              title: currentTrackDetail.title,
              artist_name: currentTrackDetail.artist_name,
              album_title: currentTrackDetail.album_title,
              duration_secs: currentTrackDetail.duration_secs,
              format: currentTrackDetail.format,
              has_cover_art: currentTrackDetail.cover_art_url ? 1 : 0,
              cover_art_url: currentTrackDetail.cover_art_url,
            }
          : {
              id: trackId,
              file_path: "",
              title: event.payload?.title || "Unknown Track",
              artist_name: event.payload?.artist || "Unknown Artist",
              duration_secs: event.payload?.duration_secs || 0,
              format: "",
              has_cover_art: 0,
              // Unknown until the backend answers: never guess local vs online.
              pending: true,
            });
      setPlaybackState((prev) => ({
        ...prev,
        // Keep an already-resolved copy of this song instead of a placeholder.
        current_track: track.pending && prev.current_track?.id === trackId && !prev.current_track?.pending
          ? prev.current_track
          : track,
        is_playing: true,
        is_paused: false,
        is_buffering: false,
        position_secs: 0,
        duration_secs:
          event.payload?.duration_secs ||
          0,
      }));
      // The event only names the song, and the library list is paged, so load
      // the authoritative (presented) track rather than trusting a guess.
      if (!found) void fetchPlaybackState();
      break;
    }

    case "PlaybackBuffering": {
      ++playbackStateRevision.current;
      // Only PlaybackService emits this initial-resolution event. Reader
      // stalls use the session-scoped event below and cannot select a song.
      playbackSessionId.current = null;
      playbackProgress.update(0, 0);
      const trackId = event.payload?.track_id;
      const found = tracksRef.current.find((t) => t.id === trackId);
      setPlaybackState((prev) => ({
        ...prev,
        current_track:
          found ||
          (trackId && prev.current_track?.id === trackId && !prev.current_track?.pending
            ? prev.current_track
            : trackId
            ? {
                id: trackId,
                file_path: "",
                title:
                  (prev.current_track?.id === trackId
                    ? prev.current_track?.title
                    : undefined) || "Resolving Audio...",
                artist_name:
                  (prev.current_track?.id === trackId
                    ? prev.current_track?.artist_name
                    : undefined) || "Please wait",
                duration_secs:
                  (prev.current_track?.id === trackId
                    ? prev.current_track?.duration_secs
                    : undefined) || 0,
                format: "",
                has_cover_art: 0,
                pending: true,
              }
            : prev.current_track),
        is_buffering: true,
        duration_secs: 0,
        position_secs: 0,
      }));
      break;
    }

    case "PlaybackBufferingChanged": {
      if (!event.payload?.session_id || event.payload.session_id !== playbackSessionId.current) break;
      ++playbackStateRevision.current;
      setPlaybackState((prev) => {
        if (prev.current_track?.id !== event.payload?.track_id) return prev;
        const buffering = !!event.payload?.is_buffering && prev.is_playing;
        return prev.is_buffering === buffering ? prev : { ...prev, is_buffering: buffering };
      });
      break;
    }

    case "PlaybackError":
      ++playbackStateRevision.current;
      setPlaybackState((prev) => ({ ...prev, is_buffering: false }));
      addAppNotification("error", "Playback unavailable", event.payload?.message || "This track couldn’t be played. Try another track or retry playback.");
      void fetchPlaybackState();
      break;

    case "PlaybackPaused":
      ++playbackStateRevision.current;
      playbackProgress.update(
        event.payload?.position_secs ?? playbackProgress.getPosition(),
        playbackProgress.getDuration(),
      );
      setPlaybackState((prev) => ({
        ...prev,
        is_playing: false,
        is_paused: true,
        is_buffering: false,
        position_secs: event.payload?.position_secs ?? prev.position_secs,
      }));
      break;

    case "PlaybackResumed":
      ++playbackStateRevision.current;
      playbackSessionId.current = event.payload?.session_id || playbackSessionId.current;
      playbackProgress.update(
        event.payload?.position_secs ?? playbackProgress.getPosition(),
        playbackProgress.getDuration(),
      );
      setPlaybackState((prev) => ({
        ...prev,
        is_playing: true,
        is_paused: false,
        is_buffering: false,
        position_secs: event.payload?.position_secs ?? prev.position_secs,
      }));
      break;

    case "PlaybackStopped":
      ++playbackStateRevision.current;
      playbackSessionId.current = null;
      playbackProgress.update(0, playbackProgress.getDuration());
      setPlaybackState((prev) => ({
        ...prev,
        is_playing: false,
        is_paused: false,
        is_buffering: false,
        position_secs: 0,
      }));
      break;

    case "PlaybackSeeked": {
      const pos = event.payload?.position_secs ?? 0;
      playbackProgress.update(pos, playbackProgress.getDuration());
      break;
    }

    case "PlaybackPositionChanged": {
      if (!playbackSessionId.current && event.payload?.session_id) playbackSessionId.current = event.payload.session_id;
      if (event.payload?.session_id && playbackSessionId.current && event.payload.session_id !== playbackSessionId.current) break;
      const pos = event.payload?.position_secs ?? 0;
      const dur = event.payload?.duration_secs ?? 0;
      playbackProgress.update(pos, dur);
      break;
    }

    case "PlaybackVolumeChanged":
    case "VolumeChanged":
      setPlaybackState((prev) => ({
        ...prev,
        volume: event.payload?.volume ?? prev.volume,
        is_muted: event.payload?.is_muted ?? prev.is_muted,
      }));
      break;

    case "QueueUpdated": {
      void fetchFlowSession().catch(console.error);
      if (Array.isArray(event.payload?.items)) {
        setQueueEntries(
          upcomingFromEvent(
            event.payload.items,
            event.payload.current_index ?? null,
          ),
        );
        setPlaybackState((prev) => ({
          ...prev,
          current_queue_index: event.payload.current_index ?? null,
          source: event.payload.source || prev.source,
        }));
      }
      // Queue buttons show ✓ only for songs the user queued; a song the app
      // queued can still be queued, which moves it up to play next.
      setQueuedTrackIds(new Set(event.payload?.user_queue_track_ids ?? []));
      break;
    }

    case "PlaybackStateChanged":
      setPlaybackState((prev) => ({
        ...prev,
        is_playing: event.payload?.is_playing ?? prev.is_playing,
      }));
      break;

    case "TrackChanged":
      setPlaybackState((prev) => ({
        ...prev,
        current_track: event.payload?.track ?? prev.current_track,
        position_secs: 0,
        duration_secs: event.payload?.track?.duration_secs ?? 0,
      }));
      break;

    case "RepeatModeChanged":
      setPlaybackState((prev) => ({
        ...prev,
        repeat_mode: event.payload?.mode ?? prev.repeat_mode,
      }));
      break;

    case "ShuffleModeChanged":
      setPlaybackState((prev) => ({
        ...prev,
        is_shuffled: event.payload?.enabled ?? prev.is_shuffled,
      }));
      break;

    default:
      return false;
  }
  return true;
}
