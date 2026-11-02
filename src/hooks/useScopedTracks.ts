import { useCallback, useLayoutEffect, useRef, useState } from "react";
import { executeQuery } from "../services/api";
import type { Track } from "../types";

export function useScopedTracks(account: string, album?: string, query?: string, active = true) {
  const key = JSON.stringify([account, album, query, active]);
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
      const rows = album ? response.data?.tracks : response.data;
      if (!Array.isArray(rows)) throw Error("Invalid library response");
      setResult({key, rows, loading: false, error: null});
    } catch (error) {
      if (current.current === key && request === generation.current)
        setResult({key, rows: [], loading: false, error: String(error)});
    }
  }, [key, active, album, query]);
  useLayoutEffect(() => {
    void refresh();
    return () => { ++generation.current; };
  }, [refresh]);
  const setItems = useCallback((update: React.SetStateAction<Track[]>) => {
    setResult(previous => previous.key === current.current ? {...previous, rows: typeof update === "function" ? update(previous.rows) : update} : previous);
  }, []);
  return { rows: result.key === key ? result.rows : [], loading: result.key !== key || result.loading, error: result.key === key ? result.error : null, refresh, setItems };
}
