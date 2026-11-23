import { useCallback, useEffect, useState } from "react";
import type { StatsOverview } from "../../../types";
import type { MobileScreenProps } from "../types";

type Ctx = MobileScreenProps["c"];

/** "1H 52M", "48M"; from 100 hours on just "386H", so a year still fits its box. */
export function hoursMinutes(totalSecs: number): string {
  const secs = Math.max(0, totalSecs || 0);
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (h >= 100) return `${h}H`;
  return h > 0 ? `${h}H ${String(m).padStart(2, "0")}M` : `${m}M`;
}

export const MONTHS = ["JANUARY", "FEBRUARY", "MARCH", "APRIL", "MAY", "JUNE", "JULY", "AUGUST", "SEPTEMBER", "OCTOBER", "NOVEMBER", "DECEMBER"];

/** The same listening overview the PC profile loads, for a chosen year and month. */
export function useProfileStats(c: Ctx) {
  const { fetchStatsOverview, statsRefreshTrigger } = c;
  const [stats, setStats] = useState<StatsOverview | null>(null);
  const [year, setYear] = useState<number | undefined>(undefined);
  const [month, setMonth] = useState<number | undefined>(undefined);
  const [selectedDay, setSelectedDay] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [failed, setFailed] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const data = await fetchStatsOverview(year, month);
      setStats(data);
      setFailed(!data);
    } catch {
      setFailed(true);
    } finally {
      setLoading(false);
    }
  }, [fetchStatsOverview, year, month]);
  useEffect(() => { void load(); }, [load, statsRefreshTrigger]);

  const currentYear = stats?.current_year ?? new Date().getFullYear();
  const viewingYear = year ?? currentYear;
  const viewingMonth = month ?? stats?.selected_month ?? new Date().getMonth() + 1;
  const months = stats?.available_months?.length ? stats.available_months : [viewingMonth];
  const stepMonth = (delta: number) => {
    const next = months[months.indexOf(viewingMonth) + delta];
    if (next) { setMonth(next); setSelectedDay(null); }
  };
  const pickYear = (next: number) => { setYear(next); setMonth(undefined); setSelectedDay(null); };
  /** Tapping a bar shows that day and its week up top; tapping it again goes back to today. */
  const selectDay = (date: string) => setSelectedDay((current) => (current === date ? null : date));

  return {
    stats, loading, failed, reload: load, currentYear, viewingYear, viewingMonth,
    isPastYear: viewingYear < currentYear,
    canPrevMonth: months.indexOf(viewingMonth) > 0,
    canNextMonth: months.indexOf(viewingMonth) < months.length - 1,
    stepMonth, pickYear, selectedDay, selectDay,
  };
}

export type ProfileStats = ReturnType<typeof useProfileStats>;
