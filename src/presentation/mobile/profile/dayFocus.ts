/** One bar of the month graph (same shape as the backend's DailyListeningPoint). */
export interface DayPoint { day: number; date: string; total_seconds: number }

export interface DayFocus { dayLabel: string; daySeconds: number; weekLabel: string; weekSeconds: number }

const MON = ["JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"];

/**
 * The tapped day and its Monday–Sunday week, for the boxes above the graph.
 * The graph only holds this month, so a week that crosses into another month
 * is cut to this month's days, and its label says exactly which days it adds up.
 */
export function dayFocus(graph: DayPoint[], date: string): DayFocus | null {
  const point = graph.find((d) => d.date === date);
  if (!point) return null;
  const [y, m, d] = date.split("-").map(Number);
  const offset = (new Date(y, m - 1, d).getDay() + 6) % 7;
  const first = Math.max(1, d - offset);
  const last = Math.min(Math.max(...graph.map((g) => g.day)), d - offset + 6);
  const weekSeconds = graph.filter((g) => g.day >= first && g.day <= last).reduce((sum, g) => sum + g.total_seconds, 0);
  const mon = MON[m - 1];
  return {
    dayLabel: `${mon} ${d}`,
    daySeconds: point.total_seconds,
    weekLabel: first === last ? `${mon} ${first}` : `${mon} ${first}–${last}`,
    weekSeconds,
  };
}
