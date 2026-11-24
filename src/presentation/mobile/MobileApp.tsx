import { useCallback, useState } from "react";
import { InterfaceContext } from "../InterfaceContext";
import { CollectionPane } from "./CollectionPane";
import { MiniDeck } from "./MiniDeck";
import { MobileDeck } from "./MobileDeck";
import { MobileDiscover } from "./MobileDiscover";
import { MobileHome } from "./MobileHome";
import { MobileLibrary } from "./MobileLibrary";
import { MobileLyrics } from "./MobileLyrics";
import { MobileStartup } from "./MobileStartup";
import { MobilePageView } from "./MobilePageView";
import { MobileQueue } from "./MobileQueue";
import { TabBar } from "./TabBar";
import { TrackSheets } from "./sheets/TrackSheets";
import { isPlayingLiked, togglePlayingLike } from "./likeTrack";
import { MobileNavContext } from "./mobileNav";
import { useMobileShell } from "./useMobileShell";
import { usePresence } from "./usePresence";
import { useDeckSheet } from "./deck/useDeckSheet";
import { useSheetDrag } from "./deck/useSheetDrag";
import type { MobileTab } from "./tabs";
import type { MobileProps, MobileScreenProps } from "./types";
import "./mobile.css";
import "./mobile-home.css";
import "./mobile-discover.css";
import "./mobile-library.css";
import "./mobile-deck.css";
import "./mobile-pages.css";
import "./mobile-settings.css";
import "./mobile-profile.css";
import "./mobile-lyrics.css";
import "./mobile-motion.css";
import "./mobile-pull.css";
import { useCanRate } from "../../services/ratingRule";

const SCREENS: Record<MobileTab, React.ComponentType<MobileScreenProps>> = {
  home: MobileHome,
  discover: MobileDiscover,
  library: MobileLibrary,
  queue: MobileQueue,
};

/**
 * Mobile shell (Figma "Kaze Mobile — Brutalist"): one tab screen, or a page or
 * open collection over it, the Mini Deck above the Tab Bar, the full Deck (which grows out of the Mini
 * Deck and can be dragged up from it), and
 * the app's shared overlays (sign in, toasts, dialogs). Android Back closes the
 * top layer.
 */
export function MobileApp({ c, player, deck, overlays }: MobileProps) {
  const { tab, page, deckOpen, nav, closeTop } = useMobileShell(c);
  const handleLike = useCallback(() => togglePlayingLike(player), [player]);
  const lyrics = usePresence(page === "lyrics");
  const sheet = useDeckSheet(deckOpen, { open: nav.openDeck, close: closeTop });
  const [mini, setMini] = useState<HTMLDivElement | null>(null);
  const [deckHeader, setDeckHeader] = useState<HTMLElement | null>(null);
  useSheetDrag(mini, "up", sheet.drag);
  useSheetDrag(deckHeader, "down", sheet.drag);

  const Screen = SCREENS[tab];
  const screenKey = c.activeCollection ? "collection" : page && page !== "lyrics" ? `page:${page}` : `tab:${tab}`;
  let body = <Screen c={c} />;
  if (c.activeCollection) body = <CollectionPane c={c} />;
  else if (page && page !== "lyrics") body = <MobilePageView c={c} page={page} />;

  const canLike = useCanRate(player.currentTrack);
  return (
    <InterfaceContext.Provider value="mobile">
      <MobileNavContext.Provider value={nav}>
        <div className="app-container kaze-interface m-shell" data-interface="mobile">
          <main className="m-screen"><div key={screenKey} className={`m-screen__enter ${screenKey.startsWith("tab:") ? "" : "is-page"}`}>{body}</div></main>

          <MiniDeck
            track={player.currentTrack}
            coverArtUrl={player.coverArtUrl}
            isPlaying={player.playbackState.is_playing}
            isLoading={!!player.isLoading}
            liked={isPlayingLiked(player)}
            onOpen={nav.openDeck}
            onPlayPause={player.onPlayPause}
            onNext={player.onNext}
            onLike={handleLike}
            canLike={canLike}
            rootRef={setMini}
          />

          <TabBar active={tab} onSelect={nav.goTab} />

          {sheet.mounted && <MobileDeck c={c} player={player} deck={deck} onClose={closeTop} sheetRef={sheet.ref} headerRef={setDeckHeader} />}
          {lyrics.mounted && <MobileLyrics player={player} onClose={closeTop} leaving={lyrics.leaving} />}
          <TrackSheets c={c} />
          {overlays}
          <MobileStartup ready={!!c.onboardingStatus} />
        </div>
      </MobileNavContext.Provider>
    </InterfaceContext.Provider>
  );
}
