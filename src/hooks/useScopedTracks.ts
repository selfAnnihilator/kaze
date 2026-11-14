import { useCallback, useLayoutEffect, useRef, useState } from "react";
import { executeQuery } from "../services/api";
import type { Track } from "../types";

import { inSection } from "../downloadsFolder";

/** Album or search results for one section: Local (your folders) or Downloads (Kaze's folder). */
export function useScopedTracks({ account, album, query, active = true, downloadsDir, section = "local" }: {
  account: string;
  album?: string;
  query?: string;
  active?: boolean;
  downloadsDir?: string;
  section?: "local" | "downloads";
}) {
  const key = JSON.stringify([account, album, query, active, downloadsDir, section]);
  const current = useRef(key);
  current.current = key;
  const generation = useRef(0);
  const [result, setResult] = useState<{key: string; rows: Track[]; loading: boolean; error: string | null}>({key: "", rows: [], loading: true, error: null});
  const refresh = useCallback(async () => {
    const request = ++generation.current;
    if (!active || (!album && !query)) return;
    setResult({key, rows: [], loading: true, error: null});
    try {
      const response = await executeQuery(album
        ? {query: "GetAlbumDetails", payload: {album_id: album}}
        : {query: "SearchLibrary", payload: {query_text: query!, limit: 100}});
      if (current.current !== key || request !== generation.current) return;
      const found = album ? response.data?.tracks : response.data;
      if (!Array.isArray(found)) throw Error("Invalid library response");
      // Local lists the user's own files only (no cached online tracks, nothing
      // from Kaze's folder); Downloads lists only Kaze's folder.
      const rows = (album ? found : found.filter((t: Track) => !String(t.file_path ?? "").startsWith("online://")))
        .filter((t: Track) => inSection(t, downloadsDir, section));
      setResult({key, rows, loading: false, error: null});
    } catch (error) {
      if (current.current === key && request === generation.current)
        setResult({key, rows: [], loading: false, error: String(error)});
    }
  }, [key, active, album, query, downloadsDir, section]);
  useLayoutEffect(() => {
    void refresh();
    return () => { ++generation.current; };
  }, [refresh]);
  const setItems = useCallback((update: React.SetStateAction<Track[]>) => {
    setResult(previous => previous.key === current.current ? {...previous, rows: typeof update === "function" ? update(previous.rows) : update} : previous);
  }, []);
  return { rows: result.key === key ? result.rows : [], loading: result.key !== key || result.loading, error: result.key === key ? result.error : null, refresh, setItems };
}
