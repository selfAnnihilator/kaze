import { Pencil, X } from "lucide-react";
import type { PlaylistsViewContext } from "./context";

export function RenamePlaylistModal({ c }: { c: PlaylistsViewContext }) {
  return (
    <>
      {c.playlistToRename && (
        <div
          className="modal-overlay"
          onClick={() => !c.isRenaming && c.setPlaylistToRename(null)}
        >
          <div
            className="modal-content"
            style={{ maxWidth: "440px", width: "90%", padding: "24px" }}
            onClick={(e) => e.stopPropagation()}
          >
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                marginBottom: "18px",
              }}
            >
              <div
                style={{ display: "flex", alignItems: "center", gap: "10px" }}
              >
                <Pencil size={20} color="var(--accent-light, var(--text-main))" />
                <h3 style={{ fontSize: "1.2rem", fontWeight: 700, margin: 0 }}>
                  Rename Playlist
                </h3>
              </div>
              <button
                onClick={() => c.setPlaylistToRename(null)}
                disabled={c.isRenaming}
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

            <form onSubmit={c.handleConfirmRename}>
              <div style={{ marginBottom: "16px" }}>
                <label
                  style={{
                    display: "block",
                    fontSize: "0.82rem",
                    color: "var(--text-dim)",
                    marginBottom: "8px",
                  }}
                >
                  Playlist Name
                </label>
                <input
                  type="text"
                  autoFocus
                  value={c.renameValue}
                  onChange={(e) => {
                    c.setRenameValue(e.target.value);
                    c.setRenameError(null);
                  }}
                  className="input-field"
                  style={{ width: "100%", fontSize: "0.95rem" }}
                  placeholder="Enter playlist name"
                />
                {c.renameError && (
                  <p
                    style={{
                      color: "var(--danger, var(--danger))",
                      fontSize: "0.8rem",
                      marginTop: "6px",
                    }}
                  >
                    {c.renameError}
                  </p>
                )}
              </div>

              <div
                style={{
                  display: "flex",
                  gap: "10px",
                  justifyContent: "flex-end",
                }}
              >
                <button
                  type="button"
                  className="btn btn-secondary"
                  onClick={() => c.setPlaylistToRename(null)}
                  disabled={c.isRenaming}
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="btn btn-primary"
                  disabled={c.isRenaming || !c.renameValue.trim()}
                >
                  {c.isRenaming ? "Saving..." : "Save Changes"}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </>
  );
}
