import { Archive } from "lucide-react";
import type { StatsViewContext } from "./context";

export function ArchivedYearHero({ c }: { c: StatsViewContext }) {
  return (
    <div
      style={{
        background: "linear-gradient(135deg, #3b2a1d 0%, var(--bg-card) 100%)",
        border: "1px solid rgba(232, 216, 201, 0.3)",
        borderRadius: "20px",
        padding: "32px",
        marginBottom: "36px",
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        flexWrap: "wrap",
        gap: "20px",
      }}
    >
      <div>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: "8px",
            color: "var(--text-main)",
            fontSize: "13px",
            fontWeight: 700,
            textTransform: "uppercase",
            letterSpacing: "0.5px",
            marginBottom: "6px",
          }}
        >
          <Archive size={16} />
          <span>Year in Review</span>
        </div>
        <h2
          style={{
            fontSize: "36px",
            fontWeight: 800,
            margin: "0 0 8px 0",
            letterSpacing: "-0.5px",
          }}
        >
          {c.viewingYear} in Review
        </h2>
        <p
          style={{
            margin: 0,
            color: "rgba(255, 255, 255, 0.6)",
            fontSize: "14px",
          }}
        >
          Daily listening totals for {c.viewingYear}, with your all-time music rankings.
        </p>
      </div>

      <div
        style={{
          background: "rgba(0, 0, 0, 0.4)",
          border: "1px solid rgba(255, 255, 255, 0.1)",
          borderRadius: "16px",
          padding: "16px 24px",
          textAlign: "center",
        }}
      >
        <div
          style={{
            fontSize: "12px",
            color: "rgba(255, 255, 255, 0.5)",
            fontWeight: 600,
            textTransform: "uppercase",
            marginBottom: "4px",
          }}
        >
          {c.isPastYear ? `Total Listening Time (${c.viewingYear})` : `Tracked in ${c.viewingYear}`}
        </div>
        <div
          style={{
            fontSize: "32px",
            fontWeight: 800,
            color: "var(--accent-light)",
          }}
        >
          {c.formatSeconds(c.stats?.total_year_seconds || 0)}
        </div>
        <div
          style={{
            fontSize: "11px",
            color: "rgba(255, 255, 255, 0.4)",
            marginTop: "4px",
          }}
        >
          {`Daily listening totals for ${c.viewingYear}`}
        </div>
      </div>
    </div>
  );
}
