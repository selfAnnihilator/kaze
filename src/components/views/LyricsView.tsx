import React, { useState, useEffect, useRef, useMemo, useCallback } from "react";
import { List, useDynamicRowHeight, useListRef, type RowComponentProps } from "react-window";
import { Mic2, RefreshCw, Radio } from "lucide-react";
import { executeQuery } from "../../services/api";
import { LyricLine, TrackLyricsData } from "../../types";
import { usePlaybackProgress } from "../../services/playbackProgress";

interface LyricsViewProps {
  trackId?: string;
  artist: string;
  title: string;
  durationSecs?: number;
  currentTime?: number;
  onSeek: (timeSecs: number) => void;
  isFullScreen?: boolean;
}

// Parses LRC string format into timestamped LyricLine array
function parseLrc(lrcText: string): LyricLine[] {
  const lines = lrcText.split(/\r?\n/);
  const result: LyricLine[] = [];
  const timeRegex = /\[(\d{1,2}):(\d{2}(?:\.\d+)?)\]/g;

  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed) continue;

    // Match all timestamps in the line (some lines have multiple e.g. [00:12.30][01:14.20]text)
    const matches = Array.from(trimmed.matchAll(timeRegex));
    if (matches.length > 0) {
      const text = trimmed.replace(timeRegex, "").trim();
      for (const match of matches) {
        const mins = parseInt(match[1], 10);
        const secs = parseFloat(match[2]);
        const timeSecs = mins * 60 + secs;
        result.push({ timeSecs, text });
      }
    }
  }

  result.sort((a, b) => a.timeSecs - b.timeSecs);
  return result;
}

const SCROLL_KEYS = new Set(["ArrowUp", "ArrowDown", "PageUp", "PageDown", "Home", "End", " "]);

const LINE_COLOR_PAST = "var(--overlay-45)";
const LINE_COLOR_FUTURE = "var(--overlay-30)";
const LINE_COLOR_HOVER = "var(--overlay-85)";

/** Index of the last line with `timeSecs <= time`, or -1 before the first line. */
function findActiveLine(lines: LyricLine[], time: number): number {
  let lo = 0;
  let hi = lines.length - 1;
  let found = -1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    if (lines[mid].timeSecs <= time) {
      found = mid;
      lo = mid + 1;
    } else {
      hi = mid - 1;
    }
  }
  return found;
}

interface SyncedRowProps {
  lines: LyricLine[];
  activeLineIndex: number;
  isFullScreen: boolean;
  onLineClick: (timeSecs: number) => void;
}

// Row 0 and the last row are spacers so the first and last lines can centre.
function SyncedLyricRow({
  index,
  style,
  ariaAttributes,
  lines,
  activeLineIndex,
  isFullScreen,
  onLineClick,
}: RowComponentProps<SyncedRowProps>) {
  if (index === 0 || index === lines.length + 1) {
    // No aria-hidden: List skips aria-hidden children when measuring heights.
    return <div {...ariaAttributes} role="presentation" style={{ ...style, height: "38vh" }} />;
  }
  const lineIndex = index - 1;
  const line = lines[lineIndex];
  const isActive = lineIndex === activeLineIndex;
  const restColor = isActive ? "var(--text-main)" : lineIndex < activeLineIndex ? LINE_COLOR_PAST : LINE_COLOR_FUTURE;

  return (
    <div
      {...ariaAttributes}
      style={{
        ...style,
        padding: isFullScreen ? "10px 8vw" : "10px 24px",
        boxSizing: "border-box",
      }}
    >
      <div
        onClick={() => onLineClick(line.timeSecs)}
        onMouseEnter={(e) => {
          if (!isActive) e.currentTarget.style.color = LINE_COLOR_HOVER;
        }}
        onMouseLeave={(e) => {
          e.currentTarget.style.color = restColor;
        }}
        style={{
          fontSize: isFullScreen ? "2.2rem" : "1.55rem",
          fontWeight: "var(--weight-bold)",
          lineHeight: 1.45,
          cursor: "pointer",
          color: restColor,
          transform: isActive ? "scale(1.04)" : "none",
          transformOrigin: "center center",
          transition: "transform 0.25s cubic-bezier(0.4, 0, 0.2, 1)",
          textAlign: "center",
          textShadow: isActive ? "0 2px 14px var(--shade-60)" : "none",
          userSelect: "none",
          maxWidth: "960px",
          margin: "0 auto",
        }}
      >
        {line.text || "♪"}
      </div>
    </div>
  );
}

interface PlainRowProps {
  lines: string[];
  isFullScreen: boolean;
}

function PlainLyricRow({ index, style, ariaAttributes, lines, isFullScreen }: RowComponentProps<PlainRowProps>) {
  // Edge padding lives on the first/last rows: List positions rows absolutely,
  // so padding on the scroll container would not offset them.
  const edge = isFullScreen ? "8vh" : "32px";
  return (
    <div
      {...ariaAttributes}
      style={{
        ...style,
        paddingTop: index === 0 ? edge : "7px",
        paddingBottom: index === lines.length - 1 ? edge : "7px",
        paddingInline: isFullScreen ? "8vw" : "32px",
        boxSizing: "border-box",
      }}
    >
      <div
        style={{
          fontSize: isFullScreen ? "1.8rem" : "1.3rem",
          fontWeight: "var(--weight-semibold)",
          lineHeight: 1.6,
          color: "var(--overlay-75)",
          textAlign: "center",
          maxWidth: "960px",
          margin: "0 auto",
        }}
      >
        {lines[index] || "\u00a0"}
      </div>
    </div>
  );
}

export const LyricsView: React.FC<LyricsViewProps> = ({
  trackId,
  artist,
  title,
  durationSecs,
  currentTime,
  onSeek,
  isFullScreen = false,
}) => {
  const liveProgress = usePlaybackProgress(durationSecs);
  const effectiveCurrentTime = currentTime !== undefined ? currentTime : liveProgress.position;
  const [lyricsData, setLyricsData] = useState<TrackLyricsData | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isManualScroll, setIsManualScroll] = useState(false);
  const [isOutOfSync, setIsOutOfSync] = useState(false);

  const scrollTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => () => {
    if (scrollTimeoutRef.current) clearTimeout(scrollTimeoutRef.current);
  }, []);

  // Fetch lyrics whenever artist or title changes
  useEffect(() => {
    if (!artist && !title) return;

    let isMounted = true;
    setIsLoading(true);
    setError(null);
    setLyricsData(null);
    setIsManualScroll(false);
    setIsOutOfSync(false);

    executeQuery({
      query: "GetTrackLyrics",
      payload: {
        track_id: trackId,
        artist,
        title,
        duration_secs: durationSecs,
      },
    })
      .then((res) => {
        if (!isMounted) return;
        if (res.data) {
          setLyricsData(res.data as TrackLyricsData);
        } else {
          setLyricsData(null);
        }
      })
      .catch((err) => {
        if (!isMounted) return;
        console.error("Failed to load lyrics:", err);
        setError("Could not load lyrics for this track.");
      })
      .finally(() => {
        if (isMounted) setIsLoading(false);
      });

    return () => {
      isMounted = false;
    };
  }, [trackId, artist, title, durationSecs]);

  // Parse synced lyrics if available
  const parsedLines = useMemo(() => {
    if (lyricsData?.syncedLyrics) {
      return parseLrc(lyricsData.syncedLyrics);
    }
    return [];
  }, [lyricsData?.syncedLyrics]);

  // Active line = last line whose timestamp has passed (advanced by +0.18s to
  // cover the audio output delay). Binary search: lines are sorted by time.
  const activeLineIndex = useMemo(
    () => findActiveLine(parsedLines, effectiveCurrentTime + 0.18),
    [parsedLines, effectiveCurrentTime],
  );

  const listRef = useListRef(null);
  const rowHeightKey = `${trackId ?? `${artist}-${title}`}-${isFullScreen ? "fs" : "inline"}`;
  const syncedRowHeight = useDynamicRowHeight({ defaultRowHeight: 64, key: rowHeightKey });
  const plainRowHeight = useDynamicRowHeight({ defaultRowHeight: 56, key: rowHeightKey });

  const scrollToActive = useCallback((index: number) => {
    // +1 skips the leading spacer row.
    listRef.current?.scrollToRow({ index: index + 1, align: "center", behavior: "smooth" });
  }, [listRef]);

  // Auto-scroll the active line to the vertical centre of the list
  useEffect(() => {
    if (isManualScroll || activeLineIndex < 0) return;
    scrollToActive(activeLineIndex);
    setIsOutOfSync(false);
  }, [activeLineIndex, isManualScroll, scrollToActive]);

  // A user-initiated scroll (wheel, touch, keyboard, scrollbar drag) pauses
  // live tracking; it resumes after 10s of inactivity or via the sync button.
  const handleUserScroll = useCallback(() => {
    setIsManualScroll(true);
    setIsOutOfSync(true);
    if (scrollTimeoutRef.current) clearTimeout(scrollTimeoutRef.current);
    scrollTimeoutRef.current = setTimeout(() => {
      setIsManualScroll(false);
      setIsOutOfSync(false);
    }, 10000);
  }, []);

  const handlePointerDown = useCallback((e: React.PointerEvent<HTMLElement>) => {
    // Only the scrollbar targets the scroll container itself; clicks on a
    // line are seeks, not manual scrolls.
    if (e.target === e.currentTarget) handleUserScroll();
  }, [handleUserScroll]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent<HTMLElement>) => {
    if (SCROLL_KEYS.has(e.key)) handleUserScroll();
  }, [handleUserScroll]);

  const handleLineClick = useCallback((timeSecs: number) => {
    if (scrollTimeoutRef.current) clearTimeout(scrollTimeoutRef.current);
    setIsManualScroll(false);
    setIsOutOfSync(false);
    onSeek(timeSecs);
  }, [onSeek]);

  const handleSyncClick = () => {
    if (scrollTimeoutRef.current) clearTimeout(scrollTimeoutRef.current);
    setIsManualScroll(false);
    setIsOutOfSync(false);
    if (activeLineIndex >= 0) scrollToActive(activeLineIndex);
  };

  const syncedRowProps = useMemo<SyncedRowProps>(
    () => ({ lines: parsedLines, activeLineIndex, isFullScreen, onLineClick: handleLineClick }),
    [parsedLines, activeLineIndex, isFullScreen, handleLineClick],
  );

  const plainLines = useMemo(
    () => lyricsData?.plainLyrics?.split(/\r?\n/) ?? [],
    [lyricsData?.plainLyrics],
  );
  const plainRowProps = useMemo<PlainRowProps>(
    () => ({ lines: plainLines, isFullScreen }),
    [plainLines, isFullScreen],
  );

  return (
    <div
      style={{
        position: "relative",
        width: "100%",
        height: "100%",
        display: "flex",
        flexDirection: "column",
        alignItems: "center",
        justifyContent: "center",
        overflow: "hidden",
        backgroundColor: isFullScreen ? "transparent" : "var(--bg-card)",
        borderRadius: isFullScreen ? 0 : "12px",
        border: isFullScreen ? "none" : "1px solid var(--border)",
      }}
    >
      {isLoading ? (
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            gap: "14px",
            color: "var(--text-muted)",
          }}
        >
          <RefreshCw size={28} className="spin-animation" color="var(--accent-light)" />
          <span style={{ fontSize: "var(--fs-15)" }}>Finding synchronized lyrics...</span>
        </div>
      ) : error || (!lyricsData?.syncedLyrics && !lyricsData?.plainLyrics) ? (
        <div
          style={{
            textAlign: "center",
            padding: "40px 20px",
            color: "var(--text-muted)",
            maxWidth: "400px",
          }}
        >
          <Mic2 size={42} color="var(--text-dim)" style={{ marginBottom: "16px" }} />
          <h3 style={{ color: "var(--text-main)", fontSize: "var(--fs-18)", marginBottom: "8px" }}>
            {lyricsData?.instrumental ? "Instrumental Track" : "No Lyrics Available"}
          </h3>
          <p style={{ fontSize: "var(--fs-14)", lineHeight: 1.5 }}>
            {lyricsData?.instrumental
              ? "This track is marked as an instrumental piece."
              : `We couldn't find synchronized lyrics for "${title}" by ${artist}.`}
          </p>
        </div>
      ) : parsedLines.length > 0 ? (
        // Synced Lyrics Mode - Centered, virtualized
        <List<SyncedRowProps>
          listRef={listRef}
          className="custom-scrollbar"
          rowCount={parsedLines.length + 2}
          rowHeight={syncedRowHeight}
          rowComponent={SyncedLyricRow}
          rowProps={syncedRowProps}
          overscanCount={6}
          tabIndex={0}
          onWheel={handleUserScroll}
          onTouchMove={handleUserScroll}
          onPointerDown={handlePointerDown}
          onKeyDown={handleKeyDown}
          style={{ width: "100%", height: "100%", outline: "none" }}
        />
      ) : (
        // Plain Lyrics Fallback - Centered, virtualized
        <List<PlainRowProps>
          className="custom-scrollbar"
          rowCount={plainLines.length}
          rowHeight={plainRowHeight}
          rowComponent={PlainLyricRow}
          rowProps={plainRowProps}
          overscanCount={6}
          style={{ width: "100%", height: "100%" }}
        />
      )}

      {/* Floating Sync Button - Only appears when lyrics and playing song are out of sync */}
      {parsedLines.length > 0 && isOutOfSync && (
        <button
          className="lyrics-sync-button"
          type="button"
          onClick={handleSyncClick}
          title="Lyrics out of sync — click to sync lyrics to song"
          style={{
            position: "absolute",
            bottom: "24px",
            left: "50%",
            transform: "translateX(-50%)",
            backgroundColor: "var(--text-main)",
            color: "var(--palette-blue-900)",
            padding: "8px 22px",
            borderRadius: "9999px",
            border: "none",
            boxShadow: "0 6px 22px var(--shade-45)",
            display: "inline-flex",
            alignItems: "center",
            gap: "8px",
            fontWeight: "var(--weight-bold)",
            fontSize: "var(--fs-14)",
            cursor: "pointer",
            transition: "transform 0.2s cubic-bezier(0.4, 0, 0.2, 1), opacity 0.2s cubic-bezier(0.4, 0, 0.2, 1)",
            zIndex: 10,
          }}
        >
          <Radio size={15} color="var(--palette-blue-900)" />
          <span>Sync lyrics</span>
        </button>
      )}
    </div>
  );
};
