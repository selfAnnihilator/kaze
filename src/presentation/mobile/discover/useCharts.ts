import { useEffect, useRef, useState } from "react";
import { executeQuery } from "../../../services/api";
import type { CollectionData } from "../../../components/views/collection/types";
import { TASTE_LANGUAGES } from "../settings/tasteLanguages";

/** `group` is the filter a chart falls under: a language ("ENGLISH") or "TRENDING" for global charts. */
export interface ChartCard { id: string; title: string; image?: string; badge: string; group: string }

/** Language or country words in chart names → [filter name, card badge]. */
const REGION: Record<string, [string, string]> = {
  english: ["ENGLISH", "EN"], hindi: ["HINDI", "HI"], punjabi: ["PUNJABI", "PA"], tamil: ["TAMIL", "TA"],
  telugu: ["TELUGU", "TE"], marathi: ["MARATHI", "MR"], bengali: ["BENGALI", "BN"], kannada: ["KANNADA", "KN"],
  malayalam: ["MALAYALAM", "ML"], gujarati: ["GUJARATI", "GU"], bhojpuri: ["BHOJPURI", "BH"], haryanvi: ["HARYANVI", "HR"],
  korean: ["KOREAN", "KR"], spanish: ["SPANISH", "ES"], bollywood: ["HINDI", "HI"], india: ["INDIA", "IN"],
  odia: ["ODIA", "OR"], assamese: ["ASSAMESE", "AS"], urdu: ["URDU", "UR"],
};
const TRENDING = /\b(global|world|international|trending|chartbusters)\b/i;

function groupOf(title: string, language?: string): [string, string] {
  if (language && REGION[language]) return REGION[language];
  const words = title.toLowerCase().split(/[^a-z]+/);
  return words.map((w) => REGION[w]).find(Boolean) ?? (TRENDING.test(title) ? ["TRENDING", "WORLD"] : ["MORE", "TOP"]);
}

/** The filters these charts fall under, most charts first; ALL leads. */
export function chartFilters(charts: ChartCard[]): string[] {
  const counts = new Map<string, number>();
  for (const c of charts) counts.set(c.group, (counts.get(c.group) ?? 0) + 1);
  const groups = [...counts.entries()].sort((x, y) => y[1] - x[1]).map(([g]) => g);
  return [...new Set(["ALL", ...TASTE_LANGUAGES.map((name) => name.toUpperCase()), ...groups])];
}

/** Top charts from the same source the PC uses (GetCharts / RefreshCharts). */
export function useCharts(filter: string, revision: number) {
  const language = filter.toLowerCase();
  const regional = TASTE_LANGUAGES.some((name) => name.toLowerCase() === language);
  const feed = regional ? language : "all";
  const [feeds, setFeeds] = useState<Record<string, ChartCard[] | false>>({});
  const lastRefresh = useRef(0);
  useEffect(() => {
    let active = true;
    const force = lastRefresh.current !== revision;
    lastRefresh.current = revision;
    const save = (cards: ChartCard[] | false) => {
      if (active) setFeeds((current) => ({ ...current, [feed]: cards }));
    };
    executeQuery(regional
      ? { query: "GetLanguageCharts", payload: { language: feed, force_refresh: force } }
      : { query: force ? "RefreshCharts" : "GetCharts" })
      .then((result) => save(chartCards(result.data)))
      .catch(() => save(false));
    return () => { active = false; };
  }, [feed, regional, revision]);
  const cards = feeds[feed];
  return { charts: cards || null, allCharts: feeds.all || [], failed: cards === false, regional };
}

function chartCards(data: unknown): ChartCard[] {
  const rows: Array<{ id: string; title: string; image?: string; language?: string }> = Array.isArray(data) ? data : [];
  return rows.map((c) => {
    const [group, badge] = groupOf(c.title, c.language);
    return { id: `jiosaavn:playlist:${c.id}`, title: c.title, image: c.image, badge, group };
  });
}

/** A chart as the collection the Collection pane opens. */
export const chartCollection = (chart: ChartCard): CollectionData => ({
  id: chart.id, type: "chart", title: chart.title, subtitle: "Top songs", tag: `TOP CHART • ${chart.badge}`,
  bgGradient: "var(--m-paper)", accentColor: "var(--m-red)", cover_art_url: chart.image,
});
