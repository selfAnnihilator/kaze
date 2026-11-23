// The artwork each recently playing song actually showed, so the Previous
// sleeve shows the very same image once the song moves on.
const MAX = 64;
const shown = new Map<string, string>();

export const shownArtwork = {
  remember(trackId: string, url: string | null | undefined): void {
    if (!url) return;
    shown.delete(trackId);
    shown.set(trackId, url);
    if (shown.size > MAX) shown.delete(shown.keys().next().value as string);
  },
  get(trackId: string | undefined): string | undefined {
    return trackId ? shown.get(trackId) : undefined;
  },
};
