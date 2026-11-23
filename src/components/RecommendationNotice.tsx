import { memo, useEffect, useRef, useState } from "react";
import { dispatchCommand } from "../services/api";
import type { EntityFeedbackRow } from "../types";
export type RecommendationNoticeValue = { id: string; title: string; entities?: EntityFeedbackRow[] };

const joinNames = (names: string[]) => names.length < 2 ? names.join("") : `${names.slice(0, -1).join(", ")} and ${names[names.length - 1]}`;
export function recommendationNoticeText(notice: RecommendationNoticeValue): string {
  const rows = notice.entities || [];
  if (!rows.length) return `Won’t recommend “${notice.title}” again.`;
  const names = joinNames(rows.map(row => row.display));
  return rows.some(row => row.level === "block") ? `Won’t recommend ${names} again.` : `Showing less of ${names}.`;
}
/** Takes back the feedback a notice reports (shared by the PC toast and the phone notice). */
export async function undoRecommendationFeedback(notice: RecommendationNoticeValue) {
  if (!notice.entities?.length) {
    await dispatchCommand({ command: "RemoveTrackFeedback", payload: { track_id: notice.id } });
    return;
  }
  for (const row of notice.entities) await dispatchCommand({ command: "ClearEntityFeedback", payload: { kind: row.kind, key: row.key } });
}

export const RecommendationNotice = memo(function RecommendationNotice({ notice, onDismiss, onError }: {
  notice: RecommendationNoticeValue;
  onDismiss: (notice: RecommendationNoticeValue) => void;
  onError: (message: string) => void;
}) {
  const [hover, setHover] = useState(false), [focus, setFocus] = useState(false), [busy, setBusy] = useState(false);
  const remaining = useRef(5000), alive = useRef(true);
  useEffect(() => { alive.current = true; return () => { alive.current = false; }; }, []);
  useEffect(() => { remaining.current = 5000; }, [notice]);
  useEffect(() => {
    if (hover || focus || busy) return;
    const start = performance.now();
    const timer = setTimeout(() => onDismiss(notice), remaining.current);
    return () => { clearTimeout(timer); remaining.current = Math.max(0, remaining.current - (performance.now() - start)); };
  }, [notice, hover, focus, busy, onDismiss]);
  const undo = async () => {
    if (busy) return;
    setBusy(true);
    try {
      await undoRecommendationFeedback(notice);
      if (alive.current) onDismiss(notice);
    } catch (error) { if (alive.current) onError(String(error)); }
    finally { if (alive.current) setBusy(false); }
  };
  return <div className="recommendation-feedback-toast" role="status" onMouseEnter={() => setHover(true)} onMouseLeave={() => setHover(false)}
    onFocus={() => setFocus(true)} onBlur={e => { if (!e.currentTarget.contains(e.relatedTarget)) setFocus(false); }}>
    <span>{recommendationNoticeText(notice)}</span>
    <button disabled={busy} onClick={() => void undo()}>{notice.entities?.length ? "Undo" : "Allow again"}</button>
    <button aria-label="Dismiss recommendation message" disabled={busy} onClick={() => onDismiss(notice)}>×</button>
  </div>;
});
