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
            style={{ maxWidth: "440px", width: "90%", padding: "var(--space-24)" }}
            onClick={(e) => e.stopPropagation()}
          >
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "center",
                marginBottom: "var(--space-18)",
              }}
            >
              <div
                style={{ display: "flex", alignItems: "center", gap: "var(--space-10)" }}
              >
                <Pencil size={20} color="var(--accent-light, var(--text-main))" />
                <h3 style={{ fontSize: "var(--fs-20)", fontWeight: "var(--weight-bold)", margin: 0 }}>
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
              <div style={{ marginBottom: "var(--space-16)" }}>
                <label
                  style={{
                    display: "block",
                    fontSize: "var(--fs-13)",
                    color: "var(--text-dim)",
                    marginBottom: "var(--space-8)",
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
                  style={{ width: "100%", fontSize: "var(--fs-15)" }}
                  placeholder="Enter playlist name"
                />
                {c.renameError && (
                  <p
                    style={{
                      color: "var(--danger, var(--danger))",
                      fontSize: "var(--fs-13)",
                      marginTop: "var(--space-6)",
                    }}
                  >
                    {c.renameError}
                  </p>
                )}
              </div>

              <div
                style={{
                  display: "flex",
                  gap: "var(--space-10)",
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
