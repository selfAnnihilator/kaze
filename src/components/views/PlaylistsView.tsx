import React, { useState } from "react";
import {
  ListMusic,
  Sparkles,
  Plus,
  Play,
  Share2,
  ExternalLink,
  DownloadCloud,
  Check,
  AlertCircle,
  X,
} from "lucide-react";
import { Playlist, Track, SpotifyPlaylistImport } from "../../types";

interface PlaylistsViewProps {
  playlists: Playlist[];
  onSelectPlaylist: (playlistId: string) => void;
  onPlayPlaylist: (playlistId: string) => void;
  onCreatePlaylist: (name: string, description?: string) => void;
  onInspectSpotifyPlaylist: (urlOrId: string) => Promise<SpotifyPlaylistImport | null>;
  onSaveImportedPlaylist: (name: string, trackIds: string[]) => Promise<void>;
  onAddMissingToWishlist: (tracks: any[]) => Promise<void>;
  onLaunchSoulseek: (query?: string) => Promise<void>;
  onFetchPlaylistTracks: (playlistId: string) => Promise<Track[]>;
  onPlayTrack: (trackId: string) => void;
  queuedTrackIds?: Set<string>;
  onEnqueueTrack: (trackId: string) => void;
  onDequeueTrack?: (trackId: string) => void;
}

export const PlaylistsView: React.FC<PlaylistsViewProps> = ({
  playlists,
  onPlayPlaylist,
  onCreatePlaylist,
  onInspectSpotifyPlaylist,
  onSaveImportedPlaylist,
  onAddMissingToWishlist,
  onLaunchSoulseek,
  onFetchPlaylistTracks,
  onPlayTrack,
  queuedTrackIds,
  onEnqueueTrack,
  onDequeueTrack,
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
  const [wishlistAdded, setWishlistAdded] = useState(false);

  // Split into smart mixes and custom playlists
  const smartMixes = playlists.filter((p) => p.is_smart_mix === 1);
  const customPlaylists = playlists.filter((p) => p.is_smart_mix !== 1);

  const handleCreate = (e: React.FormEvent) => {
    e.preventDefault();
    if (playlistName.trim()) {
      onCreatePlaylist(playlistName.trim());
      setPlaylistName("");
      setShowCreateModal(false);
    }
  };

  const handleOpenPlaylistDetails = async (pl: Playlist) => {
    setSelectedPlaylist(pl);
    setLoadingTracks(true);
    try {
      const tracks = await onFetchPlaylistTracks(pl.id);
      setPlaylistTracks(tracks);
    } finally {
      setLoadingTracks(false);
    }
  };

  const handleInspectSpotify = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!spotifyInput.trim()) return;
    setInspectingSpotify(true);
    setSpotifyMessage(null);
    setWishlistAdded(false);
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

  const handleSaveSpotifyPlaylist = async () => {
    if (!spotifyResult) return;
    const localIds = spotifyResult.tracks
      .filter((t) => t.in_library && t.matched_local_track_id)
      .map((t) => t.matched_local_track_id as string);

    if (localIds.length === 0) {
      alert("No matched tracks from this playlist exist in your local library yet.");
      return;
    }

    await onSaveImportedPlaylist(spotifyResult.title, localIds);
    setShowSpotifyModal(false);
    setSpotifyResult(null);
  };

  const handleAddSpotifyMissingToWishlist = async () => {
    if (!spotifyResult) return;
    const missing = spotifyResult.tracks.filter((t) => !t.in_library);
    if (missing.length === 0) return;
    await onAddMissingToWishlist(missing);
    setWishlistAdded(true);
  };

  const handleSearchMissingInSoulseek = async (trackTitle: string, artistName: string) => {
    const query = `${artistName} ${trackTitle}`;
    await onLaunchSoulseek(query);
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
        <div>
          <h1 className="view-title" style={{ display: "flex", alignItems: "center", gap: "10px" }}>
            <ListMusic size={28} color="var(--accent-light)" />
            <span>Playlists & Smart Mixes</span>
          </h1>
          <p style={{ color: "var(--text-dim)", fontSize: "0.88rem", marginTop: "4px" }}>
            Auto-curated library blends, custom tracklists, and Spotify imports
          </p>
        </div>

        <div style={{ display: "flex", gap: "10px" }}>
          <button
            className="btn btn-secondary"
            onClick={() => setShowSpotifyModal(true)}
            style={{ display: "flex", alignItems: "center", gap: "8px" }}
          >
            <Share2 size={16} color="#10b981" />
            <span>Import from Spotify</span>
          </button>

          <button className="btn btn-primary" onClick={() => setShowCreateModal(true)}>
            <Plus size={16} />
            <span>New Playlist</span>
          </button>
        </div>
      </div>

      {/* SECTION 1: AUTO-GENERATED SMART MIXES */}
      <div style={{ marginBottom: "40px" }}>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "16px" }}>
          <h2 style={{ fontSize: "1.15rem", fontWeight: 600, display: "flex", alignItems: "center", gap: "8px" }}>
            <Sparkles size={18} color="var(--accent-light)" />
            <span>Curated Smart Mixes & Recommendations</span>
          </h2>
          <span style={{ fontSize: "0.82rem", color: "var(--text-dim)" }}>
            Automatically derived from your genres and library
          </span>
        </div>

        {smartMixes.length === 0 ? (
          <div
            style={{
              padding: "30px",
              borderRadius: "10px",
              backgroundColor: "var(--bg-card)",
              border: "1px dashed var(--border)",
              textAlign: "center",
              color: "var(--text-dim)",
            }}
          >
            Auto-generating smart mixes from your music tracks...
          </div>
        ) : (
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(auto-fill, minmax(250px, 1fr))",
              gap: "18px",
            }}
          >
            {smartMixes.map((mix) => (
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
              >
                <div>
                  <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", marginBottom: "12px" }}>
                    <div
                      style={{
                        width: "48px",
                        height: "48px",
                        borderRadius: "10px",
                        background: "linear-gradient(135deg, rgba(139, 92, 246, 0.35), rgba(192, 132, 252, 0.15))",
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "center",
                      }}
                    >
                      <Sparkles size={24} color="#c084fc" />
                    </div>
                    <span className="badge badge-exact">Smart Mix</span>
                  </div>

                  <div style={{ fontWeight: 700, fontSize: "1.05rem", color: "#fff", marginBottom: "6px" }}>
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
                    style={{ fontSize: "0.85rem", padding: "8px 12px" }}
                    onClick={(e) => {
                      e.stopPropagation();
                      handleOpenPlaylistDetails(mix);
                    }}
                    title="View tracks"
                  >
                    <ListMusic size={15} />
                  </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* SECTION 2: CUSTOM PLAYLISTS */}
      <div>
        <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "16px" }}>
          <h2 style={{ fontSize: "1.15rem", fontWeight: 600 }}>Your Custom Playlists</h2>
          <span style={{ fontSize: "0.82rem", color: "var(--text-dim)" }}>
            {customPlaylists.length} custom {customPlaylists.length === 1 ? "playlist" : "playlists"}
          </span>
        </div>

        {customPlaylists.length === 0 ? (
          <div
            style={{
              padding: "40px 20px",
              borderRadius: "10px",
              backgroundColor: "var(--bg-card)",
              border: "1px solid var(--border)",
              textAlign: "center",
              color: "var(--text-dim)",
            }}
          >
            <ListMusic size={36} color="var(--text-dim)" style={{ margin: "0 auto 12px" }} />
            <div style={{ fontWeight: 600, color: "var(--text-main)", marginBottom: "4px" }}>No custom playlists yet</div>
            <p style={{ fontSize: "0.85rem", marginBottom: "16px" }}>
              Create custom playlists manually or import them directly from Spotify.
            </p>
            <button className="btn btn-primary" onClick={() => setShowCreateModal(true)}>
              <Plus size={16} />
              <span>Create Your First Playlist</span>
            </button>
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
                  borderRadius: "10px",
                  padding: "16px",
                  border: "1px solid var(--border)",
                  cursor: "pointer",
                  transition: "background-color 0.15s",
                }}
                onMouseEnter={(e) => (e.currentTarget.style.backgroundColor = "var(--bg-card-hover)")}
                onMouseLeave={(e) => (e.currentTarget.style.backgroundColor = "var(--bg-card)")}
              >
                <div
                  style={{
                    width: "100%",
                    aspectRatio: "1/1",
                    borderRadius: "8px",
                    backgroundColor: "var(--bg-sidebar)",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                    marginBottom: "12px",
                  }}
                >
                  <ListMusic size={36} color="#9ca3af" />
                </div>
                <div style={{ fontWeight: 600, fontSize: "0.95rem", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                  {pl.name}
                </div>
                <div style={{ fontSize: "0.8rem", color: "var(--text-dim)", marginTop: "4px" }}>Playlist</div>
              </div>
            ))}
          </div>
        )}
      </div>

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
              <div>
                <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                  <h3 style={{ fontSize: "1.3rem", fontWeight: 700 }}>{selectedPlaylist.name}</h3>
                  {selectedPlaylist.is_smart_mix === 1 && <span className="badge badge-exact">Smart Mix</span>}
                </div>
                <p style={{ fontSize: "0.85rem", color: "var(--text-dim)", marginTop: "4px" }}>
                  {selectedPlaylist.description || selectedPlaylist.generation_reason || "Custom playlist"}
                </p>
              </div>

              <button
                onClick={() => setSelectedPlaylist(null)}
                style={{ background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer", padding: "4px" }}
              >
                <X size={20} />
              </button>
            </div>

            <div style={{ display: "flex", gap: "10px", marginBottom: "16px" }}>
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
              <span style={{ alignSelf: "center", fontSize: "0.85rem", color: "var(--text-dim)" }}>
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
                                  title={isEnqueued ? "In queue (click to remove)" : "Enqueue Track"}
                                  onClick={() => {
                                    if (isEnqueued) {
                                      if (onDequeueTrack) onDequeueTrack(tr.id);
                                    } else {
                                      onEnqueueTrack(tr.id);
                                    }
                                  }}
                                  style={{ color: isEnqueued ? "var(--success)" : "var(--text-muted)" }}
                                >
                                  {isEnqueued ? <Check size={14} /> : <Plus size={14} />}
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
        <div className="modal-overlay" onClick={() => setShowSpotifyModal(false)}>
          <div
            className="modal-content"
            style={{ maxWidth: "800px", width: "95%", maxHeight: "88vh", display: "flex", flexDirection: "column" }}
            onClick={(e) => e.stopPropagation()}
          >
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "16px" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
                <Share2 size={24} color="#10b981" />
                <h3 style={{ fontSize: "1.3rem", fontWeight: 700 }}>Import Spotify Playlist</h3>
              </div>
              <button
                onClick={() => setShowSpotifyModal(false)}
                style={{ background: "none", border: "none", color: "var(--text-muted)", cursor: "pointer" }}
              >
                <X size={20} />
              </button>
            </div>

            <p style={{ fontSize: "0.88rem", color: "var(--text-muted)", marginBottom: "16px", lineHeight: 1.5 }}>
              Paste a public Spotify playlist URL or ID. SoundFlow will inspect the tracks, match them
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
                    <div style={{ fontWeight: 700, fontSize: "1.1rem", color: "#fff" }}>{spotifyResult.title}</div>
                    <div style={{ display: "flex", gap: "12px", marginTop: "6px", fontSize: "0.82rem" }}>
                      <span style={{ color: "var(--text-dim)" }}>Total: {spotifyResult.total_tracks} tracks</span>
                      <span style={{ color: "var(--success)", fontWeight: 600 }}>
                        ● {spotifyResult.matched_tracks} in Local Library
                      </span>
                      <span style={{ color: "#f59e0b", fontWeight: 600 }}>
                        ● {spotifyResult.missing_tracks} Missing
                      </span>
                    </div>
                  </div>

                  <div style={{ display: "flex", gap: "10px", flexWrap: "wrap" }}>
                    {spotifyResult.matched_tracks > 0 && (
                      <button className="btn btn-primary" onClick={handleSaveSpotifyPlaylist}>
                        <Check size={15} />
                        <span>Save Matched ({spotifyResult.matched_tracks})</span>
                      </button>
                    )}

                    {spotifyResult.missing_tracks > 0 && (
                      <button
                        className="btn btn-secondary"
                        onClick={handleAddSpotifyMissingToWishlist}
                        disabled={wishlistAdded}
                      >
                        <Plus size={15} />
                        <span>{wishlistAdded ? "Added to Wishlist!" : "Add Missing to Wishlist"}</span>
                      </button>
                    )}

                    {spotifyResult.missing_tracks > 0 && (
                      <button
                        className="btn btn-secondary"
                        onClick={() => {
                          const firstMissing = spotifyResult.tracks.find((t) => !t.in_library);
                          if (firstMissing) {
                            handleSearchMissingInSoulseek(firstMissing.title, firstMissing.artist);
                          } else {
                            onLaunchSoulseek();
                          }
                        }}
                        title="Launch SoulseekQt to search and download missing tracks"
                      >
                        <DownloadCloud size={15} color="var(--accent-light)" />
                        <span>Open SoulseekQt</span>
                      </button>
                    )}
                  </div>
                </div>

                {/* Track inspection table */}
                <div style={{ overflowY: "auto", flex: 1 }}>
                  <table className="track-table" style={{ width: "100%" }}>
                    <thead>
                      <tr>
                        <th style={{ width: "40px", textAlign: "center" }}>#</th>
                        <th>Track</th>
                        <th>Artist</th>
                        <th style={{ width: "130px", textAlign: "center" }}>Status</th>
                        <th style={{ width: "160px", textAlign: "center" }}>Soulseek</th>
                      </tr>
                    </thead>
                    <tbody>
                      {spotifyResult.tracks.map((tr, idx) => (
                        <tr key={tr.spotify_id || idx} className="track-row">
                          <td style={{ textAlign: "center", color: "var(--text-dim)" }}>{idx + 1}</td>
                          <td className="primary">{tr.title}</td>
                          <td>{tr.artist}</td>
                          <td style={{ textAlign: "center" }}>
                            {tr.in_library ? (
                              <span className="badge badge-exact">In Library</span>
                            ) : (
                              <span
                                className="badge"
                                style={{ backgroundColor: "rgba(245, 158, 11, 0.15)", color: "#f59e0b" }}
                              >
                                Missing
                              </span>
                            )}
                          </td>
                          <td style={{ textAlign: "center" }}>
                            {!tr.in_library && (
                              <button
                                className="btn btn-secondary"
                                style={{ padding: "4px 8px", fontSize: "0.75rem" }}
                                onClick={() => handleSearchMissingInSoulseek(tr.title, tr.artist)}
                                title="Search and download in SoulseekQt"
                              >
                                <ExternalLink size={12} />
                                <span>Find in SoulseekQt</span>
                              </button>
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
    </div>
  );
};
