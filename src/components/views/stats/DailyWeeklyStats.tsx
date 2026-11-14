import { Award, Calendar, Flame } from "lucide-react";
import type { StatsViewContext } from "./context";

export function DailyWeeklyStats({ c }: { c: StatsViewContext }) {
  return (
    <div
      style={{
        display: "grid",
        gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 280px), 1fr))",
        gap: "var(--space-20)",
        marginBottom: "var(--space-20)",
      }}
    >
      {/* Daily Music Time Card */}
      <div
        style={{
          background: "linear-gradient(135deg, var(--palette-indigo-800-a60) 0%, var(--palette-gray-900-a80) 100%)",
          border: "1px solid var(--accent-a20)",
          borderRadius: "var(--radius-20)",
          padding: "var(--space-20) var(--space-24)",
          position: "relative",
          overflow: "hidden",
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            marginBottom: "var(--space-12)",
          }}
        >
          <span
            style={{
              fontSize: "var(--fs-13)",
              fontWeight: "var(--weight-semibold)",
              color: "var(--overlay-60)",
              textTransform: "uppercase",
              letterSpacing: "0.5px",
            }}
          >
            Daily Music Time
          </span>
          <div
            style={{
              width: "36px",
              height: "36px",
              borderRadius: "var(--radius-10)",
              background: "var(--palette-pink-400-a15)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              color: "var(--palette-pink-300)",
            }}
          >
            <Flame size={20} />
          </div>
        </div>
        <div style={{ fontSize: "var(--fs-32)", fontWeight: "var(--weight-extrabold)", color: "var(--text-main)" }}>
          {c.formatSeconds(c.stats?.daily_seconds || 0)}
        </div>
        <div
          style={{
            fontSize: "var(--fs-12)",
            color: "var(--overlay-40)",
            marginTop: "var(--space-4)",
          }}
        >
          Today's listening session
        </div>
      </div>

      {/* Weekly Music Time Card */}
      <div
        style={{
          background: "linear-gradient(135deg, var(--palette-blue-850-a60) 0%, var(--palette-gray-900-a80) 100%)",
          border: "1px solid var(--palette-blue-400-a20)",
          borderRadius: "var(--radius-20)",
          padding: "var(--space-20) var(--space-24)",
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            marginBottom: "var(--space-12)",
          }}
        >
          <span
            style={{
              fontSize: "var(--fs-13)",
              fontWeight: "var(--weight-semibold)",
              color: "var(--overlay-60)",
              textTransform: "uppercase",
              letterSpacing: "0.5px",
            }}
          >
            Weekly Music Time
          </span>
          <div
            style={{
              width: "36px",
              height: "36px",
              borderRadius: "var(--radius-10)",
              background: "var(--palette-blue-400-a15)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              color: "var(--palette-blue-300)",
            }}
          >
            <Calendar size={20} />
          </div>
        </div>
        <div style={{ fontSize: "var(--fs-32)", fontWeight: "var(--weight-extrabold)", color: "var(--text-main)" }}>
          {c.formatSeconds(c.stats?.weekly_seconds || 0)}
        </div>
        <div
          style={{
            fontSize: "var(--fs-12)",
            color: "var(--overlay-40)",
            marginTop: "var(--space-4)",
          }}
        >
          Past 7 days
        </div>
      </div>

      {/* Lifetime Listening Card */}
      <div
        style={{
          background: "linear-gradient(135deg, var(--palette-violet-850-a60) 0%, var(--palette-gray-900-a80) 100%)",
          border: "1px solid var(--palette-violet-350-a25)",
          borderRadius: "var(--radius-20)",
          padding: "var(--space-20) var(--space-24)",
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            marginBottom: "var(--space-12)",
          }}
        >
          <span
            style={{
              fontSize: "var(--fs-13)",
              fontWeight: "var(--weight-semibold)",
              color: "var(--overlay-60)",
              textTransform: "uppercase",
              letterSpacing: "0.5px",
            }}
          >
            Lifetime Listening
          </span>
          <div
            style={{
              width: "36px",
              height: "36px",
              borderRadius: "var(--radius-10)",
              background: "var(--palette-violet-350-a15)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              color: "var(--palette-violet-250)",
            }}
          >
            <Award size={20} />
          </div>
        </div>
        <div style={{ fontSize: "var(--fs-32)", fontWeight: "var(--weight-extrabold)", color: "var(--text-main)" }}>
          {c.formatSeconds(c.stats?.lifetime_seconds || 0)}
        </div>
        <div
          style={{
            fontSize: "var(--fs-12)",
            color: "var(--overlay-40)",
            marginTop: "var(--space-4)",
          }}
        >
          All-time cumulative history
        </div>
      </div>
    </div>
  );
}
