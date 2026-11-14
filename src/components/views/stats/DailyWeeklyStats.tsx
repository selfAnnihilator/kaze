import { Award, Calendar, Flame } from "lucide-react";
import type { StatsViewContext } from "./context";

export function DailyWeeklyStats({ c }: { c: StatsViewContext }) {
  return (
    <div
      style={{
        display: "grid",
        gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 280px), 1fr))",
        gap: "20px",
        marginBottom: "20px",
      }}
    >
      {/* Daily Music Time Card */}
      <div
        style={{
          background: "linear-gradient(135deg, var(--palette-indigo-800-a60) 0%, var(--palette-gray-900-a80) 100%)",
          border: "1px solid var(--accent-a20)",
          borderRadius: "18px",
          padding: "20px 24px",
          position: "relative",
          overflow: "hidden",
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            marginBottom: "12px",
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
              borderRadius: "10px",
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
            marginTop: "4px",
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
          borderRadius: "18px",
          padding: "20px 24px",
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            marginBottom: "12px",
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
              borderRadius: "10px",
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
            marginTop: "4px",
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
          borderRadius: "18px",
          padding: "20px 24px",
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            marginBottom: "12px",
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
              borderRadius: "10px",
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
            marginTop: "4px",
          }}
        >
          All-time cumulative history
        </div>
      </div>
    </div>
  );
}
