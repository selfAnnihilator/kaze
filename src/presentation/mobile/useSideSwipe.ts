import { useEffect, useRef, type RefObject } from "react";
import { sideStep } from "./discover/swipeStep";

/**
 * A sideways swipe on `ref` becomes one step: left is forward (+1), right is
 * back (-1). Vertical moves are left to the page's scrolling.
 */
export function useSideSwipe(ref: RefObject<HTMLElement | null>, onStep: (step: 1 | -1) => void) {
  const latest = useRef(onStep);
  latest.current = onStep;
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    let start: { x: number; y: number } | null = null;
    const down = (e: TouchEvent) => { start = { x: e.touches[0].clientX, y: e.touches[0].clientY }; };
    const up = (e: TouchEvent) => {
      if (!start) return;
      const t = e.changedTouches[0];
      const s = sideStep(t.clientX - start.x, t.clientY - start.y);
      start = null;
      if (s) latest.current(s);
    };
    el.addEventListener("touchstart", down, { passive: true });
    el.addEventListener("touchend", up, { passive: true });
    return () => {
      el.removeEventListener("touchstart", down);
      el.removeEventListener("touchend", up);
    };
  });
}
