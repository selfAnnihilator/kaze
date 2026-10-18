import { useEffect, useLayoutEffect, useRef, type RefObject } from "react";
import type { NavigationDestination } from "../navigationHistory";

/** Scroll belongs to navigation, not to the selected interface or playback clock. */
export function useRouteScroll(ref: RefObject<HTMLElement | null>, destination: NavigationDestination) {
  const key = JSON.stringify([destination.view, destination.collection?.type, destination.collection?.id, destination.lyrics, destination.libraryQuery, destination.searchQuery]);
  const current = useRef(key);
  const positions = useRef(new Map<string, number>());
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const remember = () => positions.current.set(current.current, el.scrollTop);
    el.addEventListener("scroll", remember, { passive: true });
    return () => el.removeEventListener("scroll", remember);
  }, [ref]);
  useLayoutEffect(() => {
    if (current.current === key) return;
    current.current = key;
    if (ref.current) ref.current.scrollTop = positions.current.get(key) || 0;
    if (positions.current.size > 100) positions.current.delete(positions.current.keys().next().value!);
  }, [key, ref]);
}
