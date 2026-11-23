import { useRef } from "react";

/** Where the new playing song's sleeve comes from. */
export type SlideFrom = "next" | "previous" | "other";

type Seen = { current?: string; previous?: string; next?: string; from: SlideFrom };

/** The new song was the Next sleeve (slides in from the right), the Previous one (from the left), or neither. */
export function slideFrom(newId: string | undefined, seen: Omit<Seen, "from">): SlideFrom {
  if (!newId || newId === seen.current) return "other";
  if (newId === seen.next) return "next";
  if (newId === seen.previous) return "previous";
  return "other";
}

/**
 * The class for the playing sleeve, so a tap on Next or Previous (or a song
 * ending) moves that sleeve into the middle instead of swapping it abruptly.
 */
export function useSleeveSlide(currentId?: string, previousId?: string, nextId?: string) {
  const seen = useRef<Seen>({ current: currentId, previous: previousId, next: nextId, from: "other" });
  const s = seen.current;
  if (currentId !== s.current) s.from = slideFrom(currentId, s);
  Object.assign(s, { current: currentId, previous: previousId, next: nextId });
  return `m-sleeve-slide is-from-${s.from}`;
}
