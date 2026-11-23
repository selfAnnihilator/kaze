import { useEffect, useRef, type RefObject } from "react";
import { SWIPE_MIN, WHEEL_MIN, discoverPullAction, swipeStep } from "./swipeStep";
import type { PullView } from "../pullRefresh/pullView";

type Step = (step: 1 | -1) => void;

/** Pull-to-refresh hooks for the pager: the indicator and the chapter's refresh. */
export type ChapterPull = { view: () => PullView | null; atTop: () => boolean; refresh: () => Promise<unknown> };

type Touch0 = { x: number; y: number; at: number; top: boolean };

/**
 * Touch handlers that turn one vertical swipe into one step. Swiping down from
 * the top shows the pull indicator; a long slow pull refreshes instead of
 * going back a chapter.
 */
function touchPager(step: () => Step, pull: () => ChapterPull | undefined) {
  let start: Touch0 | null = null;
  const view = () => pull()?.view() ?? null;
  return {
    down: (e: TouchEvent) => {
      start = { x: e.touches[0].clientX, y: e.touches[0].clientY, at: performance.now(), top: pull()?.atTop() ?? false };
    },
    move: (e: TouchEvent) => {
      const t = e.touches[0];
      const dy = start ? t.clientY - start.y : 0;
      if (start?.top && dy >= SWIPE_MIN && dy > Math.abs(t.clientX - start.x)) view()?.follow(dy - SWIPE_MIN);
    },
    up: (e: TouchEvent) => {
      if (!start) return;
      const t = e.changedTouches[0];
      const dx = t.clientX - start.x;
      const dy = t.clientY - start.y;
      const action = discoverPullAction(dx, dy, performance.now() - start.at, start.top);
      const s = action ? 0 : swipeStep(dx, dy);
      start = null;
      const p = pull();
      if (action === "refresh" && p) void p.view()?.spin(p.refresh());
      else view()?.reset();
      if (action === "previous") step()(-1);
      else if (s) step()(s);
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
export function useChapterSwipe(ref: RefObject<HTMLElement | null>, onStep: Step, pull?: ChapterPull) {
  const latest = useRef(onStep);
  latest.current = onStep;
  const latestPull = useRef(pull);
  latestPull.current = pull;
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const current = () => latest.current;
    const touch = touchPager(current, () => latestPull.current);
    const wheel = wheelPager(current);
    el.addEventListener("touchstart", touch.down, { passive: true });
    el.addEventListener("touchmove", touch.move, { passive: true });
    el.addEventListener("touchend", touch.up, { passive: true });
    el.addEventListener("wheel", wheel, { passive: false });
    return () => {
      el.removeEventListener("touchstart", touch.down);
      el.removeEventListener("touchmove", touch.move);
      el.removeEventListener("touchend", touch.up);
      el.removeEventListener("wheel", wheel);
    };
  }, [ref]);
}
