import { lazy } from "react";

// Views loaded on first visit, as they were in App.
export const DownloadsView = lazy(() => import("../../views/DownloadsView").then((m) => ({ default: m.DownloadsView })));
export const WishlistView = lazy(() => import("../../views/WishlistView").then((m) => ({ default: m.WishlistView })));
export const DiscoveryView = lazy(() => import("../../views/DiscoveryView").then((m) => ({ default: m.DiscoveryView })));
export const NotificationsView = lazy(() => import("../../views/NotificationsView").then((m) => ({ default: m.NotificationsView })));
export const StatsView = lazy(() => import("../../views/StatsView").then((m) => ({ default: m.StatsView })));
export const LyricsView = lazy(() => import("../../views/LyricsView").then((m) => ({ default: m.LyricsView })));
