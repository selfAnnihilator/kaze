/** True for phones and tablets: Android WebViews identify themselves as such. */
export function isMobileUserAgent(userAgent: string): boolean {
  return /Android|iPhone|iPad|iPod/i.test(userAgent);
}

export function isMobileDevice(): boolean {
  return typeof navigator !== "undefined" && isMobileUserAgent(navigator.userAgent);
}
