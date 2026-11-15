import { BookmarkCheck, MoreVertical, Pause, Pencil, Play, Plus, Search, Shuffle, Trash2, X } from "lucide-react";
import type { CollectionDetailViewContext } from "./context";

export function CollectionToolbar({ c }: { c: CollectionDetailViewContext }) {
  return (
    <div
      className="collection-toolbar"
      style={{
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        padding: "var(--space-20) var(--space-8) var(--space-16) var(--space-8)",
        flexWrap: "wrap",
        gap: "var(--space-14)",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: "var(--space-18)" }}>
        {/* Big Green Play / Pause Button */}
        <button
          type="button"
          onClick={c.onPlayAll}
          style={{
            width: "54px",
            height: "54px",
            borderRadius: "50%",
            backgroundColor: "var(--accent-secondary)",
            border: "none",
            color: "var(--text-main)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            cursor: "pointer",
            boxShadow: "0 6px 20px var(--accent-secondary-a40)",
            transition: "transform var(--duration-fast) ease, background-color var(--duration-fast) ease",
          }}
          title={c.isPlaying ? "Pause Collection" : "Play Collection"}
        >
          {c.isPlaying ? (
            <Pause size={24} fill="var(--text-main)" />
          ) : (
            <Play size={24} fill="var(--text-main)" style={{ marginLeft: "var(--space-4)" }} />
          )}
        </button>

        {/* Shuffle Button */}
        {c.onShuffleAll && (
          <button
            type="button"
            onClick={c.onShuffleAll}
            style={{
              background: c.isShuffled ? "var(--accent-secondary-a15)" : "none",
              border: c.isShuffled
                ? "1px solid var(--accent-secondary-a40)"
                : "1px solid transparent",
              borderRadius: "50%",
              width: "40px",
              height: "40px",
              color: c.isShuffled
                ? "var(--accent-secondary)"
                : "var(--text-muted)",
              cursor: "pointer",
              padding: "var(--space-8)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              transition: "transform var(--duration-fast) ease, opacity var(--duration-fast) ease",
              position: "relative",
            }}
            onMouseEnter={(e) => {
              if (!c.isShuffled) e.currentTarget.style.color = "var(--text-main)";
            }}
            onMouseLeave={(e) => {
              if (!c.isShuffled)
                e.currentTarget.style.color = "var(--text-muted)";
            }}
            title={
              c.isShuffled
                ? "Shuffle is active (Click to deactivate)"
                : "Shuffle collection"
            }
          >
            <Shuffle
              size={20}
              color={c.isShuffled ? "var(--accent-secondary)" : undefined}
            />
            {c.isShuffled && (
              <span
                style={{
                  position: "absolute",
                  bottom: "3px",
                  width: "4px",
                  height: "4px",
                  borderRadius: "50%",
                  backgroundColor: "var(--accent-secondary)",
                }}
              />
            )}
          </button>
        )}

        <SaveBadge c={c} />

        {c.collection.type === "playlist" && c.collection.playlistId && (
          <details style={{ position: "relative" }}>
            <summary
              aria-label={`Playlist options for ${c.collection.title}`}
              style={{
                listStyle: "none",
                border: "1px solid var(--overlay-20)",
                borderRadius: "50%",
                width: "40px",
                height: "40px",
                color: "var(--text-muted)",
                cursor: "pointer",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
              }}
            >
              <MoreVertical size={20} />
            </summary>
            <div
              style={{
                position: "absolute",
                top: "46px",
                left: 0,
                zIndex: "var(--z-dropdown)",
                minWidth: "160px",
                padding: "var(--space-6)",
                borderRadius: "var(--radius-8)",
                border: "1px solid var(--border)",
                background: "var(--bg-sidebar)",
                boxShadow: "0 12px 30px var(--shade-45)",
              }}
            >
              {c.isLikedSongs ? (
                <div
                  style={{
                    padding: "var(--space-8) var(--space-10)",
                    color: "var(--text-dim)",
                    fontSize: "var(--fs-12)",
                  }}
                >
                  Default playlist
                </div>
              ) : (
                <>
                  <button
                    className="btn btn-secondary"
                    style={{
                      width: "100%",
                      justifyContent: "flex-start",
                      border: 0,
                    }}
                    onClick={c.handleRenamePlaylist}
                  >
                    <Pencil size={14} /> Rename
                  </button>
                  <button
                    className="btn btn-secondary"
                    style={{
                      width: "100%",
                      justifyContent: "flex-start",
                      border: 0,
                      color: "var(--danger, var(--danger))",
                    }}
                    onClick={c.handleDeletePlaylist}
                  >
                    <Trash2 size={14} /> Delete
                  </button>
                </>
              )}
            </div>
          </details>
        )}
      </div>

      {/* Search inside Playlist */}
      {c.rawTracks.length > 0 && (
        <div
          style={{
            display: "flex",
            alignItems: "center",
            position: "relative",
          }}
        >
          <Search
            size={15}
            color="var(--text-muted)"
            style={{
              position: "absolute",
              left: "12px",
              pointerEvents: "none",
            }}
          />
          <input
            type="text"
            placeholder="Search in playlist..."
            value={c.searchQuery}
            onChange={(e) => c.setSearchQuery(e.target.value)}
            style={{
              padding: "var(--space-8) var(--space-30) var(--space-8) var(--space-36)",
              borderRadius: "var(--radius-20)",
              border: "1px solid var(--border)",
              backgroundColor: "var(--overlay-5)",
              color: "var(--text-main)",
              fontSize: "var(--fs-13)",
              outline: "none",
              width: "220px",
              transition: "transform var(--duration-base) ease, opacity var(--duration-base) ease",
            }}
          />
          {c.searchQuery && (
            <button
              type="button"
              onClick={() => c.setSearchQuery("")}
              style={{
                position: "absolute",
                right: "10px",
                background: "none",
                border: "none",
                color: "var(--text-muted)",
                cursor: "pointer",
                padding: "var(--space-2)",
                display: "flex",
                alignItems: "center",
              }}
              title="Clear search"
            >
              <X size={13} />
            </button>
          )}
        </div>
      )}
    </div>
  );
}

/** The save button, or a badge when the collection is already yours. Liked Songs is always yours, so it shows neither. */
function SaveBadge({ c }: { c: CollectionDetailViewContext }) {
  if (c.isLikedSongs) return null;
  return c.isInCollection ? (
    <div
      style={{
        border: "1px solid var(--accent-secondary-a45)",
        backgroundColor: "var(--accent-secondary-a12)",
        borderRadius: "50%",
        width: "40px",
        height: "40px",
        color: "var(--accent-secondary)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        cursor: "default",
        userSelect: "none",
      }}
      title="Already in collection"
    >
      <BookmarkCheck size={20} />
    </div>
  ) : c.onSaveToPlaylists ? (
    <button
      type="button"
      onClick={() => c.onSaveToPlaylists?.(c.collection)}
      style={{
        background: "none",
        border: "1px solid var(--overlay-20)",
        borderRadius: "50%",
        width: "40px",
        height: "40px",
        color: "var(--text-muted)",
        cursor: "pointer",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        transition: "transform var(--duration-fast) ease, opacity var(--duration-fast) ease",
      }}
      title="Save collection to your library"
    >
      <Plus size={18} />
    </button>
  ) : null;
}
