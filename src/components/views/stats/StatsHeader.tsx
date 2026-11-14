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
        gap: "20px",
        marginBottom: "32px",
      }}
    >
      <div>
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: "10px",
            marginBottom: "8px",
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
              padding: "2px 10px",
              borderRadius: "9999px",
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
      <div style={{ display: "flex", alignItems: "center", gap: "10px", flexWrap: "wrap" }}>
        {c.stats && c.stats.available_years && c.stats.available_years.length > 0 && (
          <div
            style={{
              display: "flex",
              gap: "6px",
              background: "var(--overlay-5)",
              padding: "4px",
              borderRadius: "12px",
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
                    padding: "6px 14px",
                    borderRadius: "8px",
                    border: "none",
                    background: isActive
                      ? "linear-gradient(135deg, var(--accent), var(--palette-taupe-550))"
                      : "transparent",
                    color: isActive ? "var(--text-main)" : "var(--overlay-60)",
                    fontWeight: isActive ? 700 : 500,
                    fontSize: "var(--fs-13)",
                    cursor: "pointer",
                    transition: "transform 0.15s ease, opacity 0.15s ease",
                    display: "flex",
                    alignItems: "center",
                    gap: "6px",
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
            padding: "7px 14px",
            borderRadius: "10px",
            border: "1px solid var(--overlay-12)",
            background: "var(--overlay-5)",
            color: "var(--text-main)",
            fontSize: "var(--fs-12)",
            fontWeight: "var(--weight-semibold)",
            cursor: "pointer",
            display: "flex",
            alignItems: "center",
            gap: "7px",
            transition: "transform 0.15s ease, opacity 0.15s ease",
          }}
          title="Refresh listening stats"
        >
          <RotateCw size={14} className={c.isRefreshing ? "spin-animation" : ""} />
        </button>
      </div>
    </div>
  );
}
