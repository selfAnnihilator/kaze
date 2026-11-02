import { useCallback, useLayoutEffect, useRef, useState } from "react";
import { executeQuery } from "../services/api";

/** Paging owns catalogue state; progress ticks never enter this hook. */
export function useLibraryPage<T extends { id: string }>(
  query: "GetLibraryTracksPage" | "GetLibraryAlbumsPage",
  size: number,
  identity: string,
  navigation: string,
) {
  const [items, setItems] = useState<T[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const state = useRef({ offset: 0, total: 0, generation: 0, pending: false, failed: false });
  const load = useCallback((reset = false) => {
    const current = state.current;
    if (!reset && (current.pending || current.failed || current.offset >= current.total)) return;
    if (reset) { current.generation++; current.offset = 0; }
    const generation = current.generation, offset = current.offset;
    current.pending = true; current.failed = false;
    setLoading(true); setError(null);
    // Dispatched IPC cannot be cancelled; ownership belongs to this generation.
    const task = (async () => {
      if (generation !== state.current.generation) return;
      try {
        const response = await executeQuery({ query, payload: { offset, limit: size } });
        if (generation !== state.current.generation) return;
        const page = response.data as { items: T[]; total: number };
        if (!Array.isArray(page?.items) || !Number.isFinite(page.total)) throw Error("Invalid library response");
        current.offset = offset + page.items.length;
        current.total = page.total;
        setTotal(page.total);
        setItems(previous => [...new Map([...(reset ? [] : previous), ...page.items].map(item => [item.id, item])).values()]);
        if (page.items.length === 0 && current.offset < page.total) throw Error("Library changed. Refresh to continue.");
      } catch (failure) {
        if (generation === state.current.generation) {
          current.failed = true;
          setError(failure instanceof Error ? failure.message : String(failure));
        }
      } finally {
        if (generation === state.current.generation) { current.pending = false; setLoading(false); }
      }
    })();
    return task;
  }, [query, size]);
  const refresh = useCallback(() => load(true), [load]);
  const next = useCallback(() => { void load(); }, [load]);
  const retry = useCallback(() => { state.current.failed = false; void load(state.current.offset === 0); }, [load]);
  const previousScope = useRef<{ identity: string; load: typeof load } | null>(null);
  useLayoutEffect(() => {
    const changed = previousScope.current?.identity !== identity || previousScope.current?.load !== load;
    previousScope.current = { identity, load };
    const interrupted = state.current.pending;
    state.current.generation++; state.current.pending = false; setLoading(false);
    if (changed) {
      state.current.offset = 0; state.current.total = 0;
      setItems([]); setTotal(0);
    }
    // Preserve completed pages across navigation; obsolete IPC no longer owns loading.
    if (state.current.offset === 0) void refresh();
    else if (interrupted) void load();
    return () => { state.current.generation++; };
  }, [identity, navigation, refresh, load]);
  return { items, setItems, total, loading, error, refresh, next, retry };
}
