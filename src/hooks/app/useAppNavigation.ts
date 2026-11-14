import { useCallback, useState } from "react";
import { viewDestination, type NavigationDestination } from "../../navigationHistory";
import type { ViewType } from "../../viewTypes";
import type { Album, Playlist } from "../../types";
import type { CollectionData } from "../../components/views/CollectionDetailView";

const LIBRARY_VIEWS: ViewType[] = ["library", "downloads", "albums", "playlists"];

export const isLibraryView = (view: ViewType) => LIBRARY_VIEWS.includes(view);

/** The account's own Liked Songs playlist, if any. */
export const findLikedSongs = (playlists: Playlist[]) =>
  playlists.find((playlist) => playlist.name === "Liked Songs" && !playlist.is_smart_mix);

function isLikedSongs(collection: CollectionData, likedSongs: Playlist | undefined) {
  if (collection.title === "Liked Songs") return true;
  return !!likedSongs && (collection.playlistId === likedSongs.id || collection.id === likedSongs.id);
}

/** Whether the main area shows the library ("archive") surface, and if Liked Songs is open in it. */
export function archiveSurface({ brutalist, activeCollection, likedSongs, currentView, isLyricsActive }: {
  brutalist: boolean;
  activeCollection: CollectionData | null;
  likedSongs: Playlist | undefined;
  currentView: ViewType;
  isLyricsActive: boolean;
}) {
  const isArchiveLiked = brutalist && activeCollection?.type === "playlist" && isLikedSongs(activeCollection, likedSongs);
  const isArchiveSurface = !isLyricsActive && (isArchiveLiked || (!activeCollection && isLibraryView(currentView)));
  return { isArchiveLiked, isArchiveSurface };
}

/** Sidebar, deck and home navigation callbacks, all stable for memoized children. */
export function useAppNavigation({ navigate, updateDestination, handleOpenCollection, handleToggleFullscreen, albums, libraryMoment }: {
  navigate: (destination: NavigationDestination) => void;
  updateDestination: (update: (current: NavigationDestination) => NavigationDestination) => void;
  handleOpenCollection: (collection: CollectionData) => void;
  handleToggleFullscreen: (value?: boolean) => unknown;
  albums: Album[];
  libraryMoment?: { album_id?: string | null; album_title?: string | null } | null;
}) {
  const [statsRefreshTrigger, setStatsRefreshTrigger] = useState(0);
  const handleSelectView = useCallback((view: ViewType) => {
    if (view === "stats") setStatsRefreshTrigger((prev) => prev + 1);
    navigate(viewDestination(view));
  }, [navigate]);
  const handleSelectSidebarPlaylist = useCallback((playlist: Playlist) => {
    const smart = playlist.is_smart_mix === 1;
    handleOpenCollection({
      id: playlist.id,
      type: smart ? "mix" : "playlist",
      title: playlist.name,
      subtitle: playlist.description || (smart ? "Custom algorithmic smart mix" : "User Playlist"),
      tag: smart ? "SMART MIX" : "PUBLIC PLAYLIST",
      playlistId: playlist.id,
      cover_art_url: playlist.cover_art_url,
    });
  }, [handleOpenCollection]);
  const handleOpenQueue = useCallback(() => {
    handleToggleFullscreen(false);
    navigate(viewDestination("queue"));
  }, [handleToggleFullscreen, navigate]);
  const handleOpenNowPlaying = useCallback(() => navigate(viewDestination("now_playing")), [navigate]);
  const handleHomeDiscover = useCallback(() => handleSelectView("discovery"), [handleSelectView]);

  const handleSearchLibrary = useCallback((queryText: string) => {
    updateDestination((current) => ({ ...current, libraryQuery: queryText.trim(), libraryAlbumId: undefined }));
  }, [updateDestination]);
  const momentAlbumId = libraryMoment?.album_id;
  const momentAlbumTitle = libraryMoment?.album_title;
  const handleSelectAlbum = useCallback((albumId: string) => {
    const album = albums.find((al) => al.id === albumId);
    navigate({
      ...viewDestination("library"),
      libraryQuery: album?.title || (momentAlbumId === albumId ? momentAlbumTitle ?? "" : ""),
      libraryAlbumId: albumId,
    });
  }, [albums, momentAlbumId, momentAlbumTitle, navigate]);

  return {
    statsRefreshTrigger, handleSelectView, handleSelectSidebarPlaylist, handleOpenQueue, handleOpenNowPlaying,
    handleHomeDiscover, handleSearchLibrary, handleSelectAlbum,
  };
}
