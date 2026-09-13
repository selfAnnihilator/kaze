import React, { useState } from "react";
import { X, Plus, Check, ListMusic, AlertCircle } from "lucide-react";
import { Playlist } from "../../types";

export interface AddToPlaylistModalTrack {
  id: string;
  title: string;
  artist?: string;
  album?: string;
  cover_art_url?: string;
}

interface AddToPlaylistModalProps {
  isOpen: boolean;
  onClose: () => void;
  track: AddToPlaylistModalTrack | null;
  playlists: Playlist[];
  trackPlaylistIds: string[]; // List of playlist IDs this track currently belongs to
  onTogglePlaylist: (playlistId: string, isCurrentlyMember: boolean) => Promise<void>;
  onCreatePlaylist: (name: string) => Promise<string | null>; // Returns created playlist id or null
}

export const AddToPlaylistModal: React.FC<AddToPlaylistModalProps> = ({
  isOpen,
  onClose,
  track,
  playlists,
  trackPlaylistIds,
  onTogglePlaylist,
  onCreatePlaylist,
}) => {
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [newPlaylistName, setNewPlaylistName] = useState("");
  const [createError, setCreateError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [togglingIds, setTogglingIds] = useState<Set<string>>(new Set());

  // Strictly show user-created / user-saved playlists, never automatic smart mixes
  const userPlaylists = playlists.filter((p) => p.is_smart_mix !== 1);

  if (!isOpen || !track) return null;

  const handleCreateSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const trimmed = newPlaylistName.trim();
    if (!trimmed) return;

    // Case-sensitive exact match check
    const nameExists = userPlaylists.some((p) => p.name === trimmed);
    if (nameExists) {
      setCreateError(`A playlist named "${trimmed}" already exists.`);
      return;
    }

    setIsSubmitting(true);
    setCreateError(null);
    try {
      const createdId = await onCreatePlaylist(trimmed);
      if (createdId) {
        // Automatically add current track to the newly created playlist
        await onTogglePlaylist(createdId, false);
      }
      setNewPlaylistName("");
      setShowCreateForm(false);
    } catch (err: any) {
      setCreateError(err?.message || "Failed to create playlist");
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleToggle = async (playlistId: string) => {
    if (togglingIds.has(playlistId)) return;
    const isMember = trackPlaylistIds.includes(playlistId);

    setTogglingIds((prev) => new Set(prev).add(playlistId));
    try {
      await onTogglePlaylist(playlistId, isMember);
    } finally {
      setTogglingIds((prev) => {
        const next = new Set(prev);
        next.delete(playlistId);
        return next;
      });
    }
  };

  return (
    <div
      className="modal-overlay"
      style={{
        position: "fixed",
        inset: 0,
        backgroundColor: "rgba(0, 0, 0, 0.75)",
        backdropFilter: "blur(6px)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 9999,
        padding: "20px",
      }}
      onClick={onClose}
    >
      <div
        className="modal-content"
        style={{
          backgroundColor: "#18181b",
          border: "1px solid rgba(255, 255, 255, 0.12)",
          borderRadius: "16px",
          width: "100%",
          maxWidth: "460px",
          maxHeight: "85vh",
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
          boxShadow: "0 20px 50px rgba(0, 0, 0, 0.6)",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div
          style={{
            padding: "18px 20px",
            borderBottom: "1px solid rgba(255, 255, 255, 0.08)",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            gap: "12px",
          }}
        >
          <div style={{ minWidth: 0, flex: 1 }}>
            <h2 style={{ fontSize: "1.15rem", fontWeight: 700, color: "#fff", margin: 0 }}>
              Add to Playlist
            </h2>
            <p
              style={{
                fontSize: "0.82rem",
                color: "var(--text-muted)",
                marginTop: "2px",
                whiteSpace: "nowrap",
                overflow: "hidden",
                textOverflow: "ellipsis",
              }}
            >
              {track.title} {track.artist ? `• ${track.artist}` : ""}
            </p>
          </div>

          <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
            {!showCreateForm && (
              <button
                type="button"
                className="btn btn-primary"
                onClick={() => {
                  setShowCreateForm(true);
                  setCreateError(null);
                }}
                style={{
                  fontSize: "0.78rem",
                  padding: "6px 12px",
                  borderRadius: "8px",
                  display: "inline-flex",
                  alignItems: "center",
                  gap: "5px",
                  backgroundColor: "var(--accent)",
                  fontWeight: 600,
                }}
                title="Create a new playlist"
              >
                <Plus size={15} />
                <span>New Playlist</span>
              </button>
            )}

            <button
              type="button"
              onClick={onClose}
              style={{
                background: "none",
                border: "none",
                color: "var(--text-dim)",
                cursor: "pointer",
                padding: "6px",
                display: "flex",
                borderRadius: "6px",
              }}
              title="Close modal"
            >
              <X size={18} />
            </button>
          </div>
        </div>

        {/* Inline Create Form */}
        {showCreateForm && (
          <form
            onSubmit={handleCreateSubmit}
            style={{
              padding: "14px 20px",
              backgroundColor: "rgba(255, 255, 255, 0.03)",
              borderBottom: "1px solid rgba(255, 255, 255, 0.08)",
            }}
          >
            <div style={{ display: "flex", gap: "8px" }}>
              <input
                type="text"
                autoFocus
                placeholder="Enter playlist name..."
                value={newPlaylistName}
                onChange={(e) => {
                  setNewPlaylistName(e.target.value);
                  if (createError) setCreateError(null);
                }}
                style={{
                  flex: 1,
                  backgroundColor: "rgba(0, 0, 0, 0.4)",
                  border: createError ? "1px solid #ef4444" : "1px solid rgba(255, 255, 255, 0.15)",
                  borderRadius: "8px",
                  padding: "8px 12px",
                  color: "#fff",
                  fontSize: "0.88rem",
                  outline: "none",
                }}
              />
              <button
                type="submit"
                className="btn btn-primary"
                disabled={!newPlaylistName.trim() || isSubmitting}
                style={{
                  padding: "8px 14px",
                  fontSize: "0.82rem",
                  fontWeight: 600,
                  opacity: !newPlaylistName.trim() || isSubmitting ? 0.6 : 1,
                }}
              >
                {isSubmitting ? "Creating..." : "Create"}
              </button>
              <button
                type="button"
                className="btn btn-secondary"
                onClick={() => {
                  setShowCreateForm(false);
                  setCreateError(null);
                  setNewPlaylistName("");
                }}
                style={{ padding: "8px 12px", fontSize: "0.82rem" }}
              >
                Cancel
              </button>
            </div>

            {createError && (
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: "6px",
                  marginTop: "8px",
                  fontSize: "0.78rem",
                  color: "#ef4444",
                }}
              >
                <AlertCircle size={14} />
                <span>{createError}</span>
              </div>
            )}
          </form>
        )}

        {/* Playlists List */}
        <div
          style={{
            flex: 1,
            overflowY: "auto",
            padding: "10px 14px",
            display: "flex",
            flexDirection: "column",
            gap: "4px",
          }}
        >
          {userPlaylists.length === 0 ? (
            <div style={{ textAlign: "center", padding: "40px 20px", color: "var(--text-dim)" }}>
              <ListMusic size={36} color="var(--text-dim)" style={{ marginBottom: "10px" }} />
              <p style={{ fontSize: "0.92rem", fontWeight: 600, color: "#fff", marginBottom: "4px" }}>
                No playlists available
              </p>
              <p style={{ fontSize: "0.82rem", color: "var(--text-muted)" }}>
                Click &ldquo;+ New Playlist&rdquo; above to create your first playlist.
              </p>
            </div>
          ) : (
            userPlaylists.map((pl) => {
              const isMember = trackPlaylistIds.includes(pl.id);
              const isToggling = togglingIds.has(pl.id);

              return (
                <div
                  key={pl.id}
                  onClick={() => handleToggle(pl.id)}
                  style={{
                    display: "flex",
                    alignItems: "center",
                    gap: "14px",
                    padding: "10px 12px",
                    borderRadius: "10px",
                    cursor: "pointer",
                    backgroundColor: isMember ? "rgba(99, 102, 241, 0.08)" : "transparent",
                    border: isMember
                      ? "1px solid rgba(99, 102, 241, 0.3)"
                      : "1px solid transparent",
                    transition: "all 0.15s ease",
                  }}
                  onMouseEnter={(e) => {
                    if (!isMember) {
                      e.currentTarget.style.backgroundColor = "rgba(255, 255, 255, 0.04)";
                    }
                  }}
                  onMouseLeave={(e) => {
                    if (!isMember) {
                      e.currentTarget.style.backgroundColor = "transparent";
                    }
                  }}
                >
                  {/* Checkbox indicator */}
                  <div
                    style={{
                      width: "22px",
                      height: "22px",
                      borderRadius: "6px",
                      border: isMember
                        ? "2px solid #10b981"
                        : "2px solid rgba(255, 255, 255, 0.3)",
                      backgroundColor: isMember ? "#10b981" : "transparent",
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "center",
                      flexShrink: 0,
                      transition: "all 0.15s ease",
                    }}
                  >
                    {isMember && <Check size={14} color="#fff" strokeWidth={3} />}
                  </div>

                  {/* Playlist details */}
                  <div style={{ flex: 1, minWidth: 0 }}>
                    <div
                      style={{
                        fontWeight: 600,
                        fontSize: "0.92rem",
                        color: isMember ? "#fff" : "var(--text-main)",
                        whiteSpace: "nowrap",
                        overflow: "hidden",
                        textOverflow: "ellipsis",
                      }}
                    >
                      {pl.name}
                    </div>
                    <div
                      style={{
                        fontSize: "0.76rem",
                        color: "var(--text-muted)",
                        marginTop: "1px",
                      }}
                    >
                      {pl.track_count !== undefined
                        ? `${pl.track_count} ${pl.track_count === 1 ? "song" : "songs"}`
                        : "Playlist"}
                      {pl.is_smart_mix === 1 ? " • Mix" : ""}
                    </div>
                  </div>

                  {/* Loading indicator if toggling */}
                  {isToggling && (
                    <span style={{ fontSize: "0.72rem", color: "var(--text-dim)" }}>
                      Saving...
                    </span>
                  )}
                </div>
              );
            })
          )}
        </div>

        {/* Footer info */}
        <div
          style={{
            padding: "12px 20px",
            borderTop: "1px solid rgba(255, 255, 255, 0.08)",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            fontSize: "0.78rem",
            color: "var(--text-dim)",
          }}
        >
          <span>
            {trackPlaylistIds.length === 0
              ? "Not in any playlist"
              : `In ${trackPlaylistIds.length} ${trackPlaylistIds.length === 1 ? "playlist" : "playlists"}`}
          </span>
          <button
            type="button"
            onClick={onClose}
            className="btn btn-secondary"
            style={{ padding: "5px 14px", fontSize: "0.8rem", borderRadius: "6px" }}
          >
            Done
          </button>
        </div>
      </div>
    </div>
  );
};
