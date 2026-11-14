import React from "react";

export interface RankedRow {
  key: string;
  name: string;
  total_seconds: number;
  play_count: number;
}

/** Compact ranked list card (top artists / top genres). Capped at 5 rows. */
export const RankedListCard: React.FC<{
  title: string;
  subtitle?: string;
  icon: React.ReactNode;
  iconBg: string;
  iconColor: string;
  rows: RankedRow[];
  emptyText: string;
  formatSeconds: (secs: number) => string;
}> = React.memo(({ title, subtitle, icon, iconBg, iconColor, rows, emptyText, formatSeconds }) => (
  <div
    style={{
      flex: 1,
      background: "var(--bg-card)",
      border: "1px solid var(--overlay-8)",
      borderRadius: "16px",
      padding: "var(--space-24)",
      display: "flex",
      flexDirection: "column",
      minHeight: 0,
    }}
  >
    <div style={{ display: "flex", alignItems: "center", gap: "var(--space-10)", marginBottom: "var(--space-16)" }}>
      <div
        style={{
          width: "32px",
          height: "32px",
          borderRadius: "8px",
          background: iconBg,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          color: iconColor,
        }}
      >
        {icon}
      </div>
      <div>
        <h2 style={{ fontSize: "var(--fs-18)", fontWeight: "var(--weight-bold)", margin: 0 }}>{title}</h2>
        {subtitle && (
          <p style={{ margin: 0, fontSize: "var(--fs-12)", color: "var(--overlay-40)" }}>{subtitle}</p>
        )}
      </div>
    </div>
    {rows.length === 0 ? (
      <div
        style={{
          padding: "var(--space-24) 0",
          textAlign: "center",
          color: "var(--overlay-40)",
          fontSize: "var(--fs-13)",
        }}
      >
        {emptyText}
      </div>
    ) : (
      <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-6)" }}>
        {rows.slice(0, 5).map((row, idx) => {
          const rank = idx + 1;
          const rankColor =
            rank === 1 ? "var(--palette-amber-450)" : rank === 2 ? "var(--palette-slate-100)" : rank === 3 ? "var(--palette-orange-550)" : "var(--overlay-50)";
          return (
            <div
              key={row.key}
              style={{
                display: "flex",
                alignItems: "center",
                padding: "var(--space-8) var(--space-14)",
                borderRadius: "10px",
                background: rank <= 3 ? "var(--overlay-3)" : "transparent",
                border: rank <= 3 ? "1px solid var(--overlay-5)" : "none",
              }}
            >
              <span
                style={{
                  width: "28px",
                  fontSize: "var(--fs-15)",
                  fontWeight: "var(--weight-extrabold)",
                  color: rankColor,
                  textAlign: "center",
                  flexShrink: 0,
                }}
              >
                {rank}
              </span>
              <div style={{ flex: 1, minWidth: 0, margin: "0 var(--space-14)" }}>
                <div
                  style={{
                    fontSize: "var(--fs-14)",
                    fontWeight: "var(--weight-semibold)",
                    color: "var(--text-main)",
                    overflow: "hidden",
                    textOverflow: "ellipsis",
                    whiteSpace: "nowrap",
                  }}
                >
                  {row.name}
                </div>
              </div>
              <div style={{ textAlign: "right" }}>
                <div style={{ fontSize: "var(--fs-13)", fontWeight: "var(--weight-semibold)", color: "var(--text-main)" }}>
                  {formatSeconds(row.total_seconds)}
                </div>
                <div style={{ fontSize: "var(--fs-11)", color: "var(--overlay-40)" }}>
                  {row.play_count} {row.play_count === 1 ? "play" : "plays"}
                </div>
              </div>
            </div>
          );
        })}
      </div>
    )}
  </div>
));
