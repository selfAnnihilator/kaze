/** Finger travel, in CSS pixels, before a press on the mini player becomes a drag. */
export const DRAG_START = 10;
/** Release speed, in px per ms, that opens or closes the Deck whatever the distance. */
export const FLICK = 0.5;

/**
 * Whether a finger that moved (dx, dy) since touching down is dragging the
 * Deck: far enough in `direction` ("up" opens, "down" closes) and more
 * vertical than sideways. Anything less stays a tap.
 */
export function dragStarted(dx: number, dy: number, direction: "up" | "down"): boolean {
  const along = direction === "up" ? -dy : dy;
  return along >= DRAG_START && Math.abs(dy) > Math.abs(dx);
}

/**
 * Whether a released drag ends with the Deck open. `travel` is how far it is
 * open (0 to `distance`); `velocity` is the release speed toward open (px/ms).
 * A flick wins; otherwise past halfway opens and short of it collapses.
 */
export function shouldOpen(travel: number, distance: number, velocity: number): boolean {
  if (velocity > FLICK) return true;
  if (velocity < -FLICK) return false;
  return travel > distance / 2;
}
