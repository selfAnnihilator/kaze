import { useEffect, useRef } from "react";
import { dragStarted, shouldOpen } from "./dragMath";
import { collapsedY, type SheetDrag } from "./useDeckSheet";

type Point = { x: number; y: number; t: number };
const point = (t: Touch, time: number): Point => ({ x: t.clientX, y: t.clientY, t: time });

/** How far open the Deck is for a finger that moved `dy` from where it touched down. */
const travelFor = (direction: "up" | "down", dy: number) => (direction === "up" ? -dy : collapsedY() - dy);

/** Touch handlers for one drag gesture; nothing is prevented until it really is a drag. */
function gesture(direction: "up" | "down", drag: () => SheetDrag) {
  let start: Point | null = null;
  let last: Point | null = null;
  let speed = 0;
  let active = false;
  const down = (e: TouchEvent) => { start = last = point(e.touches[0], e.timeStamp); active = false; speed = 0; };
  const move = (e: TouchEvent) => {
    if (!start || !last) return;
    const now = point(e.touches[0], e.timeStamp);
    if (!active && !dragStarted(now.x - start.x, now.y - start.y, direction)) return;
    if (!active) { active = true; drag().start(); }
    e.preventDefault();
    speed = (last.y - now.y) / Math.max(1, now.t - last.t); // toward open = finger going up
    last = now;
    drag().move(travelFor(direction, now.y - start.y));
  };
  const up = (e: TouchEvent) => {
    if (active && start && last) {
      e.preventDefault(); // the release must not also tap the button under the finger
      drag().end(shouldOpen(travelFor(direction, last.y - start.y), collapsedY(), speed));
    }
    start = last = null;
    active = false;
  };
  return { down, move, up };
}

/**
 * Drags the Deck sheet with the finger from `el`: "up" from the mini player
 * opens it, "down" on the Deck's header closes it. Taps on `el` are untouched.
 */
export function useSheetDrag(el: HTMLElement | null, direction: "up" | "down", drag: SheetDrag) {
  const latest = useRef(drag);
  latest.current = drag;
  useEffect(() => {
    if (!el) return;
    const g = gesture(direction, () => latest.current);
    el.addEventListener("touchstart", g.down, { passive: true });
    el.addEventListener("touchmove", g.move, { passive: false });
    el.addEventListener("touchend", g.up, { passive: false });
    el.addEventListener("touchcancel", g.up, { passive: false });
    return () => {
      el.removeEventListener("touchstart", g.down);
      el.removeEventListener("touchmove", g.move);
      el.removeEventListener("touchend", g.up);
      el.removeEventListener("touchcancel", g.up);
    };
  }, [el, direction]);
}
