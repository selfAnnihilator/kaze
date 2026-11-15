import { useEffect, useMemo, useState } from "react";
import type { DownloadTask, PlaybackState, Track } from "../../types";
import { useLibraryPage } from "../useLibraryPage";
import { useScopedTracks } from "../useScopedTracks";
import { useLibraryPlayback, type ListScope } from "./useLibrary";
import type { PlayingOrigin } from "../../playingOrigin";

interface Options {
  playbackState: PlaybackState;
  fetchPlaybackState: () => Promise<void>;
  beginPlaybackLoading: (trackId: string) => number;
  endPlaybackLoading: (requestId: number) => void;
  downloads: DownloadTask[];
  userId: string;
  navigation: string;
  /** The library search typed in the section (shared with Local). */
  query: string;
  /** Kaze's download folder. */
  downloadsDir?: string;
  /** Records the section as the playing song's origin when its list starts. */
  setPlayingOrigin?: (origin: PlayingOrigin) => void;
}

/**
 * The two song sections and their list playback: Local Library (your folders)
 * and Kaze Downloads (Kaze's own folder). Only gathers existing hooks, so the
 * caller gains no state beyond which section started the playing list.
 */
export function useDownloadsSection({ playbackState, fetchPlaybackState, beginPlaybackLoading, endPlaybackLoading, downloads, userId, navigation, query, downloadsDir, setPlayingOrigin }: Options) {
  // Both sections use the "archive" source; this records which one started it.
  const [activeListScope, setActiveListScope] = useState<ListScope | null>(null);
  const shared = { playbackState, fetchPlaybackState, beginPlaybackLoading, endPlaybackLoading, activeListScope, setActiveListScope, setPlayingOrigin };
  const localPlayback = useLibraryPlayback({ ...shared, scope: "local" });
  const downloadsPlayback = useLibraryPlayback({ ...shared, scope: "downloads" });

  // Songs Kaze downloaded, refreshed whenever a download finishes, is removed
  // or a file in the folder changes.
  const fullPage = useLibraryPage<Track>("GetLibraryTracksPage", { size: 200, identity: userId, navigation, scope: "downloads" });
  // Searching works like Local: matching songs from Kaze's folder only.
  const searching = navigation === "downloads" && !!query;
  const found = useScopedTracks({ account: userId, query, active: searching, downloadsDir, section: "downloads" });
  const savedDownloadsKey = useMemo(
    () => downloads.filter((task) => task.status === "COMPLETED").map((task) => `${task.id}:${task.destination_path}`).join("|"),
    [downloads],
  );
  const refreshDownloadsPage = fullPage.refresh;
  const refreshFound = found.refresh;
  useEffect(() => { void refreshDownloadsPage(); void refreshFound(); }, [savedDownloadsKey, refreshDownloadsPage, refreshFound]);
  const downloadsPage = useMemo(() => searching
    ? { ...fullPage, items: found.rows, setItems: found.setItems, total: found.rows.length, loading: found.loading, error: found.error, refresh: found.refresh, next: () => {}, retry: () => void found.refresh() }
    : fullPage, [searching, fullPage, found.rows, found.setItems, found.loading, found.error, found.refresh]);

  const downloadsListState: "idle" | "playing" | "paused" = !downloadsPlayback.downloadsListActive ? "idle"
    : playbackState.is_playing ? "playing" : "paused";

  return { localPlayback, downloadsPlayback, downloadsPage, downloadsListState };
}
