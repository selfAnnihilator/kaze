export function mergeArtistChoices(selected: string[], suggestions: string[]): string[] {
  const seen = new Set<string>();
  return [...selected, ...suggestions].filter((name) => {
    const key = name.trim().toLowerCase();
    return key && !seen.has(key) && seen.add(key);
  }).slice(0, 30);
}
