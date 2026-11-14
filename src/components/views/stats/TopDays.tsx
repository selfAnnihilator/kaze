import { Award } from "lucide-react";
import type { StatsViewContext } from "./context";

export function TopDays({ c }: { c: StatsViewContext }) {
  return (
    <div
      style={{
        background: "var(--bg-card)",
        border: "1px solid rgba(255, 255, 255, 0.08)",
        borderRadius: "16px",
        padding: "24px",
      }}
    >
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: "10px",
          marginBottom: "20px",
        }}
      >
        <div
          style={{
            width: "32px",
            height: "32px",
            borderRadius: "8px",
            background: "rgba(243, 112, 30, 0.15)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            color: "#fbbf24",
          }}
        >
          <Award size={18} />
        </div>
        <div>
          <h2 style={{ fontSize: "18px", fontWeight: 700, margin: 0 }}>
            Top 5 Days Listened to Most
          </h2>
          <p
            style={{
              margin: 0,
              fontSize: "12px",
              color: "rgba(255, 255, 255, 0.4)",
            }}
          >
            Ranked by peak listening time in a single day
          </p>
        </div>
      </div>

      {!c.stats?.top_days || c.stats.top_days.length === 0 ? (
        <div
          style={{
            padding: "36px 0",
            textAlign: "center",
            color: "rgba(255, 255, 255, 0.4)",
            fontSize: "13px",
          }}
        >
          No daily listening peaks logged yet. Keep playing your favorite tunes!
        </div>
      ) : (
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 180px), 1fr))",
            gap: "14px",
          }}
        >
          {c.stats.top_days.map((day, idx) => {
            const rank = idx + 1;
            const rankBadgeBg =
              rank === 1
                ? "linear-gradient(135deg, #fbbf24, #d97706)"
                : rank === 2
                ? "linear-gradient(135deg, #94a3b8, #64748b)"
                : rank === 3
                ? "linear-gradient(135deg, #d97706, #b45309)"
                : "rgba(255, 255, 255, 0.1)";
            return (
              <div
                key={day.day_date}
                style={{
                  background: "rgba(255, 255, 255, 0.03)",
                  border: "1px solid rgba(255, 255, 255, 0.06)",
                  borderRadius: "12px",
                  padding: "16px",
                  position: "relative",
                  overflow: "hidden",
                }}
              >
                <div
                  style={{
                    position: "absolute",
                    top: "12px",
                    right: "12px",
                    width: "24px",
                    height: "24px",
                    borderRadius: "50%",
                    background: rankBadgeBg,
                    color: "#e8d8c9",
                    fontSize: "12px",
                    fontWeight: 800,
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                  }}
                >
                  #{rank}
                </div>
                <div
                  style={{
                    fontSize: "11px",
                    fontWeight: 700,
                    color: "var(--accent-light)",
                    textTransform: "uppercase",
                    letterSpacing: "0.5px",
                    marginBottom: "4px",
                  }}
                >
                  {day.day_name}
                </div>
                <div
                  style={{
                    fontSize: "14px",
                    fontWeight: 600,
                    color: "#e8d8c9",
                    marginBottom: "8px",
                  }}
                >
                  {day.day_date}
                </div>
                <div
                  style={{
                    fontSize: "20px",
                    fontWeight: 800,
                    color: "#8b7cf6",
                  }}
                >
                  {c.formatSeconds(day.total_seconds)}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
