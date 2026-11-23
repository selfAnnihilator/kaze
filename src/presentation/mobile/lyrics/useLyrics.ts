import { useEffect, useMemo, useState } from "react";
import { executeQuery } from "../../../services/api";
import { parseLrc } from "../../../components/views/LyricsView";
import type { Track, TrackLyricsData } from "../../../types";

/** Lyrics for the playing song: timed lines when the song has them, otherwise plain lines. */
export function useLyrics(track: Track | null | undefined) {
  const [data, setData] = useState<TrackLyricsData | null>(null);
  const [state, setState] = useState<"idle" | "loading" | "error">("idle");
  const id = track?.id;
  const artist = track?.artist_name || "";
  const title = track?.title || "";
  const duration = track?.duration_secs;
  useEffect(() => {
    setData(null);
    if (!title) return;
    let active = true;
    setState("loading");
    executeQuery({ query: "GetTrackLyrics", payload: { track_id: id, artist, title, duration_secs: duration } })
      .then((res) => { if (active) { setData((res.data as TrackLyricsData) || null); setState("idle"); } })
      .catch(() => { if (active) setState("error"); });
    return () => { active = false; };
  }, [id, artist, title, duration]);
  const synced = useMemo(() => (data?.syncedLyrics ? parseLrc(data.syncedLyrics) : []), [data?.syncedLyrics]);
  const plain = useMemo(
    () => (synced.length ? [] : (data?.plainLyrics || "").split(/\r?\n/).map((l) => l.trim()).filter(Boolean)),
    [synced.length, data?.plainLyrics],
  );
  return { synced, plain, instrumental: !!data?.instrumental, loading: state === "loading", failed: state === "error" };
}
