import { ChevronLeft, ChevronRight } from "lucide-react";
import { MONTHS, hoursMinutes, type ProfileStats } from "./useProfileStats";
import { dayFocus, shownDay } from "./dayFocus";

/** Year chips (current year red, earlier years "· ARCHIVED"); pull down refreshes. */
export function YearChips({ p }: { p: ProfileStats }) {
  const years = p.stats?.available_years?.length ? [...p.stats.available_years].sort((a, b) => b - a) : [p.currentYear];
  return (
    <div className="m-profile-years">
      {years.map((y) => (
        <button key={y} type="button" className={`m-profile-chip ${y === p.viewingYear ? "is-on" : ""}`} aria-pressed={y === p.viewingYear} onClick={() => p.pickYear(y)}>
          {y < p.currentYear ? `${y} · ARCHIVED` : y}
        </button>
      ))}
    </div>
  );
}

function TimeBox({ label, seconds, red }: { label: string; seconds: number; red?: boolean }) {
  return <div className="m-profile-box"><span className="m-profile-meta">{label}</span><strong className={red ? "is-red" : undefined}>{hoursMinutes(seconds)}</strong></div>;
}

/**
 * This year / This month over Today / This week. A tapped bar swaps the bottom
 * row for that day and its week; the top row follows the month being viewed.
 */
export function TimeBoxes({ p }: { p: ProfileStats }) {
  const s = p.stats;
  const f = p.selectedDay ? dayFocus(s?.monthly_graph ?? [], p.selectedDay) : null;
  const thisMonth = p.viewingMonth === new Date().getMonth() + 1 && p.viewingYear === new Date().getFullYear();
  return (
    <div className="m-profile-boxes" aria-live="polite">
      <TimeBox label="THIS YEAR" seconds={s?.total_year_seconds ?? 0} />
      <TimeBox label={thisMonth ? "THIS MONTH" : MONTHS[p.viewingMonth - 1]} seconds={s?.monthly_seconds ?? 0} red />
      <TimeBox label={f ? f.dayLabel : "TODAY"} seconds={f ? f.daySeconds : s?.daily_seconds ?? 0} />
      <TimeBox label={f ? `WEEK · ${f.weekLabel}` : "THIS WEEK"} seconds={f ? f.weekSeconds : s?.weekly_seconds ?? 0} red />
    </div>
  );
}

/** Month bars, day by day (busiest day red; today, or the tapped day, bright), with Active Days and Daily Avg. */
export function MonthGraph({ p }: { p: ProfileStats }) {
  const graph = p.stats?.monthly_graph ?? [];
  const peak = Math.max(1, ...graph.map((d) => d.total_seconds));
  const active = graph.filter((d) => d.total_seconds > 0).length;
  const avg = graph.length ? Math.round((p.stats?.monthly_seconds ?? 0) / graph.length) : 0;
  const bright = shownDay(p.selectedDay);
  return (
    <div className="m-profile-month">
      <div className="m-profile-month__head">
        <span className="m-profile-month__nav">
          <button type="button" disabled={!p.canPrevMonth} onClick={() => p.stepMonth(-1)} aria-label="Previous month"><ChevronLeft size={18} strokeWidth={2} /></button>
          <span>{MONTHS[p.viewingMonth - 1]} {p.viewingYear}</span>
          <button type="button" disabled={!p.canNextMonth} onClick={() => p.stepMonth(1)} aria-label="Next month"><ChevronRight size={18} strokeWidth={2} /></button>
        </span>
        <span className="is-muted">DAY BY DAY</span>
      </div>
      <div className="m-profile-bars" role="group" aria-label={`Listening per day in ${MONTHS[p.viewingMonth - 1]}. Tap a day to see it above.`}>
        {graph.map((d) => (
          <button key={d.date} type="button" className={`m-profile-bar-hit ${bright === d.date ? "is-selected" : ""}`}
            aria-pressed={bright === d.date} aria-label={`${MONTHS[p.viewingMonth - 1]} ${d.day}: ${hoursMinutes(d.total_seconds)}`}
            onClick={() => p.selectDay(d.date)}>
            <span className={`m-profile-bar ${d.total_seconds === peak ? "is-peak" : ""} ${d.total_seconds ? "" : "is-empty"}`}
              style={{ transform: `scaleY(${d.total_seconds ? Math.max(0.03, d.total_seconds / peak) : 1})` }} />
          </button>
        ))}
      </div>
      <div className="m-profile-month__foot"><span>ACTIVE DAYS {active}</span><span className="is-red">DAILY AVG {hoursMinutes(avg)}</span></div>
    </div>
  );
}
