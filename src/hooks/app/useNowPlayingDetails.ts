import { useEffect, useMemo, useState, useSyncExternalStore } from "react";
import { subscribeCoverArt, coverArtGeneration, loadCoverArt, sizedCoverUrl } from "../../services/coverArt";
import type { Album, DiscoveryRecommendation, Playlist, Track } from "../../types";
import type { CollectionData } from "../../components/views/CollectionDetailView";

type ArtworkTrack = Pick<Track, "id" | "title" | "artist_name" | "album_title" | "cover_art_url">;

/** Where a song's cover can be found besides its own record. */
export interface ArtworkSources {
  tracks: Track[];
  activeCollection: CollectionData | null;
  discoveryRecs: DiscoveryRecommendation[];
  worldTrending?: DiscoveryRecommendation[];
  albums: Album[];
}

const sameText = (a: string | undefined | null, b: string | undefined | null) =>
  a?.toLowerCase().trim() === b?.toLowerCase().trim();

/** Matches a listed recording to the playing track: by id, local match, or title + artist. */
function matchesTrack(current: ArtworkTrack, item: { title: string; artist: string; matched_local_track_id?: string | null }, id: string) {
  return item.matched_local_track_id === current.id || id === current.id
    || (sameText(item.title, current.title) && sameText(item.artist, current.artist_name));
}

const NOW_PLAYING_CSS_PIXELS = 1024;

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
  const url = track?.cover_art_url || (resolved?.generation === artworkGeneration && resolved?.id === track?.id ? resolved?.url : null);
  // One image serves the bar, the stage and full screen: the largest size.
  return url ? sizedCoverUrl(url, NOW_PLAYING_CSS_PIXELS) : url;
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
    return track ? lookupArtwork(track, { tracks, activeCollection, discoveryRecs, albums }) : null;
  }, [resolvedCover, track, tracks, activeCollection, discoveryRecs, albums]);
}

/**
 * A song's cover from what the app already has: its own record, the library,
 * the open collection, Discover lists, then its album. Used for the playing
 * song and its Previous / Next neighbours alike, so all three agree.
 */
export function lookupArtwork(track: ArtworkTrack, sources: ArtworkSources): string | null {
  const listed = (list: DiscoveryRecommendation[] = []) => list.find((r) => matchesTrack(track, r, r.external_track_id))?.cover_art_url;
  const steps = [
    () => track.cover_art_url,
    () => sources.tracks.find((t) => t.id === track.id)?.cover_art_url,
    () => sources.activeCollection?.tracks?.find((t) => matchesTrack(track, t, t.id))?.cover_art_url,
    () => listed(sources.discoveryRecs),
    () => listed(sources.worldTrending),
    () => (track.album_title ? sources.albums.find((a) => sameText(a.title, track.album_title))?.cover_art_path : undefined),
  ];
  for (const step of steps) {
    const found = step();
    if (found) return found;
  }
  return null;
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
