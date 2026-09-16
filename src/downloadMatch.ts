import type { DownloadSearchResult } from "./types";

const normalize = (value: string): string =>
  value.toLowerCase().normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/[^a-z0-9\s]/g, " ")
    .replace(/\s+/g, " ")
    .trim();

/** Order provider results by likely relevance without hiding possible sources. */
export const rankResultMatch = (
  result: DownloadSearchResult,
  targetArtist: string,
  targetTitle: string
): number => {
  const cleanFilename = normalize(result.filename);
  const cleanArtist = normalize(targetArtist);
  const cleanTitle = normalize(targetTitle);
  const titleWords = cleanTitle.split(" ").filter((word) => word.length > 1);
  const artistWords = cleanArtist.split(" ").filter((word) => word.length > 1);
  const filenameWords = cleanFilename.split(" ").filter((word) => word.length > 1);
  let score = 0;

  const isLofi =
    cleanTitle.includes("lofi") || cleanTitle.includes("lo fi") ||
    cleanArtist.includes("lofi") || cleanArtist.includes("lo fi") ||
    cleanFilename.includes("lofi") || cleanFilename.includes("lo fi") ||
    cleanFilename.includes("chillhop");

  if (cleanTitle && cleanFilename.includes(cleanTitle)) {
    score += 60;
  } else if (titleWords.length > 0) {
    const matches = titleWords.filter((word) => cleanFilename.includes(word)).length;
    score += matches === titleWords.length ? 50 : (matches / titleWords.length) * 30 - 25;
  }

  const artistMatches = cleanArtist && (
    cleanFilename.includes(cleanArtist) || normalize(result.username).includes(cleanArtist)
  );
  if (artistMatches) {
    score += 35;
  } else if (isLofi && /lofi|lo fi|chill|sleep/.test(cleanFilename)) {
    score += 20;
  } else if (cleanArtist) {
    score -= 45;
  }

  const commonMusicNoise = new Set([
    "lofi", "remix", "ost", "edit", "audio", "flac", "mp3", "track",
    "official", "theme", "original", "cover",
  ]);
  const extraWords = filenameWords.filter((word) =>
    !titleWords.includes(word) && !artistWords.includes(word) &&
    !commonMusicNoise.has(word) && !/^\d+$/.test(word)
  );
  if (extraWords.length > 2) score -= (extraWords.length - 2) * 18;

  if (result.format.toLowerCase() === "flac") score += 10;
  else if (result.bitrate && result.bitrate >= 320) score += 6;
  if (result.slots_free) score += 5;
  return score;
};

export const filterAndRankDownloadResults = (
  results: DownloadSearchResult[],
  artist: string,
  title: string
): DownloadSearchResult[] => {
  // Retain the existing name for callers, but do not discard any provider hit.
  if (!title.trim()) return results;
  return [...results].sort((a, b) => rankResultMatch(b, artist, title) - rankResultMatch(a, artist, title));
};
