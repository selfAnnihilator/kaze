import { DiscoveryPreviewTarget } from "../../../presentation/DiscoveryChapters";
import { WindowedStrip } from "../../../presentation/WindowedStrip";
import { MixCard } from "./MixCard";
import { ShelfHeader } from "./ShelfHeader";
import { Sparkles } from "lucide-react";
import type { DiscoveryViewContext } from "./context";

export function MixesShelf({ c }: { c: DiscoveryViewContext }) {
  return (
    <div>
      <ShelfHeader icon={<Sparkles size={18} color="var(--accent-secondary)" />} title="Mixes For You" subtitle="Personalized blends and custom smart mixes curated for your listening style" onScroll={(d) => c.handleScroll(c.mixesScrollRef, d)} />

      <div>
        {c.allMixes.length === 0 && (
          <div
            className="content-card"
            style={{
              padding: "24px",
              color: "var(--text-muted)",
              borderStyle: "dashed",
            }}
          >
            Your mixes are getting ready. Try Refresh, or choose your music taste in Settings.
          </div>
        )}
        {c.allMixes.length > 0 && <WindowedStrip items={c.allMixes} scrollRef={c.mixesScrollRef} label="Mixes For You" initialScrollLeft={() => c.viewMemory.shelves.get(2) || 0} renderItem={mix => <DiscoveryPreviewTarget preview={c.mixPreview(mix)}><MixCard mix={mix} onPlay={() => c.handlePlayMixItem(mix)} onOpen={() => c.handleOpenMix(mix)} /></DiscoveryPreviewTarget>} />}
      </div>
    </div>
  );
}
