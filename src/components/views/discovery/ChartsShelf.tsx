import { DiscoveryPreviewTarget } from "../../../presentation/DiscoveryChapters";
import { ChartCard } from "./ChartCard";
import { ShelfHeader } from "./ShelfHeader";
import { TrendingUp } from "lucide-react";
import type { DiscoveryViewContext } from "./context";

export function ChartsShelf({ c }: { c: DiscoveryViewContext }) {
  return (
    <div>
      <ShelfHeader icon={<TrendingUp size={18} color="#f3701e" />} title="Top Charts" subtitle="Current world and country top-song charts" onScroll={(d) => c.handleScroll(c.chartsScrollRef, d)} />

      <div ref={c.chartsScrollRef} className="horizontal-scroll-row">
        {c.topCharts.map((chart) => (
          <div
            key={chart.id}
            style={{ flex: "0 0 174px", width: "174px" }}
          >
            <DiscoveryPreviewTarget preview={c.chartPreview(chart)}>
            <ChartCard
              chart={chart}
              onPlay={() => c.handlePlayChartItem(chart)}
              onOpen={() => c.handleOpenChart(chart)}
            />
            </DiscoveryPreviewTarget>
          </div>
        ))}
      </div>
    </div>
  );
}
