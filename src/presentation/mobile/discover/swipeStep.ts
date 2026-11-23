import { PULL_FLICK, pullArmed } from "../pullRefresh/pullMath.ts";

/** Least vertical travel, in CSS pixels, that turns a swipe into a chapter change. */
export const SWIPE_MIN = 48;
/** Wheel travel that turns into one chapter change (same gate as the PC). */
export const WHEEL_MIN = 40;

/**
 * A finished touch gesture as a chapter step: swiping up (content moving up)
 * goes to the next chapter, swiping down to the previous one. Mostly-sideways
 * swipes belong to the card rows and return 0.
 */
export function swipeStep(dx: number, dy: number, min = SWIPE_MIN): -1 | 0 | 1 {
  if (Math.abs(dy) < min || Math.abs(dy) <= Math.abs(dx)) return 0;
  return dy < 0 ? 1 : -1;
}

/** Keeps a chapter index inside 0…count-1. */
export const clampChapter = (index: number, count: number) => Math.max(0, Math.min(count - 1, index));

/**
 * A finished downward swipe on Discover: a long, slow pull from the top
 * refreshes the chapter; a quick swipe down goes back a chapter.
 */
export function discoverPullAction(dx: number, dy: number, ms: number, atTop: boolean): "refresh" | "previous" | null {
  if (atTop && pullArmed(dy) && dy / Math.max(ms, 1) < PULL_FLICK && dy > Math.abs(dx)) return "refresh";
  return swipeStep(dx, dy) === -1 ? "previous" : null;
}

/** A finished sideways swipe as a step: left goes forward (+1), right goes back (-1). */
export function sideStep(dx: number, dy: number, min = SWIPE_MIN): -1 | 0 | 1 {
  if (Math.abs(dx) < min || Math.abs(dx) <= Math.abs(dy)) return 0;
  return dx < 0 ? 1 : -1;
}
