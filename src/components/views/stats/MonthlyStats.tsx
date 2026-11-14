import { MONTH_NAMES } from "./monthNames";
import { Activity, Calendar, ChevronDown } from "lucide-react";
import type { StatsViewContext } from "./context";

export function MonthlyStats({ c }: { c: StatsViewContext }) {
  return (
    <div
      style={{
        background: "linear-gradient(135deg, var(--palette-teal-850-a55) 0%, var(--palette-gray-900-a85) 100%)",
        border: "1px solid var(--accent-secondary-a25)",
        borderRadius: "20px",
        padding: "var(--space-24) var(--space-28)",
        marginBottom: "var(--space-36)",
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
          gap: "var(--space-16)",
          marginBottom: "var(--space-20)",
        }}
      >
        {/* Left: Monthly Music Time Title and Main Time Display */}
        <div>
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: "var(--space-8)",
              marginBottom: "var(--space-6)",
            }}
          >
            <div
              style={{
                width: "32px",
                height: "32px",
                borderRadius: "8px",
                background: "var(--accent-secondary-a15)",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                color: "var(--accent-secondary)",
              }}
            >
              <Activity size={18} />
            </div>
            <span
              style={{
                fontSize: "var(--fs-13)",
                fontWeight: "var(--weight-bold)",
                color: "var(--overlay-70)",
                textTransform: "uppercase",
                letterSpacing: "0.5px",
              }}
            >
              Monthly Music Time
            </span>
          </div>

          <div style={{ display: "flex", alignItems: "baseline", gap: "var(--space-12)", flexWrap: "wrap" }}>
            <span
              style={{
                fontSize: "var(--fs-36)",
                fontWeight: "var(--weight-extrabold)",
                color: "var(--text-main)",
                letterSpacing: "-0.5px",
              }}
            >
              {c.formatSeconds(c.stats?.monthly_seconds || 0)}
            </span>
            <span style={{ fontSize: "var(--fs-13)", color: "var(--overlay-50)" }}>
              total in {c.activeMonthName} {c.viewingYear}
            </span>
          </div>
        </div>

        {/* Right: Month Name Badge and Option to Choose the Month */}
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: "var(--space-12)",
            flexWrap: "wrap",
          }}
        >
          {/* Active Month Name Badge */}
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: "var(--space-6)",
              background: "var(--accent-secondary-a15)",
              border: "1px solid var(--accent-secondary-a35)",
              color: "var(--accent-secondary)",
              padding: "var(--space-6) var(--space-14)",
              borderRadius: "9999px",
              fontSize: "var(--fs-13)",
              fontWeight: "var(--weight-bold)",
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
                backgroundColor: "var(--overlay-8)",
                border: "1px solid var(--overlay-20)",
                borderRadius: "10px",
                color: "var(--text-main)",
                padding: "var(--space-8) var(--space-36) var(--space-8) var(--space-14)",
                fontSize: "var(--fs-13)",
                fontWeight: "var(--weight-semibold)",
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
                    style={{ backgroundColor: "var(--bg-card)", color: "var(--text-main)" }}
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
                color: "var(--overlay-60)",
                pointerEvents: "none",
              }}
            />
          </div>
        </div>
      </div>

      {/* 30-Day Listening Activity Graph */}
      <div
        style={{
          background: "var(--shade-25)",
          borderRadius: "14px",
          padding: "var(--space-18) var(--space-20) var(--space-14)",
          border: "1px solid var(--overlay-5)",
        }}
      >
        {/* Summary line above graph */}
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            marginBottom: "var(--space-16)",
            fontSize: "var(--fs-12)",
            color: "var(--overlay-60)",
            minHeight: "22px",
          }}
        >
          <div>
            {c.hoveredDay ? (
              <span style={{ color: "var(--text-main)" }}>
                📅 <strong style={{ color: "var(--accent-secondary)" }}>{c.hoveredDay.date}</strong> (Day {c.hoveredDay.day}):{" "}
                <strong>{c.formatSeconds(c.hoveredDay.total_seconds)}</strong>
              </span>
            ) : (
              <span>
                Daily listening activity across {c.activeMonthName} ({c.monthlyGraph.length} days)
                {c.stats?.history_started_at ? (
                  <span style={{ fontSize: "var(--fs-11)", color: "var(--overlay-45)", marginLeft: "var(--space-8)" }}>
                    • Detailed timeline history available since {new Date(c.stats.history_started_at * 1000).toLocaleDateString("en-US", { month: "short", day: "numeric", year: "numeric" })}
                  </span>
                ) : null}
              </span>
            )}
          </div>

          <div style={{ display: "flex", gap: "var(--space-16)" }}>
            <span>
              Active days: <strong style={{ color: "var(--text-main)" }}>{c.daysWithMusic}</strong>/{c.monthlyGraph.length}
            </span>
            <span>
              Daily avg: <strong style={{ color: "var(--text-main)" }}>{c.formatSeconds(c.dailyAvgSecs)}</strong>
            </span>
          </div>
        </div>

        {/* Bars container */}
        <div
          style={{
            display: "flex",
            alignItems: "flex-end",
            gap: "var(--space-4)",
            height: "130px",
            width: "100%",
            paddingBottom: "var(--space-8)",
            borderBottom: "1px solid var(--overlay-10)",
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
                      background: "var(--palette-gray-900-a95)",
                      border: "1px solid var(--accent-secondary-a40)",
                      color: "var(--text-main)",
                      padding: "var(--space-6) var(--space-10)",
                      borderRadius: "8px",
                      fontSize: "var(--fs-11)",
                      fontWeight: "var(--weight-semibold)",
                      whiteSpace: "nowrap",
                      zIndex: 20,
                      boxShadow: "0 8px 20px var(--shade-50)",
                      pointerEvents: "none",
                    }}
                  >
                    <div>Day {point.day} ({point.date})</div>
                    <div style={{ color: "var(--accent-secondary)", fontWeight: "var(--weight-bold)" }}>
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
            paddingTop: "var(--space-6)",
            fontSize: "var(--fs-11)",
            color: "var(--overlay-40)",
            fontWeight: "var(--weight-semibold)",
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
