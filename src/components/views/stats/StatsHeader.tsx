import { Archive, RotateCw } from "lucide-react";
import type { StatsViewContext } from "./context";

export function StatsHeader({ c }: { c: StatsViewContext }) {
  return (
    <div
      style={{
        display: "flex",
        justifyContent: "space-between",
        alignItems: "flex-start",
        flexWrap: "wrap",
        gap: "var(--space-20)",
        marginBottom: "var(--space-32)",
      }}
    >
      <div>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: "var(--space-10)",
            marginBottom: "var(--space-8)",
          }}
        >
          <h1
            style={{
              fontSize: "var(--fs-28)",
              fontWeight: "var(--weight-extrabold)",
              margin: 0,
              letterSpacing: "-0.5px",
            }}
          >
            Listening Stats
          </h1>
          <span
            style={{
              background: "linear-gradient(135deg, var(--accent-a20), var(--accent-a20))",
              border: "1px solid var(--accent-a40)",
              color: "var(--accent-light)",
              padding: "var(--space-2) var(--space-10)",
              borderRadius: "var(--radius-pill)",
              fontSize: "var(--fs-12)",
              fontWeight: "var(--weight-bold)",
            }}
          >
            {c.viewingYear}
          </span>
        </div>

        <div
          style={{
            fontSize: "var(--fs-13)",
            color: "var(--overlay-60)",
          }}
        >
          Overview of your listening activity, trends, and top records
        </div>
      </div>

      {/* Header Right: Year Selector Tabs & Manual Refresh */}
      <div style={{ display: "flex", alignItems: "center", gap: "var(--space-10)", flexWrap: "wrap" }}>
        {c.stats && c.stats.available_years && c.stats.available_years.length > 0 && (
          <div
            style={{
              display: "flex",
              gap: "var(--space-6)",
              background: "var(--overlay-5)",
              padding: "var(--space-4)",
              borderRadius: "var(--radius-12)",
              border: "1px solid var(--overlay-8)",
            }}
          >
            {c.stats.available_years.map((year) => {
              const isActive = (c.selectedYear ?? c.currentYear) === year;
              return (
                <button
                  key={year}
                  onClick={() => c.setSelectedYear(year === c.currentYear ? undefined : year)}
                  style={{
                    padding: "var(--space-6) var(--space-14)",
                    borderRadius: "var(--radius-8)",
                    border: "none",
                    background: isActive
                      ? "linear-gradient(135deg, var(--accent), var(--palette-taupe-550))"
                      : "transparent",
                    color: isActive ? "var(--text-main)" : "var(--overlay-60)",
                    fontWeight: isActive ? 700 : 500,
                    fontSize: "var(--fs-13)",
                    cursor: "pointer",
                    transition: "transform var(--duration-fast) ease, opacity var(--duration-fast) ease",
                    display: "flex",
                    alignItems: "center",
                    gap: "var(--space-6)",
                  }}
                >
                  {year < c.currentYear && <Archive size={14} />}
                  <span>{year === c.currentYear ? `${year} (Current)` : year}</span>
                </button>
              );
            })}
          </div>
        )}

        <button
          onClick={() => c.loadStats(true)}
          disabled={c.isLoading || c.isRefreshing}
          style={{
            padding: "var(--space-8) var(--space-14)",
            borderRadius: "var(--radius-10)",
            border: "1px solid var(--overlay-12)",
            background: "var(--overlay-5)",
            color: "var(--text-main)",
            fontSize: "var(--fs-12)",
            fontWeight: "var(--weight-semibold)",
            cursor: "pointer",
            display: "flex",
            alignItems: "center",
            gap: "var(--space-8)",
            transition: "transform var(--duration-fast) ease, opacity var(--duration-fast) ease",
          }}
          title="Refresh listening stats"
        >
          <RotateCw size={14} className={c.isRefreshing ? "spin-animation" : ""} />
        </button>
      </div>
    </div>
  );
}
