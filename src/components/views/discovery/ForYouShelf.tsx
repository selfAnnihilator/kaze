import { WindowedStrip } from "../../../presentation/WindowedStrip";
import { ShelfHeader } from "./ShelfHeader";
import { Sparkles } from "lucide-react";
import type { DiscoveryViewContext } from "./context";

export function ForYouShelf({ c }: { c: DiscoveryViewContext }) {
  return (
    <section aria-labelledby="for-you-heading">
      <ShelfHeader headingId="for-you-heading" icon={<Sparkles size={18} color="var(--accent-secondary)" />} title="For You" subtitle="Songs picked from your taste that you have not saved or downloaded" showArrows={c.forYou.length > 0} arrowLabel="For You" onScroll={(d) => c.handleScroll(c.forYouScrollRef, d)} />

      {c.forYou.length > 0 ? (
        <WindowedStrip
          items={c.forYou}
          scrollRef={c.forYouScrollRef}
          label="For You"
          initialScrollLeft={() => c.viewMemory.shelves.get(1) || 0}
          renderItem={c.renderSongCard}
        />
      ) : (
        <div
          className="content-card"
          style={{
            padding: "var(--space-24)",
            color: "var(--text-muted)",
            borderStyle: "dashed",
          }}
        >
          Your recommendations are getting ready. Try Refresh, or choose your music taste in Settings.
        </div>
      )}
    </section>
  );
}
