export type MobileTab = "home" | "discover" | "library" | "queue";

export const MOBILE_TABS: ReadonlyArray<{ id: MobileTab; label: string }> = [
  { id: "home", label: "Home" },
  { id: "discover", label: "Discover" },
  { id: "library", label: "Library" },
  { id: "queue", label: "Queue" },
];

export function isMobileTab(value: string): value is MobileTab {
  return MOBILE_TABS.some((tab) => tab.id === value);
}
