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
          ? "1px solid rgba(255, 255, 255, 0.22)"
          : "1px solid var(--border)",
        borderRadius: "12px",
        padding: "10px",
        display: "flex",
        flexDirection: "column",
        cursor: "pointer",
        position: "relative",
        transition: "transform 0.2s cubic-bezier(0.4, 0, 0.2, 1), opacity 0.2s cubic-bezier(0.4, 0, 0.2, 1)",
        boxShadow: isHovered
          ? "0 10px 24px rgba(0, 0, 0, 0.4)"
          : "0 2px 8px rgba(0, 0, 0, 0.15)",
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
          borderRadius: "8px",
          overflow: "hidden",
          background: chart.bgGradient,
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          padding: "12px",
          textAlign: "center",
        }}
      >
        {chart.image && <Artwork src={chart.image} alt={chart.title} className="chart-cover" />}
        <div
          className="chart-card-badge"
          style={{
            backgroundColor: "rgba(0,0,0,0.6)",
            padding: "3px 8px",
            borderRadius: "6px",
            fontSize: "0.68rem",
            fontWeight: 800,
            letterSpacing: "1.5px",
            color: "#e8d8c9",
            marginBottom: "6px",
          }}
        >
          CHART
        </div>
        <div
          className="chart-card-region"
          style={{
            fontSize: "1.05rem",
            fontWeight: 900,
            letterSpacing: "0.5px",
            color: "#e8d8c9",
            textShadow: "0 2px 8px rgba(0,0,0,0.6)",
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
            color: "#e8d8c9",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            boxShadow: "0 4px 14px rgba(0, 0, 0, 0.6)",
            cursor: "pointer",
            opacity: isHovered ? 1 : 0,
            transform: isHovered ? "scale(1)" : "scale(0.85)",
            transition:
              "opacity 0.2s ease, transform 0.2s ease, background-color 0.2s ease",
            zIndex: 3,
          }}
          title={`Play ${chart.title}`}
        >
          <Play size={17} fill="#e8d8c9" style={{ marginLeft: "2px" }} />
        </button>
      </div>

      <div
        style={{
          marginTop: "10px",
          fontWeight: 700,
          fontSize: "0.88rem",
          color: "#e8d8c9",
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
          marginTop: "3px",
          fontWeight: 400,
          fontSize: "0.76rem",
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
