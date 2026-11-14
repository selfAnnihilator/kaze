import type { ReactNode } from "react";
import type { ChartItem } from "./ChartCard";
import type { DiscoveryPreview } from "../../../presentation/DiscoveryChapters";
import type { DiscoveryRecommendation } from "../../../types";
import type { MixItem } from "./MixCard";
import type { RefObject } from "react";
export interface DiscoveryViewContext {
  handleScroll: (ref: RefObject<HTMLDivElement | null>, direction: "left" | "right") => void;
  songsScrollRef: RefObject<HTMLDivElement | null>;
  trending: DiscoveryRecommendation[];
  trendingError: string | null;
  isTrendingLoading: boolean;
  handleSongsScroll: () => void;
  viewMemory: { chapter: number; shelves: Map<number, number>; hovered: { chapter: number; id: string; } | null; };
  renderSongCard: (rec: DiscoveryRecommendation) => ReactNode;
  forYou: DiscoveryRecommendation[];
  forYouScrollRef: RefObject<HTMLDivElement | null>;
  mixesScrollRef: RefObject<HTMLDivElement | null>;
  allMixes: MixItem[];
  mixPreview: (mix: MixItem) => DiscoveryPreview;
  handlePlayMixItem: (mix: MixItem) => void;
  handleOpenMix: (mix: MixItem) => void;
  chartsScrollRef: RefObject<HTMLDivElement | null>;
  topCharts: ChartItem[];
  chartPreview: (chart: ChartItem) => DiscoveryPreview;
  handlePlayChartItem: (chart: ChartItem) => void;
  handleOpenChart: (chart: ChartItem) => void;
}
