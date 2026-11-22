import { useCallback, useState } from "react";
import { InterfaceContext } from "../InterfaceContext";
import { MiniDeck } from "./MiniDeck";
import { MobileDeck } from "./MobileDeck";
import { MobileDiscover } from "./MobileDiscover";
import { MobileHome } from "./MobileHome";
import { MobileLibrary } from "./MobileLibrary";
import { MobileQueue } from "./MobileQueue";
import { TabBar } from "./TabBar";
import type { MobileTab } from "./tabs";
import type { MobileProps, MobileScreenProps } from "./types";
import "./mobile.css";

const SCREENS: Record<MobileTab, React.ComponentType<MobileScreenProps>> = {
  home: MobileHome, discover: MobileDiscover, library: MobileLibrary, queue: MobileQueue,
};

/** Mobile shell: one screen, the mini deck and the tab bar; the Deck opens over them. */
export function MobileApp({ c, player }: MobileProps) {
  const [tab, setTab] = useState<MobileTab>("home");
  const [deckOpen, setDeckOpen] = useState(false);
  const openDeck = useCallback(() => setDeckOpen(true), []);
  const closeDeck = useCallback(() => setDeckOpen(false), []);
  const Screen = SCREENS[tab];
  return (
    <InterfaceContext.Provider value="mobile">
      <div className="app-container kaze-interface m-shell" data-interface="mobile">
        <main className="m-screen"><Screen c={c} /></main>
        <MiniDeck track={player.currentTrack} coverArtUrl={player.coverArtUrl}
          isPlaying={player.playbackState.is_playing} isLoading={!!player.isLoading}
          onOpen={openDeck} onPlayPause={player.onPlayPause} />
        <TabBar active={tab} onSelect={setTab} />
        {deckOpen && <MobileDeck player={player} onClose={closeDeck} />}
      </div>
    </InterfaceContext.Provider>
  );
}
