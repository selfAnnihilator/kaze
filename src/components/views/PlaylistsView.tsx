import { useInterfaceStyle } from "../../presentation/InterfaceContext";
import React, { useState } from "react";
import { Playlist, Track, JioSaavnPlaylistImport, UserProfile } from "../../types";
import { CollectionData } from "./CollectionDetailView";
import type { PlaylistsViewContext } from "./playlists/context";
import { PlaylistsHeader } from "./playlists/PlaylistsHeader";
import { SmartMixesSection } from "./playlists/SmartMixesSection";
import { CustomPlaylistsSection } from "./playlists/CustomPlaylistsSection";
import { CreatePlaylistModal } from "./playlists/CreatePlaylistModal";
import { PlaylistDetailsModal } from "./playlists/PlaylistDetailsModal";
import { JioSaavnImportModal } from "./playlists/JioSaavnImportModal";
import { RenamePlaylistModal } from "./playlists/RenamePlaylistModal";
import { DeletePlaylistModal } from "./playlists/DeletePlaylistModal";

interface PlaylistsViewProps {
  viewMode?: "playlists" | "smart_mixes" | "all";
  playlists: Playlist[];
  activePlaylistId?: string | null;
  onPlayPlaylist: (playlistId: string) => void;
  onCreatePlaylist: (name: string, description?: string) => Promise<string | null>;
  onRefreshPlaylists?: () => Promise<void>;
  onInspectJioSaavnPlaylist: (urlOrId: string,
  ) => Promise<JioSaavnPlaylistImport | null>;
  onSaveImportedPlaylist: (
    name: string,
    trackIds: string[],
    tracksInfo?: Array<{
      id: string;
      title?: string;
      artist?: string;
      album?: string;
      duration_secs?: number;
      cover_art_url?: string;
      preview_url?: string;
    }>,
  ) => Promise<void>;
  onAddMissingToWishlist: (tracks: any[]) => Promise<void>;
  onSearchDirect?: (artist: string, title: string, album?: string) => void;
  onFetchPlaylistTracks: (playlistId: string) => Promise<Track[]>;
  onPlayTrack: (trackId: string) => void;
  loadingTrackId?: string | null;
  queuedTrackIds?: Set<string>;
  onEnqueueTrack: (trackId: string) => void;
  onOpenCollection?: (collection: CollectionData) => void;
  onRenamePlaylist: (playlistId: string, name: string) => Promise<void>;
  onDeletePlaylist: (playlistId: string) => Promise<void>;
  currentUser?: UserProfile | null;
  onOpenAuthModal?: () => void;
}

export const PlaylistsView: React.FC<PlaylistsViewProps> = ({
  viewMode = "all",
  playlists,
  activePlaylistId: _activePlaylistId,
  onPlayPlaylist,
  onCreatePlaylist,
  onRefreshPlaylists,
  onInspectJioSaavnPlaylist,
  onSaveImportedPlaylist,
  onAddMissingToWishlist: _onAddMissingToWishlist,
  onSearchDirect,
  onFetchPlaylistTracks,
  onPlayTrack,
  loadingTrackId,
  queuedTrackIds,
  onEnqueueTrack,
  onOpenCollection,
  onRenamePlaylist,
  onDeletePlaylist,
  currentUser,
  onOpenAuthModal,
}) => {
  const interfaceStyle = useInterfaceStyle();
  // Modal states
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [playlistName, setPlaylistName] = useState("");
  const [refreshingPlaylists, setRefreshingPlaylists] = useState(false);
  const [refreshError, setRefreshError] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const creatingRef = React.useRef(false);
  const [createError, setCreateError] = useState<string | null>(null);

  // Detail Modal states
  const [selectedPlaylist, setSelectedPlaylist] = useState<Playlist | null>(null,
  );
  const [playlistTracks, setPlaylistTracks] = useState<Track[]>([]);
  const [loadingTracks, setLoadingTracks] = useState(false);

  // JioSaavn Modal states
  const [showJioSaavnModal, setShowJioSaavnModal] = useState(false);

  // Custom themed dialog states for Delete and Rename
  const [playlistToDelete, setPlaylistToDelete] = useState<Playlist | null>(null,
  );
  const [isDeleting, setIsDeleting] = useState(false);

  const [playlistToRename, setPlaylistToRename] = useState<Playlist | null>(null,
  );
  const [renameValue, setRenameValue] = useState("");
  const [renameError, setRenameError] = useState<string | null>(null);
  const [isRenaming, setIsRenaming] = useState(false);

  // Permanent saving state for mixes
  const [savingMixId, setSavingMixId] = useState<string | null>(null);
  const [savedMixIds, setSavedMixIds] = useState<Set<string>>(new Set());

  // Split into smart mixes and custom playlists (deduplicated by name)
  const seenMixNames = new Set<string>();
  const smartMixes = playlists.filter((p) => {
    if (p.is_smart_mix !== 1) return false;
    const key = p.name.trim().toLowerCase();
    if (seenMixNames.has(key)) return false;
    seenMixNames.add(key);
    return true;
  });
  const customPlaylists = currentUser
    ? playlists.filter((p) => p.is_smart_mix !== 1 && p.name !== "Liked Songs")
    : [];

  const handleSaveMixToUserPlaylist = async (mix: Playlist) => {
    try {
      let targetName = mix.name.trim();
      while (customPlaylists.some((p) => p.name === targetName)) {
        const prompted = window.prompt(
          `A playlist named "${targetName}" already exists.\nPlease enter a new name for this playlist:`,
          `${targetName} (1)`,
        );
        if (prompted === null) {
          return;
        }
        const trimmed = prompted.trim();
        if (!trimmed) {
          alert("Playlist name cannot be empty.");
          continue;
        }
        targetName = trimmed;
      }

      setSavingMixId(mix.id);
      const tracks = await onFetchPlaylistTracks(mix.id);
      const trackIds = tracks.map((t) => t.id);
      await onSaveImportedPlaylist(targetName, trackIds);
      setSavedMixIds((prev) => new Set(prev).add(mix.id));
    } catch (err) {
      console.error("Failed to save mix as playlist:", err);
    } finally {
      setSavingMixId(null);
    }
  };

  const handleCreate = async (e: React.FormEvent) => {
    e.preventDefault();
    if (creatingRef.current) return;
    const trimmed = playlistName.trim();
    if (!trimmed) return;
    if (customPlaylists.some((p) => p.name === trimmed)) {
      alert(`A playlist named "${trimmed}" already exists.`);
      return;
    }
    creatingRef.current = true; setCreating(true); setCreateError(null);
    try {
      const id = await onCreatePlaylist(trimmed);
      if (id) { setPlaylistName(""); setShowCreateModal(false); }
    } catch (failure) {
      setCreateError(failure instanceof Error ? failure.message : String(failure));
    } finally { creatingRef.current = false; setCreating(false); }
  };

  const handleOpenPlaylistDetails = async (pl: Playlist) => {
    if (onOpenCollection) {
      onOpenCollection({
        id: pl.id,
        type: pl.is_smart_mix === 1 ? "mix" : "playlist",
        title: pl.name,
        subtitle:
          pl.description ||
          (pl.is_smart_mix === 1
            ? "Custom algorithmic smart mix"
            : "Created playlist"),
        tag: pl.is_smart_mix === 1 ? "SMART MIX" : "PUBLIC PLAYLIST",
        playlistId: pl.id,
        cover_art_url: pl.cover_art_url,
      });
      return;
    }
    setSelectedPlaylist(pl);
    setLoadingTracks(true);
    try {
      const tracks = await onFetchPlaylistTracks(pl.id);
      setPlaylistTracks(tracks);
    } finally {
      setLoadingTracks(false);
    }
  };

  const handleOpenRenameModal = (pl: Playlist) => {
    if (pl.name === "Liked Songs") return;
    setPlaylistToRename(pl);
    setRenameValue(pl.name);
    setRenameError(null);
  };

  const handleConfirmRename = async (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    if (!playlistToRename) return;
    const nextName = renameValue.trim();
    if (!nextName) {
      setRenameError("Playlist name cannot be empty.");
      return;
    }
    if (nextName === playlistToRename.name) {
      setPlaylistToRename(null);
      return;
    }
    if (
      customPlaylists.some(
        (candidate) =>
          candidate.id !== playlistToRename.id &&
          candidate.name.toLowerCase() === nextName.toLowerCase(),
      )
    ) {
      setRenameError(`A playlist named "${nextName}" already exists.`);
      return;
    }
    setIsRenaming(true);
    try {
      await onRenamePlaylist(playlistToRename.id, nextName);
      setPlaylistToRename(null);
    } catch (err: any) {
      setRenameError(err?.message || "Could not rename the playlist.");
    } finally {
      setIsRenaming(false);
    }
  };

  const handleOpenDeleteModal = (pl: Playlist) => {
    if (pl.name === "Liked Songs") return;
    setPlaylistToDelete(pl);
  };

  const handleConfirmDelete = async () => {
    if (!playlistToDelete) return;
    setIsDeleting(true);
    try {
      await onDeletePlaylist(playlistToDelete.id);
      setPlaylistToDelete(null);
    } catch (err: any) {
      alert(err?.message || "Could not delete the playlist.");
    } finally {
      setIsDeleting(false);
    }
  };

  const formatSeconds = (secs?: number) => {
    if (!secs || isNaN(secs)) return "0:00";
    const mins = Math.floor(secs / 60);
    const remainingSecs = Math.floor(secs % 60);
    return `${mins}:${remainingSecs < 10 ? "0" : ""}${remainingSecs}`;
  };

  const c: PlaylistsViewContext = {
    viewMode, onRefreshPlaylists, refreshingPlaylists, setRefreshingPlaylists, setRefreshError,
    setShowJioSaavnModal, setShowCreateModal, smartMixes, savedMixIds, savingMixId,
    handleOpenPlaylistDetails, onPlayPlaylist, handleSaveMixToUserPlaylist, customPlaylists,
    currentUser, onOpenAuthModal, interfaceStyle, handleOpenRenameModal, handleOpenDeleteModal,
    showCreateModal, creating, handleCreate, createError, playlistName, setPlaylistName,
    selectedPlaylist, setSelectedPlaylist, playlistTracks, loadingTracks, onPlayTrack, loadingTrackId,
    formatSeconds, queuedTrackIds, onEnqueueTrack, showJioSaavnModal, onInspectJioSaavnPlaylist,
    onSaveImportedPlaylist, onSearchDirect, playlistToRename, isRenaming, setPlaylistToRename,
    handleConfirmRename, renameValue, setRenameValue, setRenameError, renameError, playlistToDelete,
    isDeleting, setPlaylistToDelete, handleConfirmDelete,
  };
  return (
    <div className="playlists-view">
      {/* Header */}
      <PlaylistsHeader c={c} />

      {refreshError && <p role="alert">{refreshError}</p>}
      {/* SMART MIXES VIEW */}
      <SmartMixesSection c={c} />

      {/* CUSTOM PLAYLISTS VIEW */}
      <CustomPlaylistsSection c={c} />

      {/* MODAL 1: CREATE PLAYLIST */}
      <CreatePlaylistModal c={c} />

      {/* MODAL 2: PLAYLIST DETAILS / TRACK LIST */}
      <PlaylistDetailsModal c={c} />

      {/* MODAL 3: JIOSAAVN PLAYLIST IMPORT */}
      <JioSaavnImportModal c={c} />

      {/* MODAL 4: THEMED RENAME PLAYLIST MODAL */}
      <RenamePlaylistModal c={c} />

      {/* MODAL 5: THEMED DELETE CONFIRMATION MODAL */}
      <DeletePlaylistModal c={c} />
    </div>
  );
};
