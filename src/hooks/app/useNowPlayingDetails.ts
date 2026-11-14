import { useEffect, useMemo, useState, useSyncExternalStore } from "react";
import { subscribeCoverArt, coverArtGeneration, loadCoverArt } from "../../services/coverArt";
import type { Album, DiscoveryRecommendation, Playlist, Track } from "../../types";
import type { CollectionData } from "../../components/views/CollectionDetailView";

const sameText = (a: string | undefined | null, b: string | undefined | null) =>
  a?.toLowerCase().trim() === b?.toLowerCase().trim();

/** Matches a listed recording to the playing track: by id, local match, or title + artist. */
function matchesTrack(current: Track, item: { title: string; artist: string; matched_local_track_id?: string | null }, id: string) {
  return item.matched_local_track_id === current.id || id === current.id
    || (sameText(item.title, current.title) && sameText(item.artist, current.artist_name));
}

/** Resolves the playing track's cover once per track and shares it with the mounted views. */
function useResolvedCover(track: Track | undefined) {
  const artworkGeneration = useSyncExternalStore(subscribeCoverArt, coverArtGeneration, coverArtGeneration);
  const [resolved, setResolved] = useState<{ id: string; generation: number; url: string | null } | null>(null);
  useEffect(() => {
    if (!track || track.cover_art_url || track.id.startsWith("online:") || track.id.startsWith("itunes:")) return;
    const controller = new AbortController();
    loadCoverArt(track.id, controller.signal).then((url) => {
      if (!controller.signal.aborted) setResolved({ id: track.id, generation: artworkGeneration, url });
    });
    return () => controller.abort();
  }, [track?.id, track?.cover_art_url, track?.has_cover_art, artworkGeneration]);
  return track?.cover_art_url || (resolved?.generation === artworkGeneration && resolved?.id === track?.id ? resolved?.url : null);
}

/** The best artwork for the playing track, from the most to the least specific source. */
export function useNowPlayingArtwork({ track, tracks, activeCollection, discoveryRecs, albums }: {
  track: Track | undefined;
  tracks: Track[];
  activeCollection: CollectionData | null;
  discoveryRecs: DiscoveryRecommendation[];
  albums: Album[];
}) {
  const resolvedCover = useResolvedCover(track);
  return useMemo(() => {
    if (resolvedCover) return resolvedCover;
    if (!track) return null;
    if (track.cover_art_url) return track.cover_art_url;
    const fromLibrary = tracks.find((t) => t.id === track.id)?.cover_art_url;
    if (fromLibrary) return fromLibrary;
    const fromCollection = activeCollection?.tracks?.find((t) => matchesTrack(track, t, t.id))?.cover_art_url;
    if (fromCollection) return fromCollection;
    const fromDiscovery = discoveryRecs.find((r) => matchesTrack(track, r, r.external_track_id))?.cover_art_url;
    if (fromDiscovery) return fromDiscovery;
    if (track.album_title) {
      const album = albums.find((a) => sameText(a.title, track.album_title));
      if (album?.cover_art_path) return album.cover_art_path;
    }
    return null;
  }, [resolvedCover, track, tracks, activeCollection?.tracks, discoveryRecs, albums]);
}

function likedIds(trackPlaylistMap: Record<string, string[]>, likedPlaylistId: string | null | undefined, tracks: Track[]) {
  const set = new Set<string>();
  if (likedPlaylistId) {
    for (const [trackId, playlistIds] of Object.entries(trackPlaylistMap)) {
      if (playlistIds.includes(likedPlaylistId)) set.add(trackId);
    }
  }
  for (const t of tracks) {
    if (t.manual_like === 1) set.add(t.id);
  }
  return set;
}

function onlyPlaylists(trackPlaylistMap: Record<string, string[]>, keep: Set<string>) {
  const filtered: Record<string, string[]> = {};
  for (const [trackId, playlistIds] of Object.entries(trackPlaylistMap)) {
    filtered[trackId] = playlistIds.filter((playlistId) => keep.has(playlistId));
  }
  return filtered;
}

function inAnyPlaylist(track: Track | undefined, map: Record<string, string[]>) {
  if (!track) return false;
  const altId = `online:${track.artist_name || ""}-${track.title}`;
  return (map[track.id]?.length ?? 0) > 0 || (map[altId]?.length ?? 0) > 0;
}

/** Liked songs and regular-playlist memberships, and whether the playing track is in them. */
export function usePlaylistMembership({ track, tracks, playlists, trackPlaylistMap, likedPlaylistId }: {
  track: Track | undefined;
  tracks: Track[];
  playlists: Playlist[];
  trackPlaylistMap: Record<string, string[]>;
  likedPlaylistId: string | null | undefined;
}) {
  const likedTrackIds = useMemo(() => likedIds(trackPlaylistMap, likedPlaylistId, tracks), [trackPlaylistMap, likedPlaylistId, tracks]);
  const isCurrentTrackLiked = !!track && (likedTrackIds.has(track.id) || track.manual_like === 1);

  const regularPlaylistIds = useMemo(
    () => new Set(playlists.filter((p) => p.is_smart_mix !== 1 && p.name !== "Liked Songs").map((p) => p.id)),
    [playlists],
  );
  const regularTrackPlaylistMap = useMemo(() => onlyPlaylists(trackPlaylistMap, regularPlaylistIds), [trackPlaylistMap, regularPlaylistIds]);
  const isCurrentTrackInPlaylist = useMemo(() => inAnyPlaylist(track, regularTrackPlaylistMap), [track, regularTrackPlaylistMap]);

  return { likedTrackIds, isCurrentTrackLiked, regularPlaylistIds, regularTrackPlaylistMap, isCurrentTrackInPlaylist };
}
