import { useEffect, useMemo } from "react";
import { shownArtwork } from "../../services/shownArtwork";
import { lookupArtwork, type ArtworkSources } from "./useNowPlayingDetails";
import type { PlaybackArtworkTrack, QueueEntry } from "../../presentation/queuePresentation";
import type { Track } from "../../types";

/** A neighbour with the cover it showed while playing, or the same lookup the playing song uses. */
function withCover<T extends PlaybackArtworkTrack>(track: T, sources: ArtworkSources): T {
  const cover = shownArtwork.get(track.id) || lookupArtwork(track, sources);
  return cover && cover !== track.cover_art_url ? { ...track, cover_art_url: cover } : track;
}

/**
 * The Previous and Next songs for the sleeves on Home, The Deck and Now
 * Playing (PC and phone), with the same artwork they show while playing.
 */
export function useNeighbourTracks({ current, currentArtwork, previous, queueEntries, sources }: {
  current: Track | undefined;
  currentArtwork: string | null;
  previous: Track | undefined;
  queueEntries: QueueEntry[];
  sources: ArtworkSources;
}) {
  useEffect(() => {
    if (current) shownArtwork.remember(current.id, currentArtwork);
  }, [current?.id, currentArtwork]); // eslint-disable-line react-hooks/exhaustive-deps

  const previousTrack = useMemo(() => (previous ? withCover(previous, sources) : undefined), [previous, sources]);
  const nextTrack = useMemo<PlaybackArtworkTrack | undefined>(() => {
    const entry = queueEntries[0];
    if (!entry) return undefined;
    const base = sources.tracks.find((t) => t.id === entry.trackId)
      || { id: entry.trackId, title: entry.title || "Up next", artist_name: entry.artist };
    return withCover(base, sources);
  }, [queueEntries, sources]);
  return { previousTrack, nextTrack };
}
