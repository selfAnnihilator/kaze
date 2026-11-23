import { useCallback, useEffect, useState } from "react";
import { dispatchCommand, executeQuery } from "../../../services/api";
export { formatBytes } from "../format";
import type { EntityFeedbackRow, RemoteAudioCacheStats } from "../../../types";

export type MetadataKey = "enable_jiosaavn" | "auto_fill_metadata" | "enable_lastfm_similarity";
type Prefs = Record<MetadataKey, boolean>;

/** Online music switches and the gap-filling actions (same commands as the PC settings). */
const readPrefs = (metadata?: Partial<Prefs>): Prefs => ({
  enable_jiosaavn: metadata?.enable_jiosaavn !== false,
  auto_fill_metadata: metadata?.auto_fill_metadata !== false,
  enable_lastfm_similarity: metadata?.enable_lastfm_similarity !== false,
});

type Note = { error: boolean; text: string } | null;

/** Runs one settings action at a time and keeps its outcome line. */
function useAction() {
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<Note>(null);
  const run = useCallback(async (work: () => Promise<unknown>, done: string, failed: string) => {
    setBusy(true);
    setNote(null);
    try {
      await work();
      setNote(done ? { error: false, text: done } : null);
    } catch {
      setNote({ error: true, text: failed });
    } finally {
      setBusy(false);
    }
  }, []);
  return { busy, note, setNote, run };
}

/** Online music switches and the gap-filling actions (same commands as the PC settings). */
export function useMetadataSettings() {
  const [prefs, setPrefs] = useState<Prefs | null>(null);
  const { busy, note, setNote, run } = useAction();

  useEffect(() => {
    let cancelled = false;
    executeQuery({ query: "GetSettings" })
      .then((result) => { if (!cancelled) setPrefs(readPrefs(result.data?.metadata)); })
      .catch(() => { if (!cancelled) setNote({ error: true, text: "Couldn’t load these settings. Open Settings again to retry." }); });
    return () => { cancelled = true; };
  }, [setNote]);

  const toggle = (key: MetadataKey) => {
    if (!prefs) return;
    const enabled = !prefs[key];
    void run(async () => {
      await dispatchCommand({ command: "SetMetadataPreference", payload: { key, enabled } });
      setPrefs((current) => (current ? { ...current, [key]: enabled } : current));
    }, "", "Couldn’t save this choice. Try again.");
  };
  const fill = (retry: boolean) => void run(
    () => dispatchCommand({ command: "FillLibraryGaps", payload: { retry } }),
    "Filling missing artwork and details in the background.",
    "Couldn’t start filling. Try again.",
  );
  const removeFilled = () => void run(async () => {
    await dispatchCommand({ command: "RemoveProviderEnrichment" });
    setPrefs((current) => (current ? { ...current, auto_fill_metadata: false } : current));
  }, "Filled data removed. Your original tags are back and automatic filling is off.", "Couldn’t remove filled data. Try again.");

  return { prefs, busy, note, toggle, fill, removeFilled };
}

/** Artists and genres hidden or turned down from recommendations. */
export function useHiddenEntities() {
  const [rows, setRows] = useState<EntityFeedbackRow[] | null>(null);
  const [error, setError] = useState(false);
  useEffect(() => {
    let cancelled = false;
    executeQuery({ query: "GetEntityFeedback" })
      .then((result) => { if (!cancelled) setRows(Array.isArray(result.data) ? result.data : []); })
      .catch(() => { if (!cancelled) setError(true); });
    return () => { cancelled = true; };
  }, []);
  const remove = async (row: EntityFeedbackRow) => {
    try {
      await dispatchCommand({ command: "ClearEntityFeedback", payload: { kind: row.kind, key: row.key } });
      setRows((current) => current?.filter((r) => r.kind !== row.kind || r.key !== row.key) ?? null);
    } catch {
      setError(true);
    }
  };
  return { rows, error, remove };
}

/** Temporary audio usage (Figma F8) and clearing it. */
export function useAudioCache() {
  const [cache, setCache] = useState<RemoteAudioCacheStats | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);
  const load = useCallback(() => {
    executeQuery({ query: "GetRemoteAudioCacheStats" }).then((response) => {
      const raw = response as unknown as { data?: RemoteAudioCacheStats } & RemoteAudioCacheStats;
      const value = raw?.data ?? raw;
      if (typeof value?.total_size_bytes !== "number") throw new Error("unavailable");
      setCache(value);
      setError(false);
    }).catch(() => setError(true));
  }, []);
  useEffect(load, [load]);
  const clear = async () => {
    setBusy(true);
    try {
      await dispatchCommand({ command: "ClearRemoteAudioCache" });
      load();
    } catch {
      setError(true);
    } finally {
      setBusy(false);
    }
  };
  return { cache, busy, error, clear };
}
