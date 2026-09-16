import type { DiscoveryRecommendation, DownloadTask, Track } from "./types";
import { findLocalSearchMatch, sameSongMetadata } from "./localTrackMatch.ts";

interface DiscoverySectionsInput {
  recommendations: DiscoveryRecommendation[];
  playlistTrackIds: ReadonlySet<string>;
  likedTrackIds: ReadonlySet<string>;
  downloads: DownloadTask[];
  downloadTargets?: Record<string, { title: string; artist: string }>;
  localTracks: Track[];
  limit?: number;
}

const normalized = (value: string): string => value.normalize("NFKD")
  .replace(/[\u0300-\u036f]/g, "")
  .toLowerCase()
  .replace(/[^\p{L}\p{N}]+/gu, " ")
  .trim()
  .replace(/\s+/g, " ");

const songKey = (rec: DiscoveryRecommendation): string =>
  `${normalized(rec.title)}:${rec.artist.split(/\s*(?:,|&|\/|\b(?:and|with|feat\.?|ft\.?|x)\b)\s*/i)
    .map(normalized).filter(Boolean).sort().join("+")}`;

const isTasteBased = (rec: DiscoveryRecommendation): boolean =>
  /listening habit|library artist|library favorites|top genre|similar|your taste/i
    .test(rec.recommendation_reason);

/** Partition the existing taste-ranked feed into fresh personal picks and chart rows. */
export const splitDiscoveryRecommendations = ({
  recommendations,
  playlistTrackIds,
  likedTrackIds,
  downloads,
  downloadTargets = {},
  localTracks,
  limit = 12,
}: DiscoverySectionsInput): {
  forYou: DiscoveryRecommendation[];
  trending: DiscoveryRecommendation[];
} => {
  const seenSongs = new Set<string>();
  const personal: DiscoveryRecommendation[] = [];
  const other: DiscoveryRecommendation[] = [];

  for (const rec of recommendations) {
    const localId = rec.matched_local_track_id;
    if (rec.provider === "library" ||
      (rec.match_status === "EXACT_MATCH" && localId && !localId.startsWith("online:") && !localId.startsWith("itunes:")) ||
      findLocalSearchMatch(rec, localTracks)) continue;

    if (playlistTrackIds.has(rec.external_track_id) ||
      (localId && playlistTrackIds.has(localId)) ||
      likedTrackIds.has(rec.external_track_id) ||
      (localId && likedTrackIds.has(localId))) continue;

    if (downloads.some((task) => {
      if (task.status === "FAILED" || task.status === "CANCELLED") return false;
      const target = downloadTargets[task.id];
      return sameSongMetadata(
        target?.title || task.title,
        target?.artist || task.artist,
        rec.title,
        rec.artist
      );
    })) continue;

    const key = songKey(rec);
    if (seenSongs.has(key)) continue;
    seenSongs.add(key);
    (isTasteBased(rec) ? personal : other).push(rec);
  }

  // Pure Trending Songs: authentic global and chart hits, unaffected by user taste
  // For You: personalized recommendations shaped by listening history, library artists, and top genres
  return {
    forYou: personal.length > 0 ? personal : other.slice(0, limit),
    trending: other.length > 0 ? other : recommendations,
  };
};
