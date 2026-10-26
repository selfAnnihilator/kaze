import type { DiscoveryRecommendation, Track } from "./types";

const normalize = (value: string): string =>
  value.normalize("NFKD").replace(/[\u0300-\u036f]/g, "")
    .toLowerCase().replace(/[^\p{L}\p{N}]+/gu, " ").trim().replace(/\s+/g, " ");

const artistCredits = (value: string): string[] =>
  value.split(/\s*(?:,|&|\/|\b(?:and|with|feat\.?|ft\.?|x)\b)\s*/i)
    .map(normalize).filter(Boolean).sort();

// Artist order varies between catalogs and local tags. Require the same full
// credit set so a cover with the same title cannot be mistaken for the song.
export const sameSongMetadata = (
  firstTitle: string, firstArtist: string, secondTitle: string, secondArtist: string
): boolean => {
  if (!normalize(firstTitle) || normalize(firstTitle) !== normalize(secondTitle)) return false;
  const first = artistCredits(firstArtist);
  const second = artistCredits(secondArtist);
  return first.length > 0 && first.length === second.length && first.every((credit, index) => credit === second[index]);
};

export const findLocalSearchMatch = (rec: DiscoveryRecommendation, tracks: Track[]): Track | undefined => {
  if (rec.provider === "jiosaavn" || rec.external_track_id.startsWith("jiosaavn:")) return undefined;
  if (rec.matched_local_track_id) {
    const linked = tracks.find((track) => track.id === rec.matched_local_track_id);
    if (linked && linked.format !== "online" && !linked.file_path.startsWith("online://")) return linked;
  }

  return tracks.find((track) => {
    if (track.format === "online" || track.file_path.startsWith("online://")) return false;
    if (!sameSongMetadata(rec.title, rec.artist, track.title, track.artist_name || "")) return false;
    return !rec.duration_secs || rec.duration_secs <= 35 || !track.duration_secs ||
      Math.abs(rec.duration_secs - track.duration_secs) <= 12;
  });
};

export const attachLocalSearchMatches = (
  results: DiscoveryRecommendation[] | null, tracks: Track[]
): DiscoveryRecommendation[] | null => results?.map((rec) => {
  const local = findLocalSearchMatch(rec, tracks);
  return local ? { ...rec, match_status: "EXACT_MATCH" as const, matched_local_track_id: local.id } : rec;
}) ?? null;

export interface PlaybackCandidate {
  track_id?: string;
  id?: string;
  external_track_id?: string;
  matched_local_track_id?: string;
  title: string;
  artist_name?: string;
  artist?: string;
  duration_secs?: number;
}

export const isCurrentPlaybackTrack = (candidate: PlaybackCandidate, current?: Track | null): boolean => {
  if (!current) return false;
  const ids = [candidate.track_id, candidate.id, candidate.external_track_id, candidate.matched_local_track_id].filter(Boolean);
  if (ids.includes(current.id)) return true;
  if (ids.some(id => id?.startsWith("jiosaavn:")) || current.id.startsWith("jiosaavn:")) return false;
  // Only catalog/online aliases may fall back to metadata. Distinct local files
  // with the same tags remain distinct tracks.
  const online = (id: string) => id.startsWith("online:") || id.startsWith("itunes:");
  if (!(ids.some(id => online(id!)) || online(current.id) || current.format === "online" || current.file_path.startsWith("online://"))) return false;
  if (!sameSongMetadata(candidate.title, candidate.artist_name || candidate.artist || "", current.title, current.artist_name || "")) return false;
  return !candidate.duration_secs || candidate.duration_secs <= 35 || !current.duration_secs || Math.abs(candidate.duration_secs - current.duration_secs) <= 12;
};
