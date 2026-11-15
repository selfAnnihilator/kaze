import { useCallback, useMemo, useRef, useState } from "react";
import { dispatchCommand, executeQuery } from "../../services/api";
import type { Album, AppSettings, PlaybackState, Track } from "../../types";
import { useLibraryPage } from "../useLibraryPage";
import { useScopedTracks } from "../useScopedTracks";
import type { NavigationDestination } from "../../navigationHistory";
import type { PlayingOrigin } from "../../playingOrigin";

// Downloads section shuffle toggle, persisted per device ("1" on, "0" off).
const DOWNLOADS_SHUFFLE_KEY = "kaze.downloads.shuffle";

/**
 * Library data: paged tracks/albums, the scoped (album/query) track view,
 * settings, scan flag and library folder management.
 */
export function useLibrary({ userId, destination, initialSettings, fetchOnboardingStatus }: {
  userId: string;
  destination: NavigationDestination;
  initialSettings: AppSettings;
  fetchOnboardingStatus: () => Promise<void>;
}) {
  const [settings, setSettings] = useState<AppSettings>(initialSettings);
  // The Library section is your own folders; Kaze's downloads have their own section.
  const trackPage = useLibraryPage<Track>("GetLibraryTracksPage", { size: 200, identity: userId, navigation: `${destination.view}:${destination.libraryAlbumId || ""}:${destination.libraryQuery || ""}`, scope: "local" });
  const albumPage = useLibraryPage<Album>("GetLibraryAlbumsPage", { size: 100, identity: userId, navigation: destination.view, scope: "local" });
  const scoped = useScopedTracks({ account: userId, album: destination.libraryAlbumId, query: destination.libraryQuery, active: destination.view === "library" && !destination.collection && !destination.lyrics, downloadsDir: settings.downloads?.resolved_dir });
  const scopedRefreshRef = useRef(scoped.refresh);
  scopedRefreshRef.current = scoped.refresh;
  const libraryScoped = Boolean(destination.libraryAlbumId || destination.libraryQuery);
  const tracks = libraryScoped ? scoped.rows : trackPage.items;
  const setTracks = useCallback((update: React.SetStateAction<Track[]>) => {
    trackPage.setItems(update);
    scoped.setItems(update);
  }, [trackPage.setItems]);
  const tracksRef = useRef<Track[]>([]);
  tracksRef.current = tracks;
  const albums = albumPage.items;
  const [isScanning, setIsScanning] = useState(false);

  const fetchTracks = useCallback(async () => {
    void trackPage.refresh();
    await scopedRefreshRef.current();
  }, []);

  const fetchAlbums = albumPage.refresh;

  const fetchSettings = useCallback(async () => {
    try {
      const res = await executeQuery({ query: "GetSettings" });
      if (res.data) {
        setSettings(res.data as AppSettings);
      }
    } catch (err) {
      console.error("Failed to fetch settings:", err);
    }
  }, []);

  const handleAddFolder = useCallback(async (path: string) => {
    await dispatchCommand({
      command: "AddLibraryFolder",
      payload: { path },
    });
    fetchOnboardingStatus();
  }, [fetchOnboardingStatus]);

  const handleRemoveFolder = useCallback(async (folderId: string) => {
    await dispatchCommand({
      command: "RemoveLibraryFolder",
      payload: { folder_id: folderId },
    });
    fetchOnboardingStatus();
  }, [fetchOnboardingStatus]);

  const handleRescanLibrary = useCallback(async () => {
    setIsScanning(true);
    try {
      await dispatchCommand({
        command: "ScanLibrary",
        payload: { incremental: false },
      });
    } catch (error) {
      setIsScanning(false);
      throw error;
    }
  }, []);

  return useMemo(() => ({
    trackPage, albumPage, scoped, libraryScoped, tracks, setTracks, tracksRef, albums,
    settings, setSettings, isScanning, setIsScanning,
    fetchTracks, fetchAlbums, fetchSettings, handleAddFolder, handleRemoveFolder, handleRescanLibrary,
  }), [trackPage, albumPage, scoped, libraryScoped, tracks, setTracks, albums, settings, isScanning,
    fetchTracks, fetchAlbums, fetchSettings, handleAddFolder, handleRemoveFolder, handleRescanLibrary]);
}

/** Downloads view list playback: play all / row play / persisted shuffle toggle. */
export type ListScope = "local" | "downloads";

/** Ordered list playback for one section: Local Library or Kaze Downloads. */
export function useLibraryPlayback({ playbackState, fetchPlaybackState, beginPlaybackLoading, endPlaybackLoading, scope = "local", activeListScope, setActiveListScope, setPlayingOrigin }: {
  playbackState: PlaybackState;
  fetchPlaybackState: () => Promise<void>;
  beginPlaybackLoading: (trackId: string) => number;
  endPlaybackLoading: (requestId: number) => void;
  scope?: ListScope;
  /** Which section started the list currently playing (both use the "archive" source). */
  activeListScope: ListScope | null;
  setActiveListScope: (scope: ListScope) => void;
  /** Records this list as the playing song's origin, so the player can link back to it. */
  setPlayingOrigin?: (origin: PlayingOrigin) => void;
}) {
  // The Local list keeps its original storage key so the saved toggle survives.
  const shuffleKey = scope === "local" ? DOWNLOADS_SHUFFLE_KEY : "kaze.kazeDownloads.shuffle";
  // Downloads view: ordered list playback (plays in order, stops at the end).
  // The shuffle toggle belongs to this section only: it is never read back from
  // the queue, so other sections (playlists) cannot flip it. Persisted across restarts.
  const [libraryShuffle, setLibraryShuffle] = useState<boolean>(() => {
    try {
      return localStorage.getItem(shuffleKey) === "1";
    } catch {
      return false;
    }
  });
  const downloadsListActive =
    playbackState.source === "archive" && (playbackState.is_playing || Boolean(playbackState.is_paused)) &&
    (activeListScope ?? "local") === scope;
  const startLibraryList = useCallback(
    async (startId?: string) => {
      const requestId = beginPlaybackLoading(startId ?? "library-list");
      setActiveListScope(scope);
      setPlayingOrigin?.({ type: "library", scope, name: scope === "downloads" ? "Downloads" : "Local" });
      try {
        await dispatchCommand({
          command: "PlayTrackList",
          payload: {
            // Search only filters the view; playback covers the whole Local Library list.
            track_ids: null,
            start_track_id: startId ?? null,
            shuffle: libraryShuffle,
            scope,
          },
        });
        await fetchPlaybackState();
      } catch (err) {
        await fetchPlaybackState();
        throw err;
      } finally {
        endPlaybackLoading(requestId);
      }
    },
    [libraryShuffle, fetchPlaybackState, beginPlaybackLoading, endPlaybackLoading, scope, setActiveListScope, setPlayingOrigin],
  );
  const handleLibraryRowPlay = useCallback(
    async (trackId: string) => {
      if (playbackState.current_track?.id === trackId && (playbackState.is_playing || playbackState.is_paused)) {
        await dispatchCommand({ command: playbackState.is_playing ? "Pause" : "Resume" });
        await fetchPlaybackState();
        return;
      }
      await startLibraryList(trackId);
    },
    [playbackState.current_track?.id, playbackState.is_playing, playbackState.is_paused, fetchPlaybackState, startLibraryList],
  );
  // Play all doubles as Pause/Resume while the Downloads list is the active source.
  const handleLibraryPlayAll = useCallback(async () => {
    if (downloadsListActive) {
      await dispatchCommand({ command: playbackState.is_playing ? "Pause" : "Resume" });
      await fetchPlaybackState();
      return;
    }
    await startLibraryList();
  }, [downloadsListActive, playbackState.is_playing, fetchPlaybackState, startLibraryList]);
  const handleLibraryToggleShuffle = useCallback(async () => {
    const next = !libraryShuffle;
    setLibraryShuffle(next);
    try {
      localStorage.setItem(shuffleKey, next ? "1" : "0");
    } catch {
      // Storage unavailable: toggle still applies for this session.
    }
    // Takes effect after the current song: only the upcoming Downloads tracks are reordered.
    if (downloadsListActive) {
      await dispatchCommand({ command: "SetArchiveShuffle", payload: { enabled: next } });
    }
  }, [libraryShuffle, downloadsListActive, shuffleKey]);

  return useMemo(() => ({
    libraryShuffle, downloadsListActive, handleLibraryRowPlay, handleLibraryPlayAll, handleLibraryToggleShuffle,
  }), [libraryShuffle, downloadsListActive, handleLibraryRowPlay, handleLibraryPlayAll, handleLibraryToggleShuffle]);
}
