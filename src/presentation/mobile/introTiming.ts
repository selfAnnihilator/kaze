/** The phone's startup screen shows for at least this long after launch, so it reads as an intro, not a flash. */
export const MIN_SHOW_MS = 3000;

/** How much longer the startup screen must stay, given the time since app launch (`performance.now()`). */
export function remainingIntro(sinceLaunchMs: number): number {
  return Math.max(0, MIN_SHOW_MS - sinceLaunchMs);
}
