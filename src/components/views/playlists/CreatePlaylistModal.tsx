import { X } from "lucide-react";
import type { PlaylistsViewContext } from "./context";

export function CreatePlaylistModal({ c }: { c: PlaylistsViewContext }) {
  return (
    <>
      {c.showCreateModal && (
        <div className="modal-overlay">
          <div className="modal-content" style={{ maxWidth: "420px" }}>
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                marginBottom: "16px",
              }}
            >
              <h3 style={{ fontSize: "var(--fs-20)", fontWeight: "var(--weight-bold)" }}>
                Create New Playlist
              </h3>
              <button
                disabled={c.creating}
                onClick={() => c.setShowCreateModal(false)}
                style={{
                  background: "none",
                  border: "none",
                  color: "var(--text-muted)",
                  cursor: "pointer",
                }}
              >
                <X size={18} />
              </button>
            </div>

            <form onSubmit={c.handleCreate} aria-busy={c.creating}>
              {c.createError && <p role="alert">Couldn’t create playlist. {c.createError}</p>}
              <input
                type="text"
                disabled={c.creating}
                placeholder="Playlist name"
                value={c.playlistName}
                onChange={(e) => c.setPlaylistName(e.target.value)}
                className="input-field"
                style={{ width: "100%", marginBottom: "20px" }}
                autoFocus
              />
              <div
                style={{
                  display: "flex",
                  gap: "12px",
                  justifyContent: "flex-end",
                }}
              >
                <button
                  type="button"
                  className="btn btn-secondary"
                  disabled={c.creating}
                onClick={() => c.setShowCreateModal(false)}
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="btn btn-primary"
                  disabled={c.creating || !c.playlistName.trim()}
                >
                  Create Playlist
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </>
  );
}
