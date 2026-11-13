import { memo, useEffect, useRef, useState } from "react";
import { ThumbsDown } from "lucide-react";
import { useNotInterested, recommendationPreferences } from "../services/recommendationPreferences";
import { dispatchCommand } from "../services/api";
import type { Command } from "../types";

type Choice = "song" | "artist" | "less";
const CHOICES: { id: Choice; label: string }[] = [
  { id: "song", label: "Not this song" },
  { id: "artist", label: "Not this artist" },
  { id: "less", label: "Less like this" },
];

export const NotInterestedButton = memo(function NotInterestedButton({ trackId, title, notInterested: initial = false }: { trackId: string; title: string; notInterested?: boolean }) {
  const notInterested = useNotInterested(trackId, initial);
  const [busy, setBusy] = useState(false);
  const [open, setOpen] = useState(false);
  const [error, setError] = useState("");
  const root = useRef<HTMLSpanElement>(null);
  useEffect(() => {
    if (!open) return;
    const close = (event: Event) => { if (!root.current?.contains(event.target as Node)) setOpen(false); };
    const escape = (event: KeyboardEvent) => { if (event.key === "Escape") setOpen(false); };
    document.addEventListener("pointerdown", close);
    document.addEventListener("keydown", escape);
    return () => { document.removeEventListener("pointerdown", close); document.removeEventListener("keydown", escape); };
  }, [open]);
  const run = async (command: Command, after?: () => void) => {
    setBusy(true); setError(""); setOpen(false);
    try { await dispatchCommand(command); after?.(); }
    catch { setError("Couldn’t save your preference. Try again."); }
    finally { setBusy(false); }
  };
  const choose = (choice: Choice) => {
    const payload = { track_id: trackId };
    if (choice === "song") return run({ command: "DislikeTrack", payload }, () => recommendationPreferences.update([trackId], true));
    return run(choice === "artist" ? { command: "BlockArtist", payload } : { command: "LessLikeThis", payload });
  };
  const label = notInterested ? `Allow ${title} in recommendations again` : `Not interested in ${title}`;
  return <span ref={root} className="not-interested-control" onClick={event => event.stopPropagation()}>
    <button data-track-id={trackId} type="button" className="player-icon-btn not-interested-button" aria-label={label} title={label}
      aria-pressed={notInterested} aria-haspopup={notInterested ? undefined : "menu"} aria-expanded={notInterested ? undefined : open} disabled={busy}
      onClick={() => notInterested
        ? void run({ command: "RemoveTrackFeedback", payload: { track_id: trackId } }, () => recommendationPreferences.update([trackId], false))
        : setOpen(value => !value)}><ThumbsDown size={16} /></button>
    {open && <span role="menu" className="not-interested-menu">
      {CHOICES.map(choice => <button key={choice.id} type="button" role="menuitem" onClick={() => void choose(choice.id)}>{choice.label}</button>)}
    </span>}
    {error && <span role="alert">{error}</span>}
  </span>;
});
