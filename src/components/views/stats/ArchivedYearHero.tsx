import { Archive } from "lucide-react";
import type { StatsViewContext } from "./context";

export function ArchivedYearHero({ c }: { c: StatsViewContext }) {
  return (
    <div
      style={{
        background: "linear-gradient(135deg, var(--palette-taupe-850) 0%, var(--bg-card) 100%)",
        border: "1px solid var(--text-main-a30)",
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
            fontSize: "var(--fs-13)",
            fontWeight: "var(--weight-bold)",
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
            fontSize: "var(--fs-36)",
            fontWeight: "var(--weight-extrabold)",
            margin: "0 0 8px 0",
            letterSpacing: "-0.5px",
          }}
        >
          {c.viewingYear} in Review
        </h2>
        <p
          style={{
            margin: 0,
            color: "var(--overlay-60)",
            fontSize: "var(--fs-14)",
          }}
        >
          Daily listening totals for {c.viewingYear}, with your all-time music rankings.
        </p>
      </div>

      <div
        style={{
          background: "var(--shade-40)",
          border: "1px solid var(--overlay-10)",
          borderRadius: "16px",
          padding: "16px 24px",
          textAlign: "center",
        }}
      >
        <div
          style={{
            fontSize: "var(--fs-12)",
            color: "var(--overlay-50)",
            fontWeight: "var(--weight-semibold)",
            textTransform: "uppercase",
            marginBottom: "4px",
          }}
        >
          {c.isPastYear ? `Total Listening Time (${c.viewingYear})` : `Tracked in ${c.viewingYear}`}
        </div>
        <div
          style={{
            fontSize: "var(--fs-32)",
            fontWeight: "var(--weight-extrabold)",
            color: "var(--accent-light)",
          }}
        >
          {c.formatSeconds(c.stats?.total_year_seconds || 0)}
        </div>
        <div
          style={{
            fontSize: "var(--fs-11)",
            color: "var(--overlay-40)",
            marginTop: "4px",
          }}
        >
          {`Daily listening totals for ${c.viewingYear}`}
        </div>
      </div>
    </div>
  );
}
