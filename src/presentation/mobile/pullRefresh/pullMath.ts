/** Finger travel, in CSS pixels, past which letting go refreshes. */
export const PULL_TRIGGER = 96;
/** The indicator never travels further than this. */
export const PULL_MAX = 72;
/** Finger travel before a downward move counts as a pull (taps stay taps). */
export const PULL_START = 8;
/** Faster than this (px/ms) a downward swipe is a flick, not a pull. */
export const PULL_FLICK = 0.5;

/** Rubber band: the indicator moves at half the finger's speed, up to PULL_MAX. */
export const resist = (dy: number) => Math.min(PULL_MAX, Math.max(0, dy) / 2);

/** A pull starts only at the top of the page, moving down, mostly vertically. */
export const pullStarted = (dx: number, dy: number, scrollTop: number) =>
  scrollTop <= 0 && dy > PULL_START && dy > Math.abs(dx);

/** Whether letting go now would refresh. */
export const pullArmed = (dy: number) => dy >= PULL_TRIGGER;
