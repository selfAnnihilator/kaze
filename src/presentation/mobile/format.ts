/** m:ss for a duration in seconds; 0:00 when unknown. */
export function formatDuration(seconds: number | undefined): string {
  if (seconds === undefined || !Number.isFinite(seconds) || seconds < 0) return "0:00";
  const whole = Math.floor(seconds);
  const secs = whole % 60;
  return `${Math.floor(whole / 60)}:${secs < 10 ? "0" : ""}${secs}`;
}

/** "Artist · Album" with whichever parts exist. */
export function joinMeta(...parts: Array<string | undefined | null>): string {
  return parts.filter((part): part is string => !!part).join(" · ");
}

/** Whether a list is long enough to need windowing (AGENTS.md: over 50). */
export const NEEDS_WINDOWING = 50;

/** "Gautam Lal" → "GL", "kaze_user" → "KA". */
export function userInitials(name: string): string {
  const words = name.trim().split(/[\s._-]+/).filter(Boolean);
  const letters = words.length > 1 ? words[0][0] + words[1][0] : name.trim().slice(0, 2);
  return letters.toUpperCase();
}

/** "1.24 GB", "312.5 MB". */
export const formatBytes = (bytes: number) =>
  bytes >= 1024 ** 3 ? `${(bytes / 1024 ** 3).toFixed(2)} GB` : `${(bytes / 1024 ** 2).toFixed(1)} MB`;
