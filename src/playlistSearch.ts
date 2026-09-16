export const normalizePlaylistSearch = (value: string): string =>
  value
    .normalize("NFKD")
    .replace(/[\u0300-\u036f]/g, "")
    .toLocaleLowerCase()
    .replace(/[^\p{L}\p{N}]+/gu, " ")
    .replace(/\s+/g, " ")
    .trim();

export const matchesPlaylistSearch = (
  track: { title: string; artist: string; album?: string | null },
  query: string
): boolean => {
  const words = normalizePlaylistSearch(query).split(" ").filter(Boolean);
  if (words.length === 0) return true;
  const text = normalizePlaylistSearch([track.title, track.artist, track.album || ""].join(" "));
  return words.every((word) => text.includes(word));
};

export const filterPlaylistEntries = <T extends { id: string; title: string; artist: string; album?: string | null }>(
  tracks: T[],
  query: string
): Array<{ track: T; originalIndex: number; rowKey: string }> =>
  tracks
    .map((track, originalIndex) => ({ track, originalIndex, rowKey: `${track.id}:${originalIndex}` }))
    .filter(({ track }) => matchesPlaylistSearch(track, query));
