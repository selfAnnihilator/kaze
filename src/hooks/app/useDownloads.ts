import { useCallback, useMemo, useRef, useState } from "react";
import type { MutableRefObject } from "react";
import { dispatchCommand, executeQuery } from "../../services/api";
import { downloadProgress } from "../../services/downloadProgress";
import { filterAndRankDownloadResults } from "../../downloadMatch";
import type { DownloadSearchResult, DownloadTask } from "../../types";

/** Downloads: account-scoped task list (seeding the progress store), search, start, cancel. */
export function useDownloads({ libraryAccountRef }: { libraryAccountRef: MutableRefObject<string> }) {
  const [downloads, setDownloads] = useState<DownloadTask[]>([]);

  const downloadsRequest = useRef(0);
  const fetchDownloads = useCallback(async () => {
    const account = libraryAccountRef.current, request = ++downloadsRequest.current;
    try {
      const res = await executeQuery({
        query: "GetDownloads",
        payload: { limit: 10000 },
      });
      if (account === libraryAccountRef.current && request === downloadsRequest.current && Array.isArray(res.data)) {
        downloadProgress.replace(res.data);
        setDownloads(res.data);
      }
    } catch (err) {
      console.error("Failed to fetch downloads:", err);
    }
  }, []);

  const handleSearchDownloads = useCallback(async (
    artist: string,
    title: string,
    album?: string,
  ): Promise<DownloadSearchResult[]> => {
    const res = await dispatchCommand({
      command: "SearchDownloads",
      payload: { artist, title, album },
    });
    const results = (res as any)?.data || (res as any)?.results;
    if (Array.isArray(results)) {
      return filterAndRankDownloadResults(results, artist, title);
    }
    return [];
  }, []);

  const handleStartDownload = useCallback(async (
    search_result_id: string,
    wishlist_id?: string,
  ) => {
    await dispatchCommand({
      command: "StartDownload",
      payload: { search_result_id, wishlist_id },
    });
    await fetchDownloads();
  }, [fetchDownloads]);

  const handleCancelDownload = useCallback(async (task_id: string) => {
    await dispatchCommand({ command: "CancelDownload", payload: { task_id } });
    await fetchDownloads();
  }, [fetchDownloads]);

  return useMemo(() => ({
    downloads, setDownloads, fetchDownloads, handleSearchDownloads, handleStartDownload, handleCancelDownload,
  }), [downloads, setDownloads, fetchDownloads, handleSearchDownloads, handleStartDownload, handleCancelDownload]);
}
