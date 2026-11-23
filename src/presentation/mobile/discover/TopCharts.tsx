import { useEffect, useState } from "react";
import { chartCollection, chartFilters, useCharts, type ChartCard } from "./useCharts";
import type { MobileScreenProps } from "../types";

type Ctx = MobileScreenProps["c"];

/** "Global Top 50" → "TOP 50"; otherwise "TOP". */
const headline = (title: string) => (title.match(/top\s*\d+/i)?.[0] ?? "Top").toUpperCase();

function Chart({ chart, c }: { chart: ChartCard; c: Ctx }) {
  return (
    <button type="button" className="m-chart" onClick={() => c.handleOpenCollection(chartCollection(chart))}>
      <span className="m-chart__art">
        {chart.image && <img className="m-chart__img" src={chart.image} alt="" loading="lazy" />}
        <span className="m-chart__badge">{chart.badge}</span>
        {!chart.image && <span className="m-chart__top">{headline(chart.title)}</span>}
        <span className="m-chart__rule" />
      </span>
      <strong className="m-chart__title">{chart.title}</strong>
      <span className="m-profile-meta">UPDATED WEEKLY</span>
    </button>
  );
}

/** Top Charts (Figma C3): filter chips, then the charts in one row that scrolls sideways. */
export function TopCharts({ c, force }: { c: Ctx; force: boolean }) {
  const { charts, failed, refresh } = useCharts();
  const [filter, setFilter] = useState("ALL");
  useEffect(() => { if (force) void refresh(); }, [force]); // eslint-disable-line react-hooks/exhaustive-deps
  const all = charts ?? [];
  const filters = chartFilters(all);
  const active = filters.includes(filter) ? filter : "ALL";
  const shown = active === "ALL" ? all : all.filter((ch) => ch.group === active);
  return (
    <>
      {filters.length > 1 && (
        <div className="m-chart-tabs">
          {filters.map((f) => (
            <button key={f} type="button" className={`m-profile-chip ${f === active ? "is-on" : ""}`} aria-pressed={f === active} onClick={() => setFilter(f)}>{f}</button>
          ))}
        </div>
      )}
      <div className="m-chart-row">{shown.map((ch) => <Chart key={ch.id} chart={ch} c={c} />)}</div>
      {charts && !shown.length && <p className="m-empty-msg">No charts here yet. Tap refresh to load them again.</p>}
      {failed && <p className="m-empty-msg">Couldn’t load charts. Check your connection and tap refresh.</p>}
      {!charts && !failed && <p className="m-empty-msg">Loading charts…</p>}
    </>
  );
}
