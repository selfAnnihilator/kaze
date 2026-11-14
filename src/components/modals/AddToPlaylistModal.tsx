import { WindowedList } from "../../presentation/WindowedList";
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
  onTogglePlaylist: (playlistId: string, isCurrentlyMember: boolean,
  ) => Promise<void>;
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
  const [membershipError, setMembershipError] = useState<string | null>(null);
  const pending = React.useRef(new Set<string>());
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
      if (!createdId) return;
      // Creation succeeded. A membership failure must not retry creation.
      try { await onTogglePlaylist(createdId, false); }
      catch (failure) { setMembershipError(`Playlist created. Couldn’t add song: ${failure instanceof Error ? failure.message : String(failure)}. Select the playlist to retry.`); }
      setNewPlaylistName("");
      setShowCreateForm(false);
    } catch (err: any) {
      setCreateError(err?.message || "Failed to create playlist");
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleToggle = async (playlistId: string) => {
    if (pending.current.has(playlistId)) return;
    pending.current.add(playlistId);
    setMembershipError(null);
    const isMember = trackPlaylistIds.includes(playlistId);

    setTogglingIds((prev) => new Set(prev).add(playlistId));
    try {
      await onTogglePlaylist(playlistId, isMember);
    } catch (failure) {
      setMembershipError(`Couldn’t update playlist. ${failure instanceof Error ? failure.message : String(failure)}. Select it to retry.`);
    } finally {
      pending.current.delete(playlistId);
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
        backgroundColor: "var(--shade-75)",
        backdropFilter: "blur(6px)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: "var(--z-dialog)",
        padding: "var(--space-20)",
      }}
      onClick={() => { if (!isSubmitting && !togglingIds.size) onClose(); }}
    >
      <div
        className="modal-content"
        style={{
          backgroundColor: "var(--palette-gray-900)",
          border: "1px solid var(--overlay-12)",
          borderRadius: "var(--radius-16)",
          width: "100%",
          maxWidth: "460px",
          maxHeight: "85vh",
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
          boxShadow: "0 20px 50px var(--shade-60)",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div
          style={{
            padding: "var(--space-18) var(--space-20)",
            borderBottom: "1px solid var(--overlay-8)",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            gap: "var(--space-12)",
          }}
        >
          <div style={{ minWidth: 0, flex: 1 }}>
            <h2 style={{ fontSize: "var(--fs-18)", fontWeight: "var(--weight-bold)", color: "var(--text-main)", margin: 0,
              }}>
              Add to Playlist
            </h2>
            <p
              style={{
                fontSize: "var(--fs-13)",
                color: "var(--text-muted)",
                marginTop: "var(--space-2)",
                whiteSpace: "nowrap",
                overflow: "hidden",
                textOverflow: "ellipsis",
              }}
            >
              {track.title} {track.artist ? `• ${track.artist}` : ""}
            </p>
          </div>

          <div style={{ display: "flex", alignItems: "center", gap: "var(--space-8)" }}>
            {!showCreateForm && (
              <button
                type="button"
                className="btn btn-primary"
                onClick={() => {
                  setShowCreateForm(true);
                  setCreateError(null);
                }}
                style={{
                  fontSize: "var(--fs-12)",
                  padding: "var(--space-6) var(--space-12)",
                  borderRadius: "var(--radius-8)",
                  display: "inline-flex",
                  alignItems: "center",
                  gap: "var(--space-6)",
                  backgroundColor: "var(--accent)",
                  fontWeight: "var(--weight-semibold)",
                }}
                title="Create a new playlist"
              >
                <Plus size={15} />
                <span>New Playlist</span>
              </button>
            )}

            <button
              type="button"
              onClick={() => { if (!isSubmitting && !togglingIds.size) onClose(); }}
              style={{
                background: "none",
                border: "none",
                color: "var(--text-dim)",
                cursor: "pointer",
                padding: "var(--space-6)",
                display: "flex",
                borderRadius: "var(--radius-6)",
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
              padding: "var(--space-14) var(--space-20)",
              backgroundColor: "var(--overlay-3)",
              borderBottom: "1px solid var(--overlay-8)",
            }}
          >
            <div style={{ display: "flex", gap: "var(--space-8)" }}>
              <input
                type="text"
                autoFocus
                disabled={isSubmitting}
                placeholder="Enter playlist name..."
                value={newPlaylistName}
                onChange={(e) => {
                  setNewPlaylistName(e.target.value);
                  if (createError) setCreateError(null);
                }}
                style={{
                  flex: 1,
                  backgroundColor: "var(--shade-40)",
                  border: createError
                    ? "1px solid var(--danger)"
                    : "1px solid var(--overlay-15)",
                  borderRadius: "var(--radius-8)",
                  padding: "var(--space-8) var(--space-12)",
                  color: "var(--text-main)",
                  fontSize: "var(--fs-14)",
                  outline: "none",
                }}
              />
              <button
                type="submit"
                className="btn btn-primary"
                disabled={!newPlaylistName.trim() || isSubmitting}
                style={{
                  padding: "var(--space-8) var(--space-14)",
                  fontSize: "var(--fs-13)",
                  fontWeight: "var(--weight-semibold)",
                  opacity: !newPlaylistName.trim() || isSubmitting ? 0.6 : 1,
                }}
              >
                {isSubmitting ? "Creating..." : "Create"}
              </button>
              <button
                type="button"
                className="btn btn-secondary"
                disabled={isSubmitting}
                onClick={() => {
                  setShowCreateForm(false);
                  setCreateError(null);
                  setNewPlaylistName("");
                }}
                style={{ padding: "var(--space-8) var(--space-12)", fontSize: "var(--fs-13)" }}
              >
                Cancel
              </button>
            </div>

            {createError && (
              <div
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: "var(--space-6)",
                  marginTop: "var(--space-8)",
                  fontSize: "var(--fs-12)",
                  color: "var(--danger)",
                }}
              >
                <AlertCircle size={14} />
                <span role="alert">{createError}</span>
              </div>
            )}
          </form>
        )}

        {membershipError && <p role="alert">{membershipError}</p>}
        {/* Playlists List */}
        <div
          style={{
            flex: 1,
            overflowY: "auto",
            padding: "var(--space-10) var(--space-14)",
            display: "flex",
            flexDirection: "column",
            gap: "var(--space-4)",
          }}
        >
          {userPlaylists.length === 0 ? (
            <div
              style={{
                textAlign: "center",
                padding: "var(--space-40) var(--space-20)",
                color: "var(--text-dim)",
              }}
            >
              <ListMusic
                size={36}
                color="var(--text-dim)"
                style={{ marginBottom: "var(--space-10)" }}
              />
              <p
                style={{
                  fontSize: "var(--fs-15)",
                  fontWeight: "var(--weight-semibold)",
                  color: "var(--text-main)",
                  marginBottom: "var(--space-4)",
                }}
              >
                No playlists available
              </p>
              <p style={{ fontSize: "var(--fs-13)", color: "var(--text-muted)" }}>
                Click &ldquo;+ New Playlist&rdquo; above to create your first
                playlist.
              </p>
            </div>
          ) : (
            <WindowedList
              items={userPlaylists}
              label="Available playlists"
              rowHeight={64}
              renderItem={(pl) => {
                const isMember = trackPlaylistIds.includes(pl.id);
                const isToggling = togglingIds.has(pl.id);

                return (
                  <div
                    key={pl.id}
                    onClick={() => handleToggle(pl.id)}
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: "var(--space-14)",
                      padding: "var(--space-8) var(--space-12)",
                      borderRadius: "var(--radius-10)",
                      cursor: "pointer",
                      backgroundColor: isMember
                        ? "var(--accent-a8)"
                        : "transparent",
                      border: isMember
                        ? "1px solid var(--accent-a30)"
                        : "1px solid transparent",
                      transition: "transform var(--duration-fast) ease, opacity var(--duration-fast) ease",
                    }}
                    onMouseEnter={(e) => {
                      if (!isMember) {
                        e.currentTarget.style.backgroundColor =
                          "var(--overlay-5)";
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
                        borderRadius: "var(--radius-6)",
                        border: isMember
                          ? "2px solid var(--accent-secondary)"
                          : "2px solid var(--overlay-30)",
                        backgroundColor: isMember
                          ? "var(--accent-secondary)"
                          : "transparent",
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "center",
                        flexShrink: 0,
                        transition: "transform var(--duration-fast) ease, opacity var(--duration-fast) ease",
                      }}
                    >
                      {isMember && (
                        <Check size={14} color="var(--text-main)" strokeWidth={3} />
                      )}
                    </div>

                    {/* Playlist details */}
                    <div style={{ flex: 1, minWidth: 0 }}>
                      <div
                        style={{
                          fontWeight: "var(--weight-semibold)",
                          fontSize: "var(--fs-15)",
                          color: isMember ? "var(--text-main)" : "var(--text-main)",
                          whiteSpace: "nowrap",
                          overflow: "hidden",
                          textOverflow: "ellipsis",
                        }}
                      >
                        {pl.name}
                      </div>
                      <div
                        style={{
                          fontSize: "var(--fs-12)",
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
                      <span
                        style={{
                          fontSize: "var(--fs-12)",
                          color: "var(--text-dim)",
                        }}
                      >
                        Saving...
                      </span>
                    )}
                  </div>
                );
              }}
            />
          )}
        </div>

        {/* Footer info */}
        <div
          style={{
            padding: "var(--space-12) var(--space-20)",
            borderTop: "1px solid var(--overlay-8)",
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            fontSize: "var(--fs-12)",
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
            onClick={() => { if (!isSubmitting && !togglingIds.size) onClose(); }}
            className="btn btn-secondary"
            style={{
              padding: "var(--space-6) var(--space-14)",
              fontSize: "var(--fs-13)",
              borderRadius: "var(--radius-6)",
            }}
          >
            Done
          </button>
        </div>
      </div>
    </div>
  );
};
