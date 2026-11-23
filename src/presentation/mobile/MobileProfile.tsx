import { memo, useEffect, useRef } from "react";
import { PullIndicator } from "./pullRefresh/PullIndicator";
import { usePullToRefresh } from "./pullRefresh/usePullToRefresh";
import { cloudSyncSignal } from "../../services/cloudSyncSignal";
import { MonthGraph, TimeBoxes, YearChips } from "./profile/ProfileStats";
import { ProfileHead } from "./profile/ProfileHead";
import { ProfileTopLists } from "./profile/ProfileTopLists";
import { useProfileStats } from "./profile/useProfileStats";
import { YearReview } from "./profile/YearReview";
import type { MobileScreenProps } from "./types";

/**
 * Profile (Figma F1 Stats + F2 Top Lists, one scroll): photo and name, year
 * chips, today / this week, the month's bars, then top songs, artists and genres.
 * Earlier years open as their Year in Review (Figma F3), then their top lists.
 */
export const MobileProfile = memo(function MobileProfile({ c }: MobileScreenProps) {
  const p = useProfileStats(c);
  const { reload } = p;
  // Another device's listening arrived through cloud sync: reload in place.
  useEffect(() => cloudSyncSignal.subscribe(() => void reload()), [reload]);
  const pull = useRef<HTMLDivElement>(null);
  usePullToRefresh(pull, reload);
  return (
    <div className="m-profile">
      <PullIndicator ref={pull} />
      <ProfileHead c={c} p={p} />
      <YearChips p={p} />
      {p.failed && !p.stats && <p className="m-empty-msg">Couldn’t load your stats. Pull down to try again.</p>}
      {p.stats && (p.isPastYear ? <YearReview p={p} /> : <><TimeBoxes p={p} /><MonthGraph p={p} /></>)}
      {p.stats && <ProfileTopLists c={c} p={p} />}
      {p.loading && !p.stats && <p className="m-empty-msg">Loading your stats…</p>}
    </div>
  );
});
