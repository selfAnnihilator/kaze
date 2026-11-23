import { useEffect, useRef, type RefObject } from "react";
import { WHEEL_MIN, swipeStep } from "./swipeStep";

type Step = (step: 1 | -1) => void;

/** Touch start/end handlers that turn one vertical swipe into one step. */
function touchPager(step: () => Step) {
  let start: { x: number; y: number } | null = null;
  return {
    down: (e: TouchEvent) => { start = { x: e.touches[0].clientX, y: e.touches[0].clientY }; },
    up: (e: TouchEvent) => {
      if (!start) return;
      const t = e.changedTouches[0];
      const s = swipeStep(t.clientX - start.x, t.clientY - start.y);
      start = null;
      if (s) step()(s);
    },
  };
}

/** A wheel handler that steps once per gesture (same gate as the PC chapters). */
function wheelPager(step: () => Step) {
  const gesture = { last: 0, total: 0, used: false };
  return (e: WheelEvent) => {
    if (Math.abs(e.deltaX) >= Math.abs(e.deltaY)) return;
    e.preventDefault();
    const now = performance.now();
    if (now - gesture.last > 180) Object.assign(gesture, { total: 0, used: false });
    gesture.last = now;
    gesture.total += e.deltaY;
    if (gesture.used || Math.abs(gesture.total) < WHEEL_MIN) return;
    gesture.used = true;
    step()(gesture.total > 0 ? 1 : -1);
  };
}

/**
 * Vertical swipes (and the mouse wheel) on `ref` step between chapters, one
 * step per gesture; sideways swipes are left to the card rows.
 */
export function useChapterSwipe(ref: RefObject<HTMLElement | null>, onStep: Step) {
  const latest = useRef(onStep);
  latest.current = onStep;
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const current = () => latest.current;
    const touch = touchPager(current);
    const wheel = wheelPager(current);
    el.addEventListener("touchstart", touch.down, { passive: true });
    el.addEventListener("touchend", touch.up, { passive: true });
    el.addEventListener("wheel", wheel, { passive: false });
    return () => {
      el.removeEventListener("touchstart", touch.down);
      el.removeEventListener("touchend", touch.up);
      el.removeEventListener("wheel", wheel);
    };
  }, [ref]);
}
