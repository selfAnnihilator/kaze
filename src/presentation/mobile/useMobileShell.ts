import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useBackLayers, type BackLayer, type MobileNav, type MobilePage } from "./mobileNav";
import { userInitials } from "./format";
import type { MobileTab } from "./tabs";
import type { MobileScreenProps } from "./types";

type Ctx = MobileScreenProps["c"];

/** A collection can open from anywhere (Library, Smart Mixes, Discover): give it a Back entry. */
function useCollectionBackEntry(c: Ctx, layers: Layers) {
  const key = c.activeCollection ? `${c.activeCollection.type}:${c.activeCollection.id}` : null;
  const last = useRef<string | null>(null);
  useEffect(() => {
    if (key && key !== last.current && !layers.has("collection")) layers.push("collection");
    last.current = key;
  }, [key, layers]);
}

type Layers = ReturnType<typeof useBackLayers>;

/** Lyrics opens over the Deck (Back returns to it); other pages take the Deck's Back entry. */
function placePageLayer(layers: Layers, next: MobilePage, setDeckOpen: (open: boolean) => void) {
  if (next !== "lyrics" && layers.has("deck")) {
    layers.replaceTop("page");
    setDeckOpen(false);
  } else if (!layers.has("page")) layers.push("page");
}

/** Tab, page and Deck state plus the navigation API the screens use, all wired to Android Back. */
export function useMobileShell(c: Ctx) {
  const [tab, setTab] = useState<MobileTab>("home");
  const [page, setPage] = useState<MobilePage | null>(null);
  const [deckOpen, setDeckOpen] = useState(false);
  const [searchNonce, setSearchNonce] = useState(0);
  const [albumFocus, setAlbumFocus] = useState<string | null>(null);
  const [libraryFocus, setLibraryFocus] = useState<MobileNav["libraryFocus"]>(null);
  const { navigateBack, notifications, currentUser, setIsAuthModalOpen } = c;

  const onClose = useCallback((layer: BackLayer) => {
    if (layer === "deck") setDeckOpen(false);
    else if (layer === "page") setPage(null);
    else navigateBack();
  }, [navigateBack]);
  const layers = useBackLayers(onClose);
  useCollectionBackEntry(c, layers);

  const unread = useMemo(() => notifications.filter((n) => !n.read).length, [notifications]);
  const initials = currentUser ? userInitials(currentUser.display_name || currentUser.username) : null;
  const avatarUrl = currentUser?.avatar_data_url || null;
  const nav = useMemo<MobileNav>(() => {
    const showPage = (next: MobilePage) => { placePageLayer(layers, next, setDeckOpen); setPage(next); };
    return {
      openPage: showPage,
      openSearch: () => { layers.clear(); setTab("discover"); setSearchNonce((n) => n + 1); },
      openDeck: () => { layers.push("deck"); setDeckOpen(true); },
      // Stats belong to an account: guests get the sign-in sheet instead.
      openProfile: () => (initials ? showPage("profile") : setIsAuthModalOpen(true)),
      goTab: (next) => { layers.clear(); setTab(next); },
      openAlbum: (albumId) => { layers.clear(); setAlbumFocus(albumId); setTab("library"); },
      back: layers.back,
      searchNonce,
      albumFocus,
      libraryFocus,
      openLibrary: (tab) => { layers.clear(); setLibraryFocus((f) => ({ tab, n: (f?.n ?? 0) + 1 })); setTab("library"); },
      unread,
      initials,
      avatarUrl,
    };
  }, [layers, searchNonce, albumFocus, libraryFocus, unread, initials, avatarUrl, setIsAuthModalOpen]);

  // Logging out while on Profile leaves it: its stats belong to the account.
  useEffect(() => {
    if (!currentUser && page === "profile") layers.back();
  }, [currentUser, page, layers]);

  return { tab, page, deckOpen, nav, closeTop: layers.back };
}
