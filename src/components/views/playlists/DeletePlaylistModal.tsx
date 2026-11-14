import { Trash2 } from "lucide-react";
import type { PlaylistsViewContext } from "./context";

export function DeletePlaylistModal({ c }: { c: PlaylistsViewContext }) {
  return (
    <>
      {c.playlistToDelete && (
        <div
          className="modal-overlay"
          onClick={() => !c.isDeleting && c.setPlaylistToDelete(null)}
        >
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
            <div
              style={{
                display: "flex",
                alignItems: "flex-start",
                gap: "14px",
                marginBottom: "16px",
              }}
            >
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
                <h3
                  style={{
                    fontSize: "1.15rem",
                    fontWeight: 700,
                    margin: "0 0 6px 0",
                    color: "#fca5a5",
                  }}
                >
                  Delete Playlist
                </h3>
                <p
                  style={{
                    fontSize: "0.88rem",
                    color: "var(--text-dim)",
                    margin: 0,
                    lineHeight: 1.5,
                  }}
                >
                  Are you sure you want to delete{" "}
                  <strong style={{ color: "#fff" }}>
                    "{c.playlistToDelete.name}"
                  </strong>
                  ? This action cannot be undone.
                </p>
              </div>
            </div>

            <div
              style={{
                display: "flex",
                gap: "10px",
                justifyContent: "flex-end",
                marginTop: "20px",
              }}
            >
              <button
                type="button"
                className="btn btn-secondary"
                onClick={() => c.setPlaylistToDelete(null)}
                disabled={c.isDeleting}
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
                onClick={c.handleConfirmDelete}
                disabled={c.isDeleting}
              >
                <Trash2 size={15} />
                <span>{c.isDeleting ? "Deleting..." : "Delete Playlist"}</span>
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
