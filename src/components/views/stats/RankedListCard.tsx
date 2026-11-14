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
      padding: "24px",
      display: "flex",
      flexDirection: "column",
      minHeight: 0,
    }}
  >
    <div style={{ display: "flex", alignItems: "center", gap: "10px", marginBottom: "16px" }}>
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
        <h2 style={{ fontSize: "18px", fontWeight: 700, margin: 0 }}>{title}</h2>
        {subtitle && (
          <p style={{ margin: 0, fontSize: "12px", color: "var(--overlay-40)" }}>{subtitle}</p>
        )}
      </div>
    </div>
    {rows.length === 0 ? (
      <div
        style={{
          padding: "24px 0",
          textAlign: "center",
          color: "var(--overlay-40)",
          fontSize: "13px",
        }}
      >
        {emptyText}
      </div>
    ) : (
      <div style={{ display: "flex", flexDirection: "column", gap: "6px" }}>
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
                padding: "8px 14px",
                borderRadius: "10px",
                background: rank <= 3 ? "var(--overlay-3)" : "transparent",
                border: rank <= 3 ? "1px solid var(--overlay-5)" : "none",
              }}
            >
              <span
                style={{
                  width: "28px",
                  fontSize: "15px",
                  fontWeight: 800,
                  color: rankColor,
                  textAlign: "center",
                  flexShrink: 0,
                }}
              >
                {rank}
              </span>
              <div style={{ flex: 1, minWidth: 0, margin: "0 14px" }}>
                <div
                  style={{
                    fontSize: "14px",
                    fontWeight: 600,
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
                <div style={{ fontSize: "13px", fontWeight: 600, color: "var(--text-main)" }}>
                  {formatSeconds(row.total_seconds)}
                </div>
                <div style={{ fontSize: "11px", color: "var(--overlay-40)" }}>
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
