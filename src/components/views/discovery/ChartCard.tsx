import { Artwork } from "../../../presentation/Artwork";
import { Play } from "lucide-react";
import React, { useState } from "react";

export interface ChartItem {
  id: string;
  title: string;
  subtitle: string;
  chartNumber: string;
  region: string;
  image?: string;
  bgGradient: string;
  badgeBg: string;
}

export const ChartCard: React.FC<{
  chart: ChartItem;
  onPlay: () => void;
  onOpen?: () => void;
}> = ({ chart, onPlay, onOpen }) => {
  const [isHovered, setIsHovered] = useState(false);

  return (
    <div
      className="chart-card"
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      onClick={onOpen || onPlay}
      style={{
        backgroundColor: "var(--bg-card)",
        border: isHovered
          ? "1px solid var(--overlay-20)"
          : "1px solid var(--border)",
        borderRadius: "var(--radius-12)",
        padding: "var(--space-10)",
        display: "flex",
        flexDirection: "column",
        cursor: "pointer",
        position: "relative",
        transition: "transform var(--duration-base) var(--ease-standard), opacity var(--duration-base) var(--ease-standard)",
        boxShadow: isHovered
          ? "0 10px 24px var(--shade-40)"
          : "0 2px 8px var(--shade-15)",
        width: "100%",
        height: "100%",
      }}
    >
      <div
        className="chart-card-art"
        style={{
          position: "relative",
          width: "100%",
          aspectRatio: "1 / 1",
          borderRadius: "var(--radius-8)",
          overflow: "hidden",
          background: chart.bgGradient,
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          padding: "var(--space-12)",
          textAlign: "center",
        }}
      >
        {chart.image && <Artwork src={chart.image} alt={chart.title} className="chart-cover" />}
        <div
          className="chart-card-badge"
          style={{
            backgroundColor: "var(--shade-60)",
            padding: "var(--space-4) var(--space-8)",
            borderRadius: "var(--radius-6)",
            fontSize: "var(--fs-11)",
            fontWeight: "var(--weight-extrabold)",
            letterSpacing: "1.5px",
            color: "var(--text-main)",
            marginBottom: "var(--space-6)",
          }}
        >
          CHART
        </div>
        <div
          className="chart-card-region"
          style={{
            fontSize: "var(--fs-16)",
            fontWeight: "var(--weight-black)",
            letterSpacing: "0.5px",
            color: "var(--text-main)",
            textShadow: "0 2px 8px var(--shade-60)",
          }}
        >
          {chart.chartNumber}
        </div>

        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation();
            onPlay();
          }}
          style={{
            position: "absolute",
            bottom: "8px",
            left: "8px",
            width: "38px",
            height: "38px",
            borderRadius: "50%",
            backgroundColor: chart.badgeBg || "var(--accent)",
            border: "none",
            color: "var(--text-main)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            boxShadow: "0 4px 14px var(--shade-60)",
            cursor: "pointer",
            opacity: isHovered ? 1 : 0,
            transform: isHovered ? "scale(1)" : "scale(0.85)",
            transition:
              "opacity var(--duration-base) ease, transform var(--duration-base) ease, background-color var(--duration-base) ease",
            zIndex: 3,
          }}
          title={`Play ${chart.title}`}
        >
          <Play size={17} fill="var(--text-main)" style={{ marginLeft: "var(--space-2)" }} />
        </button>
      </div>

      <div
        style={{
          marginTop: "var(--space-10)",
          fontWeight: "var(--weight-bold)",
          fontSize: "var(--fs-14)",
          color: "var(--text-main)",
          whiteSpace: "nowrap",
          overflow: "hidden",
          textOverflow: "ellipsis",
          lineHeight: "1.3",
        }}
        title={chart.title}
      >
        {chart.title}
      </div>

      <div
        style={{
          marginTop: "var(--space-4)",
          fontWeight: "var(--weight-regular)",
          fontSize: "var(--fs-12)",
          color: "var(--text-muted)",
          whiteSpace: "nowrap",
          overflow: "hidden",
          textOverflow: "ellipsis",
          lineHeight: "1.3",
        }}
        title={chart.subtitle}
      >
        {chart.subtitle}
      </div>
    </div>
  );
};
