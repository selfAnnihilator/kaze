import { MONTH_NAMES } from "./monthNames";
import { Activity, Calendar, ChevronDown } from "lucide-react";
import type { StatsViewContext } from "./context";

export function MonthlyStats({ c }: { c: StatsViewContext }) {
  return (
    <div
      style={{
        background: "linear-gradient(135deg, rgba(16, 50, 40, 0.55) 0%, rgba(24, 24, 27, 0.85) 100%)",
        border: "1px solid rgba(139, 124, 246, 0.25)",
        borderRadius: "20px",
        padding: "24px 28px",
        marginBottom: "36px",
        width: "100%",
        boxSizing: "border-box",
        position: "relative",
      }}
    >
      {/* Monthly Card Top Header: Time on Left, Month Name and Month Selector on Top Right */}
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "flex-start",
          flexWrap: "wrap",
          gap: "16px",
          marginBottom: "20px",
        }}
      >
        {/* Left: Monthly Music Time Title and Main Time Display */}
        <div>
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: "8px",
              marginBottom: "6px",
            }}
          >
            <div
              style={{
                width: "32px",
                height: "32px",
                borderRadius: "8px",
                background: "rgba(139, 124, 246, 0.15)",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                color: "#8b7cf6",
              }}
            >
              <Activity size={18} />
            </div>
            <span
              style={{
                fontSize: "13px",
                fontWeight: 700,
                color: "rgba(255, 255, 255, 0.7)",
                textTransform: "uppercase",
                letterSpacing: "0.5px",
              }}
            >
              Monthly Music Time
            </span>
          </div>

          <div style={{ display: "flex", alignItems: "baseline", gap: "12px", flexWrap: "wrap" }}>
            <span
              style={{
                fontSize: "36px",
                fontWeight: 800,
                color: "#e8d8c9",
                letterSpacing: "-0.5px",
              }}
            >
              {c.formatSeconds(c.stats?.monthly_seconds || 0)}
            </span>
            <span style={{ fontSize: "13px", color: "rgba(255, 255, 255, 0.5)" }}>
              total in {c.activeMonthName} {c.viewingYear}
            </span>
          </div>
        </div>

        {/* Right: Month Name Badge and Option to Choose the Month */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: "12px",
            flexWrap: "wrap",
          }}
        >
          {/* Active Month Name Badge */}
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: "6px",
              background: "rgba(139, 124, 246, 0.15)",
              border: "1px solid rgba(139, 124, 246, 0.35)",
              color: "#8b7cf6",
              padding: "6px 14px",
              borderRadius: "9999px",
              fontSize: "13px",
              fontWeight: 700,
            }}
          >
            <Calendar size={14} />
            <span>{c.activeMonthName}</span>
          </div>

          {/* Option to Choose Month Dropdown */}
          <div style={{ position: "relative" }}>
            <select
              value={c.activeMonthNum}
              onChange={(e) => {
                const m = parseInt(e.target.value, 10);
                c.setSelectedMonth(m);
              }}
              style={{
                appearance: "none",
                backgroundColor: "rgba(255, 255, 255, 0.08)",
                border: "1px solid rgba(255, 255, 255, 0.18)",
                borderRadius: "10px",
                color: "#e8d8c9",
                padding: "7px 34px 7px 14px",
                fontSize: "13px",
                fontWeight: 600,
                cursor: "pointer",
                outline: "none",
                transition: "border-color 0.15s ease",
              }}
            >
              {MONTH_NAMES.map((mName, idx) => {
                const mNum = idx + 1;
                return (
                  <option
                    key={mNum}
                    value={mNum}
                    style={{ backgroundColor: "var(--bg-card)", color: "#e8d8c9" }}
                  >
                    {mName}
                  </option>
                );
              })}
            </select>
            <ChevronDown
              size={14}
              style={{
                position: "absolute",
                right: "12px",
                top: "50%",
                transform: "translateY(-50%)",
                color: "rgba(255, 255, 255, 0.6)",
                pointerEvents: "none",
              }}
            />
          </div>
        </div>
      </div>

      {/* 30-Day Listening Activity Graph */}
      <div
        style={{
          background: "rgba(0, 0, 0, 0.25)",
          borderRadius: "14px",
          padding: "18px 20px 14px",
          border: "1px solid rgba(255, 255, 255, 0.06)",
        }}
      >
        {/* Summary line above graph */}
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            marginBottom: "16px",
            fontSize: "12px",
            color: "rgba(255, 255, 255, 0.6)",
            minHeight: "22px",
          }}
        >
          <div>
            {c.hoveredDay ? (
              <span style={{ color: "#e8d8c9" }}>
                📅 <strong style={{ color: "#8b7cf6" }}>{c.hoveredDay.date}</strong> (Day {c.hoveredDay.day}):{" "}
                <strong>{c.formatSeconds(c.hoveredDay.total_seconds)}</strong>
              </span>
            ) : (
              <span>
                Daily listening activity across {c.activeMonthName} ({c.monthlyGraph.length} days)
                {c.stats?.history_started_at ? (
                  <span style={{ fontSize: "11px", color: "rgba(255, 255, 255, 0.45)", marginLeft: "8px" }}>
                    • Detailed timeline history available since {new Date(c.stats.history_started_at * 1000).toLocaleDateString("en-US", { month: "short", day: "numeric", year: "numeric" })}
                  </span>
                ) : null}
              </span>
            )}
          </div>

          <div style={{ display: "flex", gap: "16px" }}>
            <span>
              Active days: <strong style={{ color: "#e8d8c9" }}>{c.daysWithMusic}</strong>/{c.monthlyGraph.length}
            </span>
            <span>
              Daily avg: <strong style={{ color: "#e8d8c9" }}>{c.formatSeconds(c.dailyAvgSecs)}</strong>
            </span>
          </div>
        </div>

        {/* Bars container */}
        <div
          style={{
            display: "flex",
            alignItems: "flex-end",
            gap: "4px",
            height: "130px",
            width: "100%",
            paddingBottom: "8px",
            borderBottom: "1px solid rgba(255, 255, 255, 0.1)",
          }}
        >
          {c.monthlyGraph.map((point) => {
            const isHovered = c.hoveredDay?.day === point.day;
            const hasTime = point.total_seconds > 0;
            const heightPct = hasTime
              ? Math.max(Math.round((point.total_seconds / c.maxGraphSeconds) * 100), 8)
              : 4;

            return (
              <div
                key={point.day}
                onMouseEnter={() => c.setHoveredDay(point)}
                onMouseLeave={() => c.setHoveredDay(null)}
                style={{
                  flex: 1,
                  height: "100%",
                  display: "flex",
                  flexDirection: "column",
                  justifyContent: "flex-end",
                  alignItems: "center",
                  cursor: "pointer",
                  position: "relative",
                }}
              >
                {/* Tooltip on hover */}
                {isHovered && (
                  <div
                    style={{
                      position: "absolute",
                      bottom: "calc(100% + 8px)",
                      background: "rgba(24, 24, 27, 0.95)",
                      border: "1px solid rgba(139, 124, 246, 0.4)",
                      color: "#e8d8c9",
                      padding: "6px 10px",
                      borderRadius: "8px",
                      fontSize: "11px",
                      fontWeight: 600,
                      whiteSpace: "nowrap",
                      zIndex: 20,
                      boxShadow: "0 8px 20px rgba(0, 0, 0, 0.5)",
                      pointerEvents: "none",
                    }}
                  >
                    <div>Day {point.day} ({point.date})</div>
                    <div style={{ color: "var(--accent-secondary)", fontWeight: 700 }}>
                      {c.formatSeconds(point.total_seconds)}
                    </div>
                  </div>
                )}

                {/* Bar element */}
                <div
                  className="listening-graph-bar"
                  data-has-time={hasTime}
                  data-hovered={isHovered}
                  style={{
                    width: "100%",
                    maxWidth: "24px",
                    height: `${heightPct}%`,
                    borderRadius: "4px 4px 0 0",
                  }}
                />
              </div>
            );
          })}
        </div>

        {/* Day markers along X-Axis */}
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            paddingTop: "6px",
            fontSize: "11px",
            color: "rgba(255, 255, 255, 0.4)",
            fontWeight: 600,
          }}
        >
          <span>Day 1</span>
          <span>Day 5</span>
          <span>Day 10</span>
          <span>Day 15</span>
          <span>Day 20</span>
          <span>Day 25</span>
          <span>Day {c.monthlyGraph.length}</span>
        </div>
      </div>
    </div>
  );
}
