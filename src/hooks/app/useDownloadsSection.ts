import { useEffect, useMemo, useState } from "react";
import type { DownloadTask, PlaybackState, Track } from "../../types";
import { useLibraryPage } from "../useLibraryPage";
import { useLibraryPlayback, type ListScope } from "./useLibrary";

interface Options {
  playbackState: PlaybackState;
  fetchPlaybackState: () => Promise<void>;
  beginPlaybackLoading: (trackId: string) => number;
  endPlaybackLoading: (requestId: number) => void;
  downloads: DownloadTask[];
  userId: string;
  navigation: string;
}

/**
 * The two song sections and their list playback: Local Library (your folders)
 * and Kaze Downloads (Kaze's own folder). Only gathers existing hooks, so the
 * caller gains no state beyond which section started the playing list.
 */
export function useDownloadsSection({ playbackState, fetchPlaybackState, beginPlaybackLoading, endPlaybackLoading, downloads, userId, navigation }: Options) {
  // Both sections use the "archive" source; this records which one started it.
  const [activeListScope, setActiveListScope] = useState<ListScope | null>(null);
  const shared = { playbackState, fetchPlaybackState, beginPlaybackLoading, endPlaybackLoading, activeListScope, setActiveListScope };
  const localPlayback = useLibraryPlayback({ ...shared, scope: "local" });
  const downloadsPlayback = useLibraryPlayback({ ...shared, scope: "downloads" });

  // Songs Kaze downloaded, refreshed whenever a download finishes, is removed
  // or a file in the folder changes.
  const downloadsPage = useLibraryPage<Track>("GetLibraryTracksPage", { size: 200, identity: userId, navigation, scope: "downloads" });
  const savedDownloadsKey = useMemo(
    () => downloads.filter((task) => task.status === "COMPLETED").map((task) => `${task.id}:${task.destination_path}`).join("|"),
    [downloads],
  );
  const refreshDownloadsPage = downloadsPage.refresh;
  useEffect(() => { void refreshDownloadsPage(); }, [savedDownloadsKey, refreshDownloadsPage]);

  const downloadsListState: "idle" | "playing" | "paused" = !downloadsPlayback.downloadsListActive ? "idle"
    : playbackState.is_playing ? "playing" : "paused";

  return { localPlayback, downloadsPlayback, downloadsPage, downloadsListState };
}
