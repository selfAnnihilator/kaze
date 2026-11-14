import { Archive } from "lucide-react";
import type { StatsViewContext } from "./context";

export function ArchivedYearHero({ c }: { c: StatsViewContext }) {
  return (
    <div
      style={{
        background: "linear-gradient(135deg, var(--palette-taupe-850) 0%, var(--bg-card) 100%)",
        border: "1px solid var(--text-main-a30)",
        borderRadius: "var(--radius-20)",
        padding: "var(--space-32)",
        marginBottom: "var(--space-36)",
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        flexWrap: "wrap",
        gap: "var(--space-20)",
      }}
    >
      <div>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: "var(--space-8)",
            color: "var(--text-main)",
            fontSize: "var(--fs-13)",
            fontWeight: "var(--weight-bold)",
            textTransform: "uppercase",
            letterSpacing: "0.5px",
            marginBottom: "var(--space-6)",
          }}
        >
          <Archive size={16} />
          <span>Year in Review</span>
        </div>
        <h2
          style={{
            fontSize: "var(--fs-36)",
            fontWeight: "var(--weight-extrabold)",
            margin: "0 0 var(--space-8) 0",
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
          borderRadius: "var(--radius-16)",
          padding: "var(--space-16) var(--space-24)",
          textAlign: "center",
        }}
      >
        <div
          style={{
            fontSize: "var(--fs-12)",
            color: "var(--overlay-50)",
            fontWeight: "var(--weight-semibold)",
            textTransform: "uppercase",
            marginBottom: "var(--space-4)",
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
            marginTop: "var(--space-4)",
          }}
        >
          {`Daily listening totals for ${c.viewingYear}`}
        </div>
      </div>
    </div>
  );
}
