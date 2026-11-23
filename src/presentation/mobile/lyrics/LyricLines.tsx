import { useCallback, useRef, useState, type TouchEvent } from "react";
import { LocateFixed } from "lucide-react";
import { findActiveLine } from "../../../components/views/LyricsView";
import { usePlaybackSelector } from "../../../services/playbackProgress";
import type { LyricLine } from "../../../types";

const BEFORE = 2;
const AFTER = 6;
const STEP_PX = 45;
/** Fading of lines around the current one (Figma E3: .5 .7 | current | 1 .6 .4). */
const PAST_OPACITY = [0.7, 0.5];
const NEXT_OPACITY = [1, 0.6, 0.4];

function lineClass(offset: number) {
  if (offset === 0) return "m-lyric is-now";
  return offset < 0 ? "m-lyric is-past" : "m-lyric";
}
function lineOpacity(offset: number) {
  if (offset < 0) return PAST_OPACITY[Math.min(-offset - 1, PAST_OPACITY.length - 1)];
  return NEXT_OPACITY[Math.min(offset - 1, NEXT_OPACITY.length - 1)];
}

/**
 * Timed lyrics (Figma E3): two lines before the current one, the current line
 * big and red with a bar, then what comes next. Swipe to read ahead; Sync
 * Lyrics returns to the song. Tap a line to seek. Only this leaf re-renders as
 * the song plays, and only when the line changes.
 */
export function LyricLines({ lines, onSeek }: { lines: LyricLine[]; onSeek: (secs: number) => void }) {
  const select = useCallback((position: number) => findActiveLine(lines, position + 0.18), [lines]);
  const active = usePlaybackSelector(select);
  const [shift, setShift] = useState(0);
  const touchY = useRef<number | null>(null);
  const anchor = Math.min(lines.length - 1, Math.max(0, active + shift));

  const onTouchStart = (e: TouchEvent) => { touchY.current = e.touches[0].clientY; };
  const onTouchMove = (e: TouchEvent) => {
    if (touchY.current === null) return;
    const steps = Math.trunc((touchY.current - e.touches[0].clientY) / STEP_PX);
    if (!steps) return;
    touchY.current = e.touches[0].clientY;
    setShift((s) => s + steps);
  };
  const shown = lines.slice(Math.max(0, anchor - BEFORE), anchor + AFTER + 1);
  const first = Math.max(0, anchor - BEFORE);

  return (
    <>
      <div className="m-lyrics__lines" onTouchStart={onTouchStart} onTouchMove={onTouchMove} onTouchEnd={() => { touchY.current = null; }}>
        {shown.map((line, i) => {
          const offset = first + i - active;
          return (
            <button key={`${first + i}:${line.timeSecs}`} type="button" className={lineClass(offset)}
              style={offset === 0 ? undefined : { opacity: lineOpacity(offset) }} onClick={() => { setShift(0); onSeek(line.timeSecs); }}>
              {offset === 0 && <span className="m-lyric__bar" />}
              <span>{offset === 0 ? line.text.toUpperCase() : line.text || "♪"}</span>
            </button>
          );
        })}
      </div>
      <div className="m-lyrics__sync">
        <button type="button" className="m-btn m-btn--sm" disabled={shift === 0} onClick={() => setShift(0)}>
          <LocateFixed size={18} strokeWidth={2} /><span>SYNC LYRICS</span>
        </button>
      </div>
    </>
  );
}
