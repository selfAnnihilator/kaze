import React, { useState, useEffect, useRef, useMemo } from "react";
import { Mic2, RefreshCw, Radio } from "lucide-react";
import { executeQuery } from "../../services/api";
import { LyricLine, TrackLyricsData } from "../../types";

interface LyricsViewProps {
  trackId?: string;
  artist: string;
  title: string;
  durationSecs?: number;
  currentTime: number;
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

export const LyricsView: React.FC<LyricsViewProps> = ({
  trackId,
  artist,
  title,
  durationSecs,
  currentTime,
  onSeek,
  isFullScreen = false,
}) => {
  const [lyricsData, setLyricsData] = useState<TrackLyricsData | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isManualScroll, setIsManualScroll] = useState(false);
  const [isOutOfSync, setIsOutOfSync] = useState(false);

  const containerRef = useRef<HTMLDivElement>(null);
  const lineRefs = useRef<(HTMLDivElement | null)[]>([]);
  const scrollTimeoutRef = useRef<any>(null);

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

  // Determine active lyric line index (advancing by +0.18s to eliminate the 0.1-0.2s audio delay)
  const activeLineIndex = useMemo(() => {
    if (parsedLines.length === 0) return -1;
    const effectiveTime = currentTime + 0.18;
    let activeIdx = -1;
    for (let i = 0; i < parsedLines.length; i++) {
      if (effectiveTime >= parsedLines[i].timeSecs) {
        activeIdx = i;
      } else {
        break;
      }
    }
    return activeIdx;
  }, [parsedLines, currentTime]);

  // Auto-scroll active line to vertical center of container
  useEffect(() => {
    if (isManualScroll || activeLineIndex < 0) return;
    const lineEl = lineRefs.current[activeLineIndex];
    const containerEl = containerRef.current;
    if (lineEl && containerEl) {
      const targetScroll =
        lineEl.offsetTop - containerEl.clientHeight / 2 + lineEl.clientHeight / 2;
      containerEl.scrollTo({
        top: Math.max(0, targetScroll),
        behavior: "smooth",
      });
      setIsOutOfSync(false);
    }
  }, [activeLineIndex, isManualScroll]);

  // Handle manual scroll detection
  const handleScroll = () => {
    if (!containerRef.current || activeLineIndex < 0) return;
    const lineEl = lineRefs.current[activeLineIndex];
    if (lineEl) {
      const targetScroll =
        lineEl.offsetTop - containerRef.current.clientHeight / 2 + lineEl.clientHeight / 2;
      const diff = Math.abs(containerRef.current.scrollTop - targetScroll);
      if (diff > 50) {
        setIsManualScroll(true);
        setIsOutOfSync(true);
      } else {
        setIsManualScroll(false);
        setIsOutOfSync(false);
      }
    }

    if (scrollTimeoutRef.current) clearTimeout(scrollTimeoutRef.current);
    scrollTimeoutRef.current = setTimeout(() => {
      // Re-enable live tracking after user stops interacting
      setIsManualScroll(false);
      setIsOutOfSync(false);
    }, 10000);
  };

  const handleSyncClick = () => {
    setIsManualScroll(false);
    setIsOutOfSync(false);
    if (activeLineIndex >= 0) {
      const lineEl = lineRefs.current[activeLineIndex];
      const containerEl = containerRef.current;
      if (lineEl && containerEl) {
        const targetScroll =
          lineEl.offsetTop - containerEl.clientHeight / 2 + lineEl.clientHeight / 2;
        containerEl.scrollTo({
          top: Math.max(0, targetScroll),
          behavior: "smooth",
        });
      }
    }
  };

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
          <span style={{ fontSize: "0.95rem" }}>Finding synchronized lyrics...</span>
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
          <h3 style={{ color: "#fff", fontSize: "1.1rem", marginBottom: "8px" }}>
            {lyricsData?.instrumental ? "Instrumental Track" : "No Lyrics Available"}
          </h3>
          <p style={{ fontSize: "0.88rem", lineHeight: 1.5 }}>
            {lyricsData?.instrumental
              ? "This track is marked as an instrumental piece."
              : `We couldn't find synchronized lyrics for "${title}" by ${artist}.`}
          </p>
        </div>
      ) : parsedLines.length > 0 ? (
        // Synced Lyrics Mode - Centered
        <div
          ref={containerRef}
          onScroll={handleScroll}
          className="custom-scrollbar"
          style={{
            width: "100%",
            height: "100%",
            overflowY: "auto",
            paddingTop: "38vh",
            paddingBottom: "38vh",
            paddingLeft: isFullScreen ? "8vw" : "24px",
            paddingRight: isFullScreen ? "8vw" : "24px",
            scrollBehavior: "smooth",
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            textAlign: "center",
            gap: "20px",
          }}
        >
          {parsedLines.map((line, idx) => {
            const isActive = idx === activeLineIndex;
            const isPast = idx < activeLineIndex;

            return (
              <div
                key={idx}
                ref={(el) => {
                  lineRefs.current[idx] = el;
                }}
                onClick={() => {
                  setIsManualScroll(false);
                  setIsOutOfSync(false);
                  onSeek(line.timeSecs);
                }}
                style={{
                  fontSize: isFullScreen ? "2.2rem" : "1.55rem",
                  fontWeight: 700,
                  lineHeight: 1.45,
                  cursor: "pointer",
                  transition: "all 0.25s cubic-bezier(0.4, 0, 0.2, 1)",
                  color: isActive
                    ? "#ffffff"
                    : isPast
                    ? "rgba(255, 255, 255, 0.45)"
                    : "rgba(255, 255, 255, 0.3)",
                  transform: isActive ? "scale(1.04)" : "none",
                  transformOrigin: "center center",
                  textAlign: "center",
                  filter: isActive ? "drop-shadow(0 2px 14px rgba(0,0,0,0.6))" : "none",
                  userSelect: "none",
                  maxWidth: "960px",
                  width: "100%",
                }}
                onMouseEnter={(e) => {
                  if (!isActive) e.currentTarget.style.color = "rgba(255, 255, 255, 0.85)";
                }}
                onMouseLeave={(e) => {
                  if (!isActive)
                    e.currentTarget.style.color = isPast
                      ? "rgba(255, 255, 255, 0.45)"
                      : "rgba(255, 255, 255, 0.3)";
                }}
              >
                {line.text || "♪"}
              </div>
            );
          })}
        </div>
      ) : (
        // Plain Lyrics Fallback - Centered
        <div
          ref={containerRef}
          className="custom-scrollbar"
          style={{
            width: "100%",
            height: "100%",
            overflowY: "auto",
            padding: isFullScreen ? "8vh 8vw" : "32px",
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            textAlign: "center",
            gap: "14px",
          }}
        >
          {lyricsData?.plainLyrics?.split(/\r?\n/).map((line, idx) => (
            <div
              key={idx}
              style={{
                fontSize: isFullScreen ? "1.8rem" : "1.3rem",
                fontWeight: 600,
                lineHeight: 1.6,
                color: "rgba(255, 255, 255, 0.75)",
                textAlign: "center",
                maxWidth: "960px",
                width: "100%",
              }}
            >
              {line || " "}
            </div>
          ))}
        </div>
      )}

      {/* Floating Sync Button - Only appears when lyrics and playing song are out of sync */}
      {parsedLines.length > 0 && isOutOfSync && (
        <button
          type="button"
          onClick={handleSyncClick}
          title="Lyrics out of sync — click to sync lyrics to song"
          style={{
            position: "absolute",
            bottom: "24px",
            left: "50%",
            transform: "translateX(-50%)",
            backgroundColor: "#ffffff",
            color: "#0f172a",
            padding: "8px 22px",
            borderRadius: "9999px",
            border: "none",
            boxShadow: "0 6px 22px rgba(0, 0, 0, 0.45)",
            display: "inline-flex",
            alignItems: "center",
            gap: "8px",
            fontWeight: 700,
            fontSize: "0.85rem",
            cursor: "pointer",
            transition: "all 0.2s cubic-bezier(0.4, 0, 0.2, 1)",
            zIndex: 10,
          }}
          onMouseEnter={(e) => {
            e.currentTarget.style.transform = "translateX(-50%) scale(1.05)";
          }}
          onMouseLeave={(e) => {
            e.currentTarget.style.transform = "translateX(-50%) scale(1)";
          }}
        >
          <Radio size={15} color="#0f172a" />
          <span>Sync lyrics</span>
        </button>
      )}
    </div>
  );
};
