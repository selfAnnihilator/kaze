import { useEffect, useState } from "react";
import { Trash2 } from "lucide-react";
import { dispatchCommand, executeQuery } from "../../../services/api";
import type { RemoteAudioCacheStats } from "../../../types";

/** Accepts the stats either bare or wrapped in a query response. */
function cacheSnapshot(response: unknown): RemoteAudioCacheStats {
  const raw = response as {
    data?: RemoteAudioCacheStats;
    total_size_bytes?: number;
  } | null;
  const value = (raw?.data || raw) as RemoteAudioCacheStats | null;
  if (
    !value ||
    typeof value.total_size_bytes !== "number" ||
    typeof value.file_count !== "number"
  )
    throw new Error("Cache information unavailable");
  return value;
}

const size = (bytes: number) =>
  bytes >= 1024 ** 3
    ? `${(bytes / 1024 ** 3).toFixed(1)} GB`
    : `${(bytes / 1024 ** 2).toFixed(1)} MB`;

/** Settings → System & Storage: temporary audio usage and clearing it. */
export function StorageSettings() {
  const [cache, setCache] = useState<RemoteAudioCacheStats | null>(null);
  const [cacheLoading, setCacheLoading] = useState(false);
  const [cacheError, setCacheError] = useState<string | null>(null);
  const [cacheMessage, setCacheMessage] = useState<string | null>(null);
  const [clearingCache, setClearingCache] = useState(false);
  useEffect(() => {
    let cancelled = false;
    setCacheLoading(true);
    setCache(null);
    setCacheError(null);
    executeQuery({ query: "GetRemoteAudioCacheStats" })
      .then((response) => {
        const value = cacheSnapshot(response);
        if (!cancelled) setCache(value);
      })
      .catch(() => {
        if (!cancelled) setCacheError("Couldn't load storage usage. Reopen this section to try again.");
      })
      .finally(() => {
        if (!cancelled) setCacheLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);
  const clearCache = async () => {
    if (clearingCache) return;
    setClearingCache(true);
    setCacheError(null);
    setCacheMessage(null);
    try {
      await dispatchCommand({ command: "ClearRemoteAudioCache" });
      setCacheMessage("Temporary audio cleared. Tracks in use stay available.");
      try {
        setCache(cacheSnapshot(await executeQuery({ query: "GetRemoteAudioCacheStats" })));
      } catch {
        setCacheError("Cache cleared, but usage couldn't refresh. Reopen this section to check it.");
      }
    } catch {
      setCacheError("Couldn't clear temporary audio. Please try again.");
    } finally {
      setClearingCache(false);
    }
  };
  return (
    <section
      className="settings-detail"
      aria-label="System & Storage settings"
    >
      <div className="settings-storage-summary">
        <span className="settings-eyebrow">TEMPORARY AUDIO</span>
        <strong>
          {cacheLoading
            ? "Loading…"
            : cache
              ? size(cache.total_size_bytes)
              : "Unavailable"}
        </strong>
        {cache && (
          <p>
            {cache.file_count} cached{" "}
            {cache.file_count === 1 ? "track" : "tracks"}
            {cache.max_size_bytes > 0
              ? ` · ${size(cache.max_size_bytes)} limit`
              : ""}
          </p>
        )}
      </div>
      <div className="settings-storage-action">
        <div>
          <h2>Clear temporary audio</h2>
          <p>
            Free up space. Your music files and playlists stay in place.
          </p>
        </div>
        <button
          type="button"
          className="settings-action"
          disabled={clearingCache || cacheLoading}
          onClick={() => void clearCache()}
        >
          <Trash2 size={16} />
          {clearingCache ? "Clearing…" : "Clear Cache"}
        </button>
      </div>
      {cacheMessage && <p role="status">{cacheMessage}</p>}
      {cacheError && <p role="alert">{cacheError}</p>}
    </section>
  );
}
