import { useCallback, useEffect, useState } from "react";
import { executeQuery } from "../../../services/api";
import type { CollectionData } from "../../../components/views/collection/types";

/** `group` is the filter a chart falls under: a language ("ENGLISH") or "TRENDING" for global charts. */
export interface ChartCard { id: string; title: string; image?: string; badge: string; group: string }

/** Language or country words in chart names → [filter name, card badge]. */
const REGION: Record<string, [string, string]> = {
  english: ["ENGLISH", "EN"], hindi: ["HINDI", "HI"], punjabi: ["PUNJABI", "PA"], tamil: ["TAMIL", "TA"],
  telugu: ["TELUGU", "TE"], marathi: ["MARATHI", "MR"], bengali: ["BENGALI", "BN"], kannada: ["KANNADA", "KN"],
  malayalam: ["MALAYALAM", "ML"], gujarati: ["GUJARATI", "GU"], bhojpuri: ["BHOJPURI", "BH"], haryanvi: ["HARYANVI", "HR"],
  korean: ["KOREAN", "KR"], spanish: ["SPANISH", "ES"], bollywood: ["HINDI", "HI"], india: ["INDIA", "IN"],
};
const TRENDING = /\b(global|world|international|trending|chartbusters)\b/i;

function groupOf(title: string): [string, string] {
  if (TRENDING.test(title)) return ["TRENDING", "WORLD"];
  const words = title.toLowerCase().split(/[^a-z]+/);
  return words.map((w) => REGION[w]).find(Boolean) ?? ["MORE", "TOP"];
}

/** The filters these charts fall under, most charts first; ALL leads. */
export function chartFilters(charts: ChartCard[]): string[] {
  const counts = new Map<string, number>();
  for (const c of charts) counts.set(c.group, (counts.get(c.group) ?? 0) + 1);
  const groups = [...counts.entries()].sort((x, y) => y[1] - x[1]).map(([g]) => g);
  return groups.length > 1 ? ["ALL", ...groups] : ["ALL"];
}

/** Top charts from the same source the PC uses (GetCharts / RefreshCharts). */
export function useCharts() {
  const [charts, setCharts] = useState<ChartCard[] | null>(null);
  const [failed, setFailed] = useState(false);
  const load = useCallback(async (force = false) => {
    try {
      const result = await executeQuery({ query: force ? "RefreshCharts" : "GetCharts" });
      const rows: Array<{ id: string; title: string; image?: string }> = Array.isArray(result.data) ? result.data : [];
      setCharts(rows.map((c) => {
        const [group, badge] = groupOf(c.title);
        return { id: `jiosaavn:playlist:${c.id}`, title: c.title, image: c.image, badge, group };
      }));
      setFailed(false);
    } catch {
      setFailed(true);
    }
  }, []);
  useEffect(() => { void load(); }, [load]);
  return { charts, failed, refresh: () => load(true) };
}

/** A chart as the collection the Collection pane opens. */
export const chartCollection = (chart: ChartCard): CollectionData => ({
  id: chart.id, type: "chart", title: chart.title, subtitle: "Top songs", tag: `TOP CHART • ${chart.badge}`,
  bgGradient: "var(--m-paper)", accentColor: "var(--m-red)", cover_art_url: chart.image,
});
