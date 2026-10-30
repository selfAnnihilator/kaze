import { memo, useEffect, useState } from "react";
const eventName = "kaze-empty-playback";
export function showEmptyPlaybackNotice() { window.dispatchEvent(new Event(eventName)); }
/** Ephemeral status only: no notification history, intervals, or root timers. */
export const PlaybackNotice = memo(function PlaybackNotice() {
  const [notice, setNotice] = useState<object | null>(null);
  useEffect(() => {
    const show = () => setNotice({});
    window.addEventListener(eventName, show);
    return () => window.removeEventListener(eventName, show);
  }, []);
  useEffect(() => {
    if (!notice) return;
    const timer = setTimeout(() => setNotice(null), 2000);
    return () => clearTimeout(timer);
  }, [notice]);
  return notice ? <div className="playback-notice" role="status">Select a song to play</div> : null;
});
