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
          background: "linear-gradient(135deg, rgba(30, 27, 75, 0.6) 0%, rgba(24, 24, 27, 0.8) 100%)",
          border: "1px solid rgba(243, 112, 30, 0.2)",
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
              fontSize: "13px",
              fontWeight: 600,
              color: "rgba(255, 255, 255, 0.6)",
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
              background: "rgba(236, 72, 153, 0.15)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              color: "#f472b6",
            }}
          >
            <Flame size={20} />
          </div>
        </div>
        <div style={{ fontSize: "32px", fontWeight: 800, color: "#e8d8c9" }}>
          {c.formatSeconds(c.stats?.daily_seconds || 0)}
        </div>
        <div
          style={{
            fontSize: "12px",
            color: "rgba(255, 255, 255, 0.4)",
            marginTop: "4px",
          }}
        >
          Today's listening session
        </div>
      </div>

      {/* Weekly Music Time Card */}
      <div
        style={{
          background: "linear-gradient(135deg, rgba(20, 35, 60, 0.6) 0%, rgba(24, 24, 27, 0.8) 100%)",
          border: "1px solid rgba(59, 130, 246, 0.2)",
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
              fontSize: "13px",
              fontWeight: 600,
              color: "rgba(255, 255, 255, 0.6)",
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
              background: "rgba(59, 130, 246, 0.15)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              color: "#60a5fa",
            }}
          >
            <Calendar size={20} />
          </div>
        </div>
        <div style={{ fontSize: "32px", fontWeight: 800, color: "#e8d8c9" }}>
          {c.formatSeconds(c.stats?.weekly_seconds || 0)}
        </div>
        <div
          style={{
            fontSize: "12px",
            color: "rgba(255, 255, 255, 0.4)",
            marginTop: "4px",
          }}
        >
          Past 7 days
        </div>
      </div>

      {/* Lifetime Listening Card */}
      <div
        style={{
          background: "linear-gradient(135deg, rgba(40, 25, 60, 0.6) 0%, rgba(24, 24, 27, 0.8) 100%)",
          border: "1px solid rgba(168, 85, 247, 0.25)",
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
              fontSize: "13px",
              fontWeight: 600,
              color: "rgba(255, 255, 255, 0.6)",
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
              background: "rgba(168, 85, 247, 0.15)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              color: "#c084fc",
            }}
          >
            <Award size={20} />
          </div>
        </div>
        <div style={{ fontSize: "32px", fontWeight: 800, color: "#e8d8c9" }}>
          {c.formatSeconds(c.stats?.lifetime_seconds || 0)}
        </div>
        <div
          style={{
            fontSize: "12px",
            color: "rgba(255, 255, 255, 0.4)",
            marginTop: "4px",
          }}
        >
          All-time cumulative history
        </div>
      </div>
    </div>
  );
}
