import { WindowedStrip } from "../../../presentation/WindowedStrip";
import { ShelfHeader } from "./ShelfHeader";
import { Music2, Sparkles } from "lucide-react";
import type { DiscoveryViewContext } from "./context";

export function TrendingShelf({ c }: { c: DiscoveryViewContext }) {
  return (
    <div>
      <ShelfHeader icon={<Music2 size={18} color="var(--accent-light)" />} title="Trending Songs" subtitle="Popular songs from the charts" arrowGap="8px" onScroll={(d) => c.handleScroll(c.songsScrollRef, d)} />

      {c.trending.length > 0 && c.trendingError && (
        <p role="alert">{c.trendingError}</p>
      )}
      {c.trending.length === 0 ? (
        <div
          className="content-card"
          role="status"
          aria-busy={c.isTrendingLoading}
          style={{
            textAlign: "center",
            padding: "45px 20px",
            color: "var(--text-dim)",
            borderStyle: "dashed",
          }}
        >
          <Sparkles
            size={36}
            color="var(--accent-light)"
            style={{ marginBottom: "10px" }}
          />
          <p
            style={{
              fontSize: "1.05rem",
              fontWeight: 600,
              color: "var(--text-main)",
              marginBottom: "4px",
            }}
          >
            {c.isTrendingLoading ? "Loading trending songs…" : c.trendingError ? "Trending songs unavailable" : "No trending songs found"}
          </p>
          <p
            style={{
              fontSize: "0.85rem",
              color: "var(--text-muted)",
              maxWidth: "480px",
              margin: "0 auto",
            }}
          >
            {c.isTrendingLoading ? "Finding the latest songs for you." : c.trendingError || "Click Refresh to discover fresh trending tracks."}
          </p>
        </div>
      ) : (
        <WindowedStrip
          items={c.trending}
          scrollRef={c.songsScrollRef}
          onScroll={c.handleSongsScroll}
          label="Trending Songs"
          initialScrollLeft={() => c.viewMemory.shelves.get(0) || 0}
          renderItem={c.renderSongCard}
        />
      )}
    </div>
  );
}
