import React, { useState, useMemo } from "react";
import {
  ListMusic,
  Sparkles,
  Plus,
  Play,
  Share2,
  DownloadCloud,
  Check,
  AlertCircle,
  X,
  MoreVertical,
  Pencil,
  Trash2,
  Heart,
  ListPlus,
  Search,
} from "lucide-react";
import { Playlist, Track, SpotifyPlaylistImport, UserProfile } from "../../types";
import { CollectionData } from "./CollectionDetailView";
import { matchesPlaylistSearch } from "../../playlistSearch";

interface PlaylistsViewProps {
  viewMode?: "playlists" | "smart_mixes" | "all";
  playlists: Playlist[];
  activePlaylistId?: string | null;
  onSelectPlaylist: (playlistId: string) => void;
  onPlayPlaylist: (playlistId: string) => void;
  onCreatePlaylist: (name: string, description?: string) => void;
  onInspectSpotifyPlaylist: (urlOrId: string) => Promise<SpotifyPlaylistImport | null>;
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
    }>
  ) => Promise<void>;
  onAddMissingToWishlist: (tracks: any[]) => Promise<void>;
  onLaunchSoulseek: (query?: string, filter?: string) => Promise<void>;
  onSearchDirect?: (artist: string, title: string, album?: string) => void;
  onFetchPlaylistTracks: (playlistId: string) => Promise<Track[]>;
  onPlayTrack: (trackId: string) => void;
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
  onInspectSpotifyPlaylist,
  onSaveImportedPlaylist,
  onAddMissingToWishlist: _onAddMissingToWishlist,
  onLaunchSoulseek,
  onSearchDirect,
  onFetchPlaylistTracks,
  onPlayTrack,
  queuedTrackIds,
  onEnqueueTrack,
  onOpenCollection,
  onRenamePlaylist,
  onDeletePlaylist,
  currentUser,
  onOpenAuthModal,
}) => {
  // Modal states
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [playlistName, setPlaylistName] = useState("");

  // Detail Modal states
  const [selectedPlaylist, setSelectedPlaylist] = useState<Playlist | null>(null);
  const [playlistTracks, setPlaylistTracks] = useState<Track[]>([]);
  const [loadingTracks, setLoadingTracks] = useState(false);

  // Spotify Modal states
  const [showSpotifyModal, setShowSpotifyModal] = useState(false);
  const [spotifyInput, setSpotifyInput] = useState("");
  const [inspectingSpotify, setInspectingSpotify] = useState(false);
  const [spotifyResult, setSpotifyResult] = useState<SpotifyPlaylistImport | null>(null);
  const [spotifyMessage, setSpotifyMessage] = useState<string | null>(null);
  const [spotifyFilterQuery, setSpotifyFilterQuery] = useState("");

  // Custom themed dialog states for Delete and Rename
  const [playlistToDelete, setPlaylistToDelete] = useState<Playlist | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);

  const [playlistToRename, setPlaylistToRename] = useState<Playlist | null>(null);
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
  const customPlaylists = currentUser ? playlists.filter((p) => p.is_smart_mix !== 1) : [];

  const handleSaveMixToUserPlaylist = async (mix: Playlist) => {
    try {
      let targetName = mix.name.trim();
      while (customPlaylists.some((p) => p.name === targetName)) {
        const prompted = window.prompt(
          `A playlist named "${targetName}" already exists.\nPlease enter a new name for this playlist:`,
          `${targetName} (1)`
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

  const handleCreate = (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = playlistName.trim();
    if (!trimmed) return;
    if (customPlaylists.some((p) => p.name === trimmed)) {
      alert(`A playlist named "${trimmed}" already exists.`);
      return;
    }
    onCreatePlaylist(trimmed);
    setPlaylistName("");
    setShowCreateModal(false);
  };

  const handleOpenPlaylistDetails = async (pl: Playlist) => {
    if (onOpenCollection) {
      onOpenCollection({
        id: pl.id,
        type: pl.is_smart_mix === 1 ? "mix" : "playlist",
        title: pl.name,
        subtitle: pl.description || (pl.is_smart_mix === 1 ? "Custom algorithmic smart mix" : "Created playlist"),
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
    if (customPlaylists.some((candidate) => candidate.id !== playlistToRename.id && candidate.name.toLowerCase() === nextName.toLowerCase())) {
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

  const handleInspectSpotify = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!spotifyInput.trim()) return;
    setInspectingSpotify(true);
    setSpotifyMessage(null);
    try {
      const res = await onInspectSpotifyPlaylist(spotifyInput.trim());
      if (res) {
        setSpotifyResult(res);
      } else {
        setSpotifyMessage("Could not retrieve Spotify playlist. Please verify the URL or ID.");
      }
    } catch (err: any) {
      setSpotifyMessage(err?.message || "Failed to fetch Spotify playlist.");
    } finally {
      setInspectingSpotify(false);
    }
  };

  const handleSaveSpotifyPlaylist = async (importAll: boolean = true) => {
    if (!spotifyResult) return;
    if (!currentUser && onOpenAuthModal) {
      onOpenAuthModal();
      return;
    }

    // Determine tracks to import
    const targetTracks = importAll
      ? spotifyResult.tracks
      : spotifyResult.tracks.filter((t) => t.in_library && t.matched_local_track_id);

    if (targetTracks.length === 0) {
      alert("No tracks to import.");
      return;
    }

    const trackIds = targetTracks.map((t) =>
      t.in_library &&
      t.matched_local_track_id &&
      !t.matched_local_track_id.startsWith("online:") &&
      !t.matched_local_track_id.startsWith("itunes:")
        ? t.matched_local_track_id
        : `online:${t.spotify_id || Math.random().toString(36).substring(2, 9)}`
    );

    const tracksInfo = targetTracks.map((t, idx) => ({
      id: trackIds[idx],
      title: t.title,
      artist: t.artist,
      duration_secs: t.duration_secs,
      cover_art_url: spotifyResult.cover_url,
    }));

    await onSaveImportedPlaylist(spotifyResult.title, trackIds, tracksInfo);
    setShowSpotifyModal(false);
    setSpotifyResult(null);
    setSpotifyInput("");
    setSpotifyMessage(null);
    setSpotifyFilterQuery("");
  };

  const displayedSpotifyTracks = useMemo(() => {
    if (!spotifyResult) return [];
    if (!spotifyFilterQuery.trim()) return spotifyResult.tracks;
    return spotifyResult.tracks.filter((track) => matchesPlaylistSearch(track, spotifyFilterQuery));
  }, [spotifyResult, spotifyFilterQuery]);

  const handleSearchMissingInSoulseek = async (trackTitle: string, artistName: string) => {
    await onLaunchSoulseek(trackTitle, artistName);
  };

  const formatSeconds = (secs?: number) => {
    if (!secs || isNaN(secs)) return "0:00";
    const mins = Math.floor(secs / 60);
    const remainingSecs = Math.floor(secs % 60);
    return `${mins}:${remainingSecs < 10 ? "0" : ""}${remainingSecs}`;
  };

  return (
    <div>
      {/* Header */}
      <div className="view-header">
        {viewMode === "smart_mixes" ? (
          <div>
            <h1 className="view-title" style={{ display: "flex", alignItems: "center", gap: "10px" }}>
              <Sparkles size={28} color="var(--accent-secondary)" />
              <span>Smart Mixes</span>
            </h1>
            <p style={{ color: "var(--text-dim)", fontSize: "0.88rem", marginTop: "4px" }}>
              Auto-curated library blends, daily discovery, and genre mixes based on your library
            </p>
          </div>
        ) : (
          <div>
            <h1 className="view-title" style={{ display: "flex", alignItems: "center", gap: "10px" }}>
              <ListMusic size={28} color="var(--accent-light)" />
              <span>Playlists</span>
            </h1>
            <p style={{ color: "var(--text-dim)", fontSize: "0.88rem", marginTop: "4px" }}>
              Your custom playlists and imported Spotify collections
            </p>
          </div>
        )}

        {viewMode !== "smart_mixes" && (
          <div style={{ display: "flex", gap: "10px" }}>
            <button
              className="btn btn-secondary"
              onClick={() => setShowSpotifyModal(true)}
              style={{ display: "flex", alignItems: "center", gap: "8px" }}
            >
              <Share2 size={16} color="var(--accent-secondary)" />
              <span>Import from Spotify</span>
            </button>

            <button className="btn btn-primary" onClick={() => setShowCreateModal(true)}>
              <Plus size={16} />
              <span>New Playlist</span>
            </button>
          </div>
        )}
      </div>

      {/* SMART MIXES VIEW */}
      {(viewMode === "smart_mixes" || viewMode === "all") && (
        <div style={{ marginBottom: viewMode === "all" ? "40px" : "0" }}>
          {viewMode === "all" && (
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "16px" }}>
              <h2 style={{ fontSize: "1.15rem", fontWeight: 600, display: "flex", alignItems: "center", gap: "8px" }}>
                <Sparkles size={18} color="var(--accent-light)" />
                <span>Curated Smart Mixes & Recommendations</span>
              </h2>
              <span style={{ fontSize: "0.82rem", color: "var(--text-dim)" }}>
                Automatically derived from your genres and library
              </span>
            </div>
          )}

          {smartMixes.length === 0 ? (
            <div
              style={{
                padding: "40px 20px",
                borderRadius: "10px",
                backgroundColor: "var(--bg-card)",
                border: "1px dashed var(--border)",
                textAlign: "center",
                color: "var(--text-dim)",
              }}
            >
              <Sparkles size={36} color="var(--accent-light)" style={{ margin: "0 auto 12px" }} />
              <div style={{ fontWeight: 600, color: "var(--text-main)", marginBottom: "4px" }}>Generating Smart Mixes...</div>
              <p style={{ fontSize: "0.85rem" }}>
                Auto-curating mixes based on your music library folders and genres.
              </p>
            </div>
          ) : (
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(auto-fill, minmax(250px, 1fr))",
                gap: "18px",
              }}
            >
              {smartMixes.map((mix) => {
                const isSaved = savedMixIds.has(mix.id);
                const isSaving = savingMixId === mix.id;

                return (
                  <div
                    key={mix.id}
                    onClick={() => handleOpenPlaylistDetails(mix)}
                    style={{
                      backgroundColor: "var(--bg-card)",
                      borderRadius: "12px",
                      padding: "18px",
                      border: "1px solid var(--border)",
                      cursor: "pointer",
                      display: "flex",
                      flexDirection: "column",
                      justifyContent: "space-between",
                      transition: "transform 0.15s, background-color 0.15s, border-color 0.15s",
                    }}
                    onMouseEnter={(e) => {
                      e.currentTarget.style.backgroundColor = "var(--bg-card-hover)";
                      e.currentTarget.style.borderColor = "var(--accent-light)";
                    }}
                    onMouseLeave={(e) => {
                      e.currentTarget.style.backgroundColor = "var(--bg-card)";
                      e.currentTarget.style.borderColor = "var(--border)";
                    }}
                    title="Click to view tracks in this mix"
                  >
                    <div>
                      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", marginBottom: "12px" }}>
                        <div
                          style={{
                            width: "48px",
                            height: "48px",
                            borderRadius: "10px",
                            background: "linear-gradient(135deg, rgba(243, 112, 30, 0.35), rgba(139, 124, 246, 0.15))",
                            display: "flex",
                            alignItems: "center",
                            justifyContent: "center",
                            overflow: "hidden",
                            flexShrink: 0,
                          }}
                        >
                            <img
                              src={mix.cover_art_url || "/kaze-playlist-default.svg"}
                              alt={mix.name}
                              style={{ width: "100%", height: "100%", objectFit: "cover" }}
                            />
                        </div>
                        <span className="badge badge-exact">Smart Mix</span>
                      </div>

                      <div style={{ fontWeight: 700, fontSize: "1.05rem", color: "#e8d8c9", marginBottom: "6px" }}>
                        {mix.name}
                      </div>
                      <div style={{ fontSize: "0.82rem", color: "var(--text-dim)", lineHeight: 1.4, marginBottom: "16px" }}>
                        {mix.description || mix.generation_reason || "Curated blend of local tracks"}
                      </div>
                    </div>

                    <div style={{ display: "flex", gap: "8px", borderTop: "1px solid var(--border)", paddingTop: "14px" }}>
                      <button
                        className="btn btn-primary"
                        style={{ flex: 1, justifyContent: "center", fontSize: "0.85rem", padding: "8px 12px" }}
                        onClick={(e) => {
                          e.stopPropagation();
                          onPlayPlaylist(mix.id);
                        }}
                      >
                        <Play size={15} fill="currentColor" />
                        <span>Play Mix</span>
                      </button>

                      <button
                        className="btn btn-secondary"
                        style={{
                          fontSize: "0.82rem",
                          padding: "8px 12px",
                          display: "flex",
                          alignItems: "center",
                          gap: "6px",
                          backgroundColor: isSaved ? "rgba(139, 124, 246, 0.15)" : undefined,
                          borderColor: isSaved ? "var(--accent-secondary)" : undefined,
                          color: isSaved ? "var(--accent-secondary)" : undefined,
                        }}
                        onClick={(e) => {
                          e.stopPropagation();
                          handleSaveMixToUserPlaylist(mix);
                        }}
                        disabled={isSaving}
                        title="Add this mix as a permanent playlist in your library"
                      >
                        {isSaved ? (
                          <>
                            <Check size={14} color="var(--accent-secondary)" />
                            <span>Saved</span>
                          </>
                        ) : (
                          <>
                            <Plus size={14} />
                            <span>{isSaving ? "Saving..." : "Add to Playlist"}</span>
                          </>
                        )}
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      )}

      {/* CUSTOM PLAYLISTS VIEW */}
      {(viewMode === "playlists" || viewMode === "all") && (
        <div>
          {viewMode === "all" && (
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "16px" }}>
              <h2 style={{ fontSize: "1.15rem", fontWeight: 600 }}>Your Custom Playlists</h2>
              <span style={{ fontSize: "0.82rem", color: "var(--text-dim)" }}>
                {customPlaylists.length} custom {customPlaylists.length === 1 ? "playlist" : "playlists"}
              </span>
            </div>
          )}

          {!currentUser ? (
            <div
              style={{
                padding: "50px 20px",
                borderRadius: "10px",
                backgroundColor: "var(--bg-card)",
                border: "1px solid var(--border)",
                textAlign: "center",
                color: "var(--text-dim)",
              }}
            >
              <ListMusic size={40} color="var(--text-dim)" style={{ margin: "0 auto 12px" }} />
              <div style={{ fontWeight: 600, fontSize: "1.05rem", color: "var(--text-main)", marginBottom: "6px" }}>
                Sign In to Manage Playlists
              </div>
              <p style={{ fontSize: "0.88rem", marginBottom: "20px" }}>
                Sign in to your account to create, customize, and synchronize your personal playlists across devices.
              </p>
              {onOpenAuthModal && (
                <div style={{ display: "flex", justifyContent: "center", gap: "12px" }}>
                  <button className="btn btn-primary" onClick={onOpenAuthModal}>
                    <span>Sign In / Sign Up</span>
                  </button>
                </div>
              )}
            </div>
          ) : customPlaylists.length === 0 ? (
            <div
              style={{
                padding: "50px 20px",
                borderRadius: "10px",
                backgroundColor: "var(--bg-card)",
                border: "1px solid var(--border)",
                textAlign: "center",
                color: "var(--text-dim)",
              }}
            >
              <ListMusic size={40} color="var(--text-dim)" style={{ margin: "0 auto 12px" }} />
              <div style={{ fontWeight: 600, fontSize: "1.05rem", color: "var(--text-main)", marginBottom: "6px" }}>
                No custom playlists yet
              </div>
              <p style={{ fontSize: "0.88rem", marginBottom: "20px" }}>
                Create custom playlists manually, save a Smart Mix as a permanent playlist, or import directly from Spotify.
              </p>
              <div style={{ display: "flex", justifyContent: "center", gap: "12px" }}>
                <button className="btn btn-primary" onClick={() => setShowCreateModal(true)}>
                  <Plus size={16} />
                  <span>Create Playlist</span>
                </button>
                <button className="btn btn-secondary" onClick={() => setShowSpotifyModal(true)}>
                  <Share2 size={16} color="var(--accent-secondary)" />
                  <span>Import from Spotify</span>
                </button>
              </div>
            </div>
          ) : (
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(auto-fill, minmax(220px, 1fr))",
                gap: "18px",
              }}
            >
              {customPlaylists.map((pl) => (
                <div
                  key={pl.id}
                  onClick={() => handleOpenPlaylistDetails(pl)}
                  style={{
                    backgroundColor: "var(--bg-card)",
                    borderRadius: "12px",
                    padding: "16px",
                    border: "1px solid var(--border)",
                    cursor: "pointer",
                    display: "flex",
                    flexDirection: "column",
                    justifyContent: "space-between",
                    transition: "background-color 0.15s, border-color 0.15s, transform 0.15s",
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.backgroundColor = "var(--bg-card-hover)";
                    e.currentTarget.style.borderColor = "var(--accent-light)";
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.backgroundColor = "var(--bg-card)";
                    e.currentTarget.style.borderColor = "var(--border)";
                  }}
                  title="Click to view tracks in this playlist"
                >
                  <div>
                    <div
                      style={{
                        position: "relative",
                        width: "100%",
                        aspectRatio: "1/1",
                        borderRadius: "8px",
                        backgroundColor: "var(--bg-sidebar)",
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "center",
                        marginBottom: "12px",
                        overflow: "hidden",
                      }}
                    >
                      <details
                        style={{ position: "absolute", top: "8px", right: "8px", zIndex: 4 }}
                        onClick={(e) => e.stopPropagation()}
                      >
                        <summary
                          className="player-icon-btn"
                          aria-label={`Playlist options for ${pl.name}`}
                          style={{
                            listStyle: "none",
                            width: "32px",
                            height: "32px",
                            display: "flex",
                            alignItems: "center",
                            justifyContent: "center",
                            borderRadius: "50%",
                            background: "rgba(0, 0, 0, 0.68)",
                            color: "#e8d8c9",
                            cursor: "pointer",
                          }}
                        >
                          <MoreVertical size={18} />
                        </summary>
                        <div
                          style={{
                            position: "absolute",
                            top: "38px",
                            right: 0,
                            minWidth: "150px",
                            padding: "6px",
                            borderRadius: "8px",
                            border: "1px solid var(--border)",
                            background: "var(--bg-sidebar)",
                            boxShadow: "0 12px 30px rgba(0,0,0,0.45)",
                          }}
                        >
                          {pl.name === "Liked Songs" ? (
                            <div style={{ padding: "8px 10px", color: "var(--text-dim)", fontSize: "0.78rem" }}>
                              Default playlist
                            </div>
                          ) : (
                            <>
                              <button
                                className="btn btn-secondary"
                                style={{ width: "100%", justifyContent: "flex-start", border: 0 }}
                                onClick={(e) => {
                                  e.stopPropagation();
                                  (e.currentTarget.closest("details") as HTMLDetailsElement)?.removeAttribute("open");
                                  handleOpenRenameModal(pl);
                                }}
                              >
                                <Pencil size={14} /> Rename
                              </button>
                              <button
                                className="btn btn-secondary"
                                style={{ width: "100%", justifyContent: "flex-start", border: 0, color: "var(--danger, #ef4444)" }}
                                onClick={(e) => {
                                  e.stopPropagation();
                                  (e.currentTarget.closest("details") as HTMLDetailsElement)?.removeAttribute("open");
                                  handleOpenDeleteModal(pl);
                                }}
                              >
                                <Trash2 size={14} /> Delete
                              </button>
                            </>
                          )}
                        </div>
                      </details>
                        {pl.name === "Liked Songs" ? (
                          <div style={{ width: "100%", height: "100%", display: "flex", alignItems: "center", justifyContent: "center", background: "linear-gradient(145deg, rgba(236,72,153,0.32), rgba(139,124,246,0.18))" }}>
                            <Heart size={82} color="#f472b6" fill="#ec4899" strokeWidth={1.5} />
                          </div>
                        ) : (
                          <img
                            src={pl.cover_art_url || "/kaze-playlist-default.svg"}
                            alt={pl.name}
                            style={{ width: "100%", height: "100%", objectFit: "cover" }}
                          />
                        )}
                    </div>
                    <div style={{ fontWeight: 600, fontSize: "1rem", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                      {pl.name}
                    </div>
                    <div style={{ fontSize: "0.8rem", color: "var(--text-dim)", marginTop: "4px" }}>
                      {pl.description || "User Playlist"}
                    </div>
                  </div>

                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginTop: "14px", borderTop: "1px solid var(--border)", paddingTop: "10px" }}>
                    <button
                      className="btn btn-primary"
                      style={{ fontSize: "0.8rem", padding: "6px 12px", display: "flex", alignItems: "center", gap: "6px" }}
                      onClick={(e) => {
                        e.stopPropagation();
                        onPlayPlaylist(pl.id);
                      }}
                    >
                      <Play size={13} fill="currentColor" />
                      <span>Play</span>
                    </button>
                    <span style={{ fontSize: "0.8rem", color: "var(--text-dim)" }}>
                      {pl.track_count ?? 0} tracks
                    </span>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* MODAL 1: CREATE PLAYLIST */}
      {showCreateModal && (
        <div className="modal-overlay">
          <div className="modal-content" style={{ maxWidth: "420px" }}>
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "16px" }}>
              <h3 style={{ fontSize: "1.2rem", fontWeight: 700 }}>Create New Playlist</h3>
              <button
                onClick={() => setShowCreateModal(false)}
                style={{ background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer" }}
              >
                <X size={18} />
              </button>
            </div>

            <form onSubmit={handleCreate}>
              <input
                type="text"
                placeholder="Playlist name"
                value={playlistName}
                onChange={(e) => setPlaylistName(e.target.value)}
                className="input-field"
                style={{ width: "100%", marginBottom: "20px" }}
                autoFocus
              />
              <div style={{ display: "flex", gap: "12px", justifyContent: "flex-end" }}>
                <button
                  type="button"
                  className="btn btn-secondary"
                  onClick={() => setShowCreateModal(false)}
                >
                  Cancel
                </button>
                <button type="submit" className="btn btn-primary" disabled={!playlistName.trim()}>
                  Create Playlist
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* MODAL 2: PLAYLIST DETAILS / TRACK LIST */}
      {selectedPlaylist && (
        <div className="modal-overlay" onClick={() => setSelectedPlaylist(null)}>
          <div
            className="modal-content"
            style={{ maxWidth: "720px", width: "95%", maxHeight: "85vh", display: "flex", flexDirection: "column" }}
            onClick={(e) => e.stopPropagation()}
          >
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", marginBottom: "16px" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "14px" }}>
                {selectedPlaylist.cover_art_url && (
                  <img
                    src={selectedPlaylist.cover_art_url}
                    alt={selectedPlaylist.name}
                    style={{ width: "56px", height: "56px", borderRadius: "8px", objectFit: "cover", flexShrink: 0 }}
                  />
                )}
                <div>
                  <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                    <h3 style={{ fontSize: "1.3rem", fontWeight: 700 }}>{selectedPlaylist.name}</h3>
                    {selectedPlaylist.is_smart_mix === 1 && <span className="badge badge-exact">Smart Mix</span>}
                  </div>
                  <p style={{ fontSize: "0.85rem", color: "var(--text-dim)", marginTop: "4px" }}>
                    {selectedPlaylist.description || selectedPlaylist.generation_reason || "Custom playlist"}
                  </p>
                </div>
              </div>

              <button
                onClick={() => setSelectedPlaylist(null)}
                style={{ background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer", padding: "4px" }}
              >
                <X size={20} />
              </button>
            </div>

            <div style={{ display: "flex", gap: "10px", alignItems: "center", marginBottom: "16px" }}>
              <button
                className="btn btn-primary"
                onClick={() => {
                  onPlayPlaylist(selectedPlaylist.id);
                  setSelectedPlaylist(null);
                }}
              >
                <Play size={15} fill="currentColor" />
                <span>Play Entire Playlist</span>
              </button>

              {selectedPlaylist.is_smart_mix === 1 && (
                <button
                  className="btn btn-secondary"
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: "6px",
                    backgroundColor: savedMixIds.has(selectedPlaylist.id) ? "rgba(139, 124, 246, 0.15)" : undefined,
                    borderColor: savedMixIds.has(selectedPlaylist.id) ? "var(--accent-secondary)" : undefined,
                    color: savedMixIds.has(selectedPlaylist.id) ? "var(--accent-secondary)" : undefined,
                  }}
                  onClick={() => handleSaveMixToUserPlaylist(selectedPlaylist)}
                  disabled={savingMixId === selectedPlaylist.id}
                >
                  {savedMixIds.has(selectedPlaylist.id) ? (
                    <>
                      <Check size={14} color="var(--accent-secondary)" />
                      <span>Saved to Playlists</span>
                    </>
                  ) : (
                    <>
                      <Plus size={14} />
                      <span>{savingMixId === selectedPlaylist.id ? "Saving..." : "Add to Playlists"}</span>
                    </>
                  )}
                </button>
              )}

              <span style={{ fontSize: "0.85rem", color: "var(--text-dim)", marginLeft: "auto" }}>
                {playlistTracks.length} tracks
              </span>
            </div>

            {loadingTracks ? (
              <div style={{ padding: "40px 0", textAlign: "center", color: "var(--text-muted)" }}>
                Loading tracks...
              </div>
            ) : playlistTracks.length === 0 ? (
              <div style={{ padding: "40px 0", textAlign: "center", color: "var(--text-dim)" }}>
                No tracks currently in this playlist.
              </div>
            ) : (
              <div style={{ overflowY: "auto", flex: 1 }}>
                <table className="track-table" style={{ width: "100%" }}>
                  <thead>
                    <tr>
                      <th style={{ width: "40px", textAlign: "center" }}>#</th>
                      <th>Title</th>
                      <th>Artist</th>
                      <th style={{ width: "80px", textAlign: "right" }}>Length</th>
                      <th style={{ width: "80px", textAlign: "center" }}>Actions</th>
                    </tr>
                  </thead>
                  <tbody>
                    {playlistTracks.map((tr, idx) => (
                      <tr key={tr.id} className="track-row" onDoubleClick={() => onPlayTrack(tr.id)}>
                        <td style={{ textAlign: "center", color: "var(--text-dim)" }}>{idx + 1}</td>
                        <td className="primary">{tr.title}</td>
                        <td>{tr.artist_name || "Unknown Artist"}</td>
                        <td style={{ textAlign: "right", fontFamily: "monospace" }}>
                          {formatSeconds(tr.duration_secs)}
                        </td>
                        <td>
                          <div style={{ display: "flex", justifyContent: "center", gap: "6px" }}>
                            <button
                              className="player-icon-btn"
                              title="Play Track"
                              onClick={() => onPlayTrack(tr.id)}
                            >
                              <Play size={14} />
                            </button>
                            {(() => {
                              const isEnqueued = queuedTrackIds ? queuedTrackIds.has(tr.id) : false;
                              return (
                                <button
                                  className="player-icon-btn"
                                  title={isEnqueued ? "Already in queue" : "Add to queue"}
                                  onClick={() => onEnqueueTrack(tr.id)}
                                  style={{ color: isEnqueued ? "var(--success)" : "var(--text-muted)" }}
                                >
                                  {isEnqueued ? <Check size={14} /> : <ListPlus size={14} />}
                                </button>
                              );
                            })()}
                          </div>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        </div>
      )}

      {/* MODAL 3: SPOTIFY PLAYLIST IMPORT */}
      {showSpotifyModal && (
        <div
          className="modal-overlay"
          onClick={() => {
            setShowSpotifyModal(false);
            setSpotifyResult(null);
            setSpotifyInput("");
            setSpotifyMessage(null);
          }}
        >
          <div
            className="modal-content"
            style={{ maxWidth: "800px", width: "95%", maxHeight: "88vh", display: "flex", flexDirection: "column" }}
            onClick={(e) => e.stopPropagation()}
          >
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "16px" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
                <Share2 size={24} color="var(--accent-secondary)" />
                <h3 style={{ fontSize: "1.3rem", fontWeight: 700 }}>Import Spotify Playlist</h3>
              </div>
              <button
                onClick={() => {
                  setShowSpotifyModal(false);
                  setSpotifyResult(null);
                  setSpotifyInput("");
                  setSpotifyMessage(null);
                }}
                style={{ background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer" }}
              >
                <X size={20} />
              </button>
            </div>

            <p style={{ fontSize: "0.88rem", color: "var(--text-muted)", marginBottom: "16px", lineHeight: 1.5 }}>
              Paste a public Spotify playlist URL or ID. Kaze will inspect the tracks, match them
              against your local library, and let you download missing tracks via SoulseekQt.
            </p>

            {/* Input Form */}
            <form onSubmit={handleInspectSpotify} style={{ display: "flex", gap: "10px", marginBottom: "20px" }}>
              <input
                type="text"
                placeholder="https://open.spotify.com/playlist/... or 37i9dQZF1DXcBWIGoYBM5M"
                value={spotifyInput}
                onChange={(e) => setSpotifyInput(e.target.value)}
                className="input-field"
                style={{ flex: 1, fontFamily: "monospace", fontSize: "0.9rem" }}
              />
              <button
                type="submit"
                disabled={inspectingSpotify || !spotifyInput.trim()}
                className="btn btn-primary"
              >
                <Sparkles size={15} />
                <span>{inspectingSpotify ? "Inspecting..." : "Inspect Playlist"}</span>
              </button>
            </form>

            {spotifyMessage && (
              <div
                style={{
                  padding: "12px 16px",
                  borderRadius: "8px",
                  backgroundColor: "rgba(239, 68, 68, 0.12)",
                  border: "1px solid var(--danger)",
                  color: "#fca5a5",
                  fontSize: "0.85rem",
                  marginBottom: "16px",
                  display: "flex",
                  alignItems: "center",
                  gap: "8px",
                }}
              >
                <AlertCircle size={16} />
                <span>{spotifyMessage}</span>
              </div>
            )}

            {/* Inspected Playlist Result */}
            {spotifyResult && (
              <div style={{ display: "flex", flexDirection: "column", flex: 1, overflow: "hidden" }}>
                <div
                  style={{
                    display: "flex",
                    justifyContent: "space-between",
                    alignItems: "center",
                    padding: "14px",
                    borderRadius: "8px",
                    backgroundColor: "var(--bg-card)",
                    border: "1px solid var(--border)",
                    marginBottom: "16px",
                    flexWrap: "wrap",
                    gap: "12px",
                  }}
                >
                  <div>
                    <div style={{ fontWeight: 700, fontSize: "1.1rem", color: "#e8d8c9" }}>{spotifyResult.title}</div>
                    <div style={{ display: "flex", gap: "12px", marginTop: "6px", fontSize: "0.82rem" }}>
                      <span style={{ color: "var(--text-dim)" }}>Total: {spotifyResult.total_tracks} tracks</span>
                      <span style={{ color: "var(--success)", fontWeight: 600 }}>
                        ● {spotifyResult.matched_tracks} Local
                      </span>
                      <span style={{ color: "var(--accent-secondary, #f3701e)", fontWeight: 600 }}>
                        ● {spotifyResult.missing_tracks} Online
                      </span>
                    </div>
                  </div>

                  <div>
                    <button
                      className="btn btn-primary"
                      onClick={() => handleSaveSpotifyPlaylist(true)}
                      title="Import this entire playlist into Kaze"
                      style={{ display: "flex", alignItems: "center", gap: "8px" }}
                    >
                      <Check size={16} />
                      <span>Add as Playlist</span>
                    </button>
                  </div>
                </div>

                {/* Track Search Bar in Spotify Inspection Modal */}
                <div style={{ position: "relative", marginBottom: "12px" }}>
                  <Search
                    size={14}
                    color="var(--text-muted)"
                    style={{ position: "absolute", left: "12px", top: "50%", transform: "translateY(-50%)", pointerEvents: "none" }}
                  />
                  <input
                    type="text"
                    placeholder="Search songs or artists in this playlist..."
                    value={spotifyFilterQuery}
                    onChange={(e) => setSpotifyFilterQuery(e.target.value)}
                    style={{
                      width: "100%",
                      padding: "8px 32px 8px 34px",
                      borderRadius: "8px",
                      border: "1px solid var(--border)",
                      backgroundColor: "rgba(0, 0, 0, 0.25)",
                      color: "#e8d8c9",
                      fontSize: "0.82rem",
                      outline: "none",
                    }}
                  />
                  {spotifyFilterQuery && (
                    <button
                      type="button"
                      onClick={() => setSpotifyFilterQuery("")}
                      style={{
                        position: "absolute",
                        right: "10px",
                        top: "50%",
                        transform: "translateY(-50%)",
                        background: "none",
                        border: "none",
                        color: "var(--text-muted)",
                        cursor: "pointer",
                        padding: "2px",
                        display: "flex",
                        alignItems: "center",
                      }}
                      title="Clear search"
                    >
                      <X size={13} />
                    </button>
                  )}
                </div>

                {/* Track inspection table */}
                <div style={{ overflowY: "auto", flex: 1 }}>
                  <table className="track-table" style={{ width: "100%" }}>
                    <thead>
                      <tr>
                        <th style={{ width: "40px", textAlign: "center" }}>#</th>
                        <th>Track</th>
                        <th>Artist</th>
                        <th style={{ width: "120px", textAlign: "center" }}>Status</th>
                        <th style={{ width: "120px", textAlign: "center" }}>Action</th>
                      </tr>
                    </thead>
                    <tbody>
                      {displayedSpotifyTracks.map((tr, idx) => (
                        <tr key={tr.spotify_id || idx} className="track-row">
                          <td style={{ textAlign: "center", color: "var(--text-dim)" }}>{idx + 1}</td>
                          <td className="primary">{tr.title}</td>
                          <td>{tr.artist}</td>
                          <td style={{ textAlign: "center" }}>
                            {tr.in_library ? (
                              <span className="badge badge-exact">Local</span>
                            ) : (
                              <span
                                className="badge"
                                style={{ backgroundColor: "rgba(243, 112, 30, 0.15)", color: "var(--accent-secondary, #f3701e)" }}
                              >
                                Online
                              </span>
                            )}
                          </td>
                          <td style={{ textAlign: "center" }}>
                            {!tr.in_library && (
                              <div style={{ display: "flex", alignItems: "center", justifyContent: "center" }}>
                                {onSearchDirect ? (
                                  <button
                                    className="btn btn-secondary"
                                    style={{ padding: "4px 10px", fontSize: "0.75rem", display: "inline-flex", alignItems: "center", gap: "5px" }}
                                    onClick={() => onSearchDirect(tr.artist, tr.title)}
                                    title="Download this track to your local library"
                                  >
                                    <DownloadCloud size={13} />
                                    <span>Download</span>
                                  </button>
                                ) : (
                                  <button
                                    className="btn btn-secondary"
                                    style={{ padding: "4px 10px", fontSize: "0.75rem", display: "inline-flex", alignItems: "center", gap: "5px" }}
                                    onClick={() => handleSearchMissingInSoulseek(tr.title, tr.artist)}
                                    title="Search and download track"
                                  >
                                    <DownloadCloud size={13} />
                                    <span>Download</span>
                                  </button>
                                )}
                              </div>
                            )}
                          </td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </div>
            )}
          </div>
        </div>
      )}

      {/* MODAL 4: THEMED RENAME PLAYLIST MODAL */}
      {playlistToRename && (
        <div className="modal-overlay" onClick={() => !isRenaming && setPlaylistToRename(null)}>
          <div
            className="modal-content"
            style={{ maxWidth: "440px", width: "90%", padding: "24px" }}
            onClick={(e) => e.stopPropagation()}
          >
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "18px" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
                <Pencil size={20} color="var(--accent-light, #e8d8c9)" />
                <h3 style={{ fontSize: "1.2rem", fontWeight: 700, margin: 0 }}>Rename Playlist</h3>
              </div>
              <button
                onClick={() => setPlaylistToRename(null)}
                disabled={isRenaming}
                style={{ background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer" }}
              >
                <X size={18} />
              </button>
            </div>

            <form onSubmit={handleConfirmRename}>
              <div style={{ marginBottom: "16px" }}>
                <label style={{ display: "block", fontSize: "0.82rem", color: "var(--text-dim)", marginBottom: "8px" }}>
                  Playlist Name
                </label>
                <input
                  type="text"
                  autoFocus
                  value={renameValue}
                  onChange={(e) => {
                    setRenameValue(e.target.value);
                    setRenameError(null);
                  }}
                  className="input-field"
                  style={{ width: "100%", fontSize: "0.95rem" }}
                  placeholder="Enter playlist name"
                />
                {renameError && (
                  <p style={{ color: "var(--danger, #ef4444)", fontSize: "0.8rem", marginTop: "6px" }}>
                    {renameError}
                  </p>
                )}
              </div>

              <div style={{ display: "flex", gap: "10px", justifyContent: "flex-end" }}>
                <button
                  type="button"
                  className="btn btn-secondary"
                  onClick={() => setPlaylistToRename(null)}
                  disabled={isRenaming}
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="btn btn-primary"
                  disabled={isRenaming || !renameValue.trim()}
                >
                  {isRenaming ? "Saving..." : "Save Changes"}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* MODAL 5: THEMED DELETE CONFIRMATION MODAL */}
      {playlistToDelete && (
        <div className="modal-overlay" onClick={() => !isDeleting && setPlaylistToDelete(null)}>
          <div
            className="modal-content"
            style={{
              maxWidth: "420px",
              width: "90%",
              padding: "24px",
              border: "1px solid rgba(239, 68, 68, 0.25)",
              boxShadow: "0 20px 45px rgba(0, 0, 0, 0.6)",
            }}
            onClick={(e) => e.stopPropagation()}
          >
            <div style={{ display: "flex", alignItems: "flex-start", gap: "14px", marginBottom: "16px" }}>
              <div
                style={{
                  width: "42px",
                  height: "42px",
                  borderRadius: "10px",
                  backgroundColor: "rgba(239, 68, 68, 0.12)",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  flexShrink: 0,
                  color: "var(--danger, #ef4444)",
                }}
              >
                <Trash2 size={22} />
              </div>
              <div>
                <h3 style={{ fontSize: "1.15rem", fontWeight: 700, margin: "0 0 6px 0", color: "#fca5a5" }}>
                  Delete Playlist
                </h3>
                <p style={{ fontSize: "0.88rem", color: "var(--text-dim)", margin: 0, lineHeight: 1.5 }}>
                  Are you sure you want to delete <strong style={{ color: "#fff" }}>"{playlistToDelete.name}"</strong>?
                  This action cannot be undone.
                </p>
              </div>
            </div>

            <div style={{ display: "flex", gap: "10px", justifyContent: "flex-end", marginTop: "20px" }}>
              <button
                type="button"
                className="btn btn-secondary"
                onClick={() => setPlaylistToDelete(null)}
                disabled={isDeleting}
              >
                Cancel
              </button>
              <button
                type="button"
                className="btn"
                style={{
                  backgroundColor: "var(--danger, #ef4444)",
                  color: "#fff",
                  fontWeight: 600,
                  border: "none",
                  display: "flex",
                  alignItems: "center",
                  gap: "6px",
                }}
                onClick={handleConfirmDelete}
                disabled={isDeleting}
              >
                <Trash2 size={15} />
                <span>{isDeleting ? "Deleting..." : "Delete Playlist"}</span>
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
