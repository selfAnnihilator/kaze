import type { UserProfile } from "./types";

export type ViewType =
  | "home"
  | "library"
  | "albums"
  | "playlists"
  | "smart_mixes"
  | "discovery"
  | "notifications"
  | "stats"
  | "settings"
  | "now_playing"
  | "queue"
  | "downloads"
  | "wishlist";

/** Props shared by the navigation rails (spatial + profile). */
export interface NavigationProps {
  currentView: ViewType;
  currentUser?: UserProfile | null;
  onSelectView: (view: ViewType) => void;
  unreadNotificationsCount?: number;
}
