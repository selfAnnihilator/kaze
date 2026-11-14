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
        padding: "20px 8px 16px 8px",
        flexWrap: "wrap",
        gap: "14px",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: "18px" }}>
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
            color: "#e8d8c9",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            cursor: "pointer",
            boxShadow: "0 6px 20px rgba(139, 124, 246, 0.4)",
            transition: "transform 0.15s ease, background-color 0.15s ease",
          }}
          title={c.isPlaying ? "Pause Collection" : "Play Collection"}
        >
          {c.isPlaying ? (
            <Pause size={24} fill="#e8d8c9" />
          ) : (
            <Play size={24} fill="#e8d8c9" style={{ marginLeft: "3px" }} />
          )}
        </button>

        {/* Shuffle Button */}
        {c.onShuffleAll && (
          <button
            type="button"
            onClick={c.onShuffleAll}
            style={{
              background: c.isShuffled ? "rgba(139, 124, 246, 0.15)" : "none",
              border: c.isShuffled
                ? "1px solid rgba(139, 124, 246, 0.4)"
                : "1px solid transparent",
              borderRadius: "50%",
              width: "40px",
              height: "40px",
              color: c.isShuffled
                ? "var(--accent-secondary)"
                : "var(--text-muted)",
              cursor: "pointer",
              padding: "8px",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              transition: "transform 0.15s ease, opacity 0.15s ease",
              position: "relative",
            }}
            onMouseEnter={(e) => {
              if (!c.isShuffled) e.currentTarget.style.color = "#e8d8c9";
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

        {/* Already in Collection Bookmarked Icon (non-clickable) OR Save Button */}
        {c.isInCollection ? (
          <div
            style={{
              border: "1px solid rgba(139, 124, 246, 0.45)",
              backgroundColor: "rgba(139, 124, 246, 0.12)",
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
              border: "1px solid rgba(255, 255, 255, 0.2)",
              borderRadius: "50%",
              width: "40px",
              height: "40px",
              color: "var(--text-muted)",
              cursor: "pointer",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              transition: "transform 0.15s ease, opacity 0.15s ease",
            }}
            title="Save collection to your library"
          >
            <Plus size={18} />
          </button>
        ) : null}

        {c.collection.type === "playlist" && c.collection.playlistId && (
          <details style={{ position: "relative" }}>
            <summary
              aria-label={`Playlist options for ${c.collection.title}`}
              style={{
                listStyle: "none",
                border: "1px solid rgba(255, 255, 255, 0.2)",
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
                zIndex: 20,
                minWidth: "160px",
                padding: "6px",
                borderRadius: "8px",
                border: "1px solid var(--border)",
                background: "var(--bg-sidebar)",
                boxShadow: "0 12px 30px rgba(0,0,0,0.45)",
              }}
            >
              {c.isLikedSongs ? (
                <div
                  style={{
                    padding: "8px 10px",
                    color: "var(--text-dim)",
                    fontSize: "0.78rem",
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
                      color: "var(--danger, #ef4444)",
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
              padding: "8px 30px 8px 34px",
              borderRadius: "20px",
              border: "1px solid var(--border)",
              backgroundColor: "rgba(255, 255, 255, 0.05)",
              color: "#e8d8c9",
              fontSize: "0.82rem",
              outline: "none",
              width: "220px",
              transition: "transform 0.2s ease, opacity 0.2s ease",
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
      )}
    </div>
  );
}
