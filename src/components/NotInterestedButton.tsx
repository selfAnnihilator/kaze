import { memo, useState } from "react";
import { ThumbsDown } from "lucide-react";
import { useNotInterested, recommendationPreferences } from "../services/recommendationPreferences";
import { dispatchCommand } from "../services/api";

export const NotInterestedButton = memo(function NotInterestedButton({ trackId, title, notInterested: initial = false }: { trackId: string; title: string; notInterested?: boolean }) {
  const notInterested = useNotInterested(trackId, initial);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const label = notInterested ? `Allow ${title} in recommendations again` : `Not interested in ${title}`;
  return <>
    <button data-track-id={trackId} type="button" className="player-icon-btn not-interested-button" aria-label={label} title={label} aria-pressed={notInterested} disabled={busy} onClick={async event => {
      event.stopPropagation(); setBusy(true); setError("");
      try { await dispatchCommand({ command: notInterested ? "RemoveTrackFeedback" : "DislikeTrack", payload: { track_id: trackId } }); recommendationPreferences.update([trackId], !notInterested); }
      catch { setError("Couldn’t save your preference. Try again."); }
      finally { setBusy(false); }
    }}><ThumbsDown size={16} /></button>
    {error && <span role="alert">{error}</span>}
  </>;
});
