import { Award } from "lucide-react";
import type { StatsViewContext } from "./context";

export function TopDays({ c }: { c: StatsViewContext }) {
  return (
    <div
      style={{
        background: "var(--bg-card)",
        border: "1px solid var(--overlay-8)",
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
            background: "var(--accent-a15)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            color: "var(--palette-amber-450)",
          }}
        >
          <Award size={18} />
        </div>
        <div>
          <h2 style={{ fontSize: "var(--fs-18)", fontWeight: "var(--weight-bold)", margin: 0 }}>
            Top 5 Days Listened to Most
          </h2>
          <p
            style={{
              margin: 0,
              fontSize: "var(--fs-12)",
              color: "var(--overlay-40)",
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
            color: "var(--overlay-40)",
            fontSize: "var(--fs-13)",
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
                ? "linear-gradient(135deg, var(--palette-amber-450), var(--palette-orange-550))"
                : rank === 2
                ? "linear-gradient(135deg, var(--palette-slate-350), var(--palette-slate-550))"
                : rank === 3
                ? "linear-gradient(135deg, var(--palette-orange-550), var(--palette-orange-650))"
                : "var(--overlay-10)";
            return (
              <div
                key={day.day_date}
                style={{
                  background: "var(--overlay-3)",
                  border: "1px solid var(--overlay-5)",
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
                    color: "var(--text-main)",
                    fontSize: "var(--fs-12)",
                    fontWeight: "var(--weight-extrabold)",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                  }}
                >
                  #{rank}
                </div>
                <div
                  style={{
                    fontSize: "var(--fs-11)",
                    fontWeight: "var(--weight-bold)",
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
                    fontSize: "var(--fs-14)",
                    fontWeight: "var(--weight-semibold)",
                    color: "var(--text-main)",
                    marginBottom: "8px",
                  }}
                >
                  {day.day_date}
                </div>
                <div
                  style={{
                    fontSize: "var(--fs-20)",
                    fontWeight: "var(--weight-extrabold)",
                    color: "var(--accent-secondary)",
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
