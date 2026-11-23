import { createContext, useCallback, useContext, useEffect, useRef } from "react";
import type { MobileTab } from "./tabs";

/** Screens that open over the tabs (back bar on top, tab bar still below). */
export type MobilePage = "settings" | "notifications" | "profile" | "lyrics" | "smart_mixes";

export type LibraryFocusTab = "albums" | "local" | "downloads" | "playlists" | "liked";

export interface MobileNav {
  openPage: (page: MobilePage) => void;
  openSearch: () => void;
  openDeck: () => void;
  goTab: (tab: MobileTab) => void;
  openAlbum: (albumId: string) => void;
  /** Close the top layer (page, Deck or collection) like the phone's Back button. */
  back: () => void;
  /** Bumped by openSearch so Discover focuses its field. */
  searchNonce: number;
  /** Album the Library should show next (from the Deck's "Open album"). */
  albumFocus: string | null;
  /** Library tab to show next (e.g. Downloads from the offline screen); `n` makes repeats count. */
  libraryFocus: { tab: LibraryFocusTab; n: number } | null;
  openLibrary: (tab: LibraryFocusTab) => void;
  unread: number;
  /** Initials of the signed-in user, or null when signed out. */
  initials: string | null;
  /** The signed-in user's uploaded photo, shown in the top bar tile. */
  avatarUrl?: string | null;
  /** Profile (Figma F1); a guest sees "Guest listener" and can sign in from Settings. */
  openProfile: () => void;
}

const noop = () => {};
export const MobileNavContext = createContext<MobileNav>({
  openPage: noop, openSearch: noop, openDeck: noop, goTab: noop, openAlbum: noop, back: noop,
  searchNonce: 0, albumFocus: null, libraryFocus: null, openLibrary: noop, unread: 0, initials: null, openProfile: noop,
});
export const useMobileNav = () => useContext(MobileNavContext);

export type BackLayer = "deck" | "page" | "collection";

/**
 * Ties open layers to browser history so Android's Back button closes the top
 * layer instead of leaving the app. Each layer is one history entry that records
 * its depth; whenever history moves back (hardware Back, `back`, or `clear`),
 * every layer above the new depth is closed through `onClose`.
 */
export function useBackLayers(onClose: (layer: BackLayer) => void) {
  const stack = useRef<BackLayer[]>([]);
  // Entries left from before a reload (e.g. after logging out) belong to another session: depth 0.
  const session = useRef(`${Date.now()}:${Math.random()}`);
  const closeRef = useRef(onClose);
  closeRef.current = onClose;

  useEffect(() => {
    const onPop = (event: PopStateEvent) => {
      const own = event.state?.kazeSession === session.current;
      const depth = own && typeof event.state?.kazeDepth === "number" ? event.state.kazeDepth : 0;
      while (stack.current.length > depth) {
        const layer = stack.current.pop();
        if (layer) closeRef.current(layer);
      }
    };
    window.addEventListener("popstate", onPop);
    return () => window.removeEventListener("popstate", onPop);
  }, []);

  const push = useCallback((layer: BackLayer) => {
    stack.current.push(layer);
    window.history.pushState({ kazeDepth: stack.current.length, kazeSession: session.current }, "");
  }, []);
  const has = useCallback((layer: BackLayer) => stack.current.includes(layer), []);
  const back = useCallback(() => {
    if (stack.current.length) window.history.back();
  }, []);
  /** The top layer becomes another one in the same history entry (Deck → page). */
  const replaceTop = useCallback((layer: BackLayer) => {
    if (stack.current.length) stack.current[stack.current.length - 1] = layer;
  }, []);
  /** Close every layer (switching tabs). */
  const clear = useCallback(() => {
    if (stack.current.length) window.history.go(-stack.current.length);
  }, []);
  return { push, back, has, clear, replaceTop };
}
