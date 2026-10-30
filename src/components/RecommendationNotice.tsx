import { memo, useEffect, useRef, useState } from "react";
import { dispatchCommand } from "../services/api";
export type RecommendationNoticeValue = { id: string; title: string };
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
      await dispatchCommand({ command: "RemoveTrackFeedback", payload: { track_id: notice.id } });
      if (alive.current) onDismiss(notice);
    } catch (error) { if (alive.current) onError(String(error)); }
    finally { if (alive.current) setBusy(false); }
  };
  return <div className="recommendation-feedback-toast" role="status" onMouseEnter={() => setHover(true)} onMouseLeave={() => setHover(false)}
    onFocus={() => setFocus(true)} onBlur={e => { if (!e.currentTarget.contains(e.relatedTarget)) setFocus(false); }}>
    <span>Won’t recommend “{notice.title}” again.</span>
    <button disabled={busy} onClick={() => void undo()}>Allow again</button>
    <button aria-label="Dismiss recommendation message" disabled={busy} onClick={() => onDismiss(notice)}>×</button>
  </div>;
});
