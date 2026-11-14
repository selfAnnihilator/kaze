import { WindowedTable } from "../../../presentation/WindowedTable";
import { Check, ListPlus, Play, Plus, RefreshCw, X } from "lucide-react";
import type { PlaylistsViewContext } from "./context";

export function PlaylistDetailsModal({ c }: { c: PlaylistsViewContext }) {
  const selectedPlaylist = c.selectedPlaylist;
  if (!selectedPlaylist) return null;
  return (
    <>
      {selectedPlaylist && (
        <div
          className="modal-overlay"
          onClick={() => c.setSelectedPlaylist(null)}
        >
          <div
            className="modal-content"
            style={{
              maxWidth: "720px",
              width: "95%",
              maxHeight: "85vh",
              display: "flex",
              flexDirection: "column",
            }}
            onClick={(e) => e.stopPropagation()}
          >
            <div
              style={{
                display: "flex",
                justifyContent: "space-between",
                alignItems: "flex-start",
                marginBottom: "16px",
              }}
            >
              <div
                style={{ display: "flex", alignItems: "center", gap: "14px" }}
              >
                {selectedPlaylist.cover_art_url && (
                  <img decoding="async"
                    src={selectedPlaylist.cover_art_url}
                    alt={selectedPlaylist.name}
                    style={{
                      width: "56px",
                      height: "56px",
                      borderRadius: "8px",
                      objectFit: "cover",
                      flexShrink: 0,
                    }}
                  />
                )}
                <div>
                  <div
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: "8px",
                    }}
                  >
                    <h3 style={{ fontSize: "var(--fs-20)", fontWeight: "var(--weight-bold)" }}>
                      {selectedPlaylist.name}
                    </h3>
                    {selectedPlaylist.is_smart_mix === 1 && (
                      <span className="badge badge-exact">Smart Mix</span>
                    )}
                  </div>
                  <p
                    style={{
                      fontSize: "var(--fs-14)",
                      color: "var(--text-dim)",
                      marginTop: "4px",
                    }}
                  >
                    {selectedPlaylist.description ||
                      selectedPlaylist.generation_reason ||
                      "Custom playlist"}
                  </p>
                </div>
              </div>

              <button
                onClick={() => c.setSelectedPlaylist(null)}
                style={{
                  background: "none",
                  border: "none",
                  color: "var(--text-muted)",
                  cursor: "pointer",
                  padding: "4px",
                }}
              >
                <X size={20} />
              </button>
            </div>

            <div
              style={{
                display: "flex",
                gap: "10px",
                alignItems: "center",
                marginBottom: "16px",
              }}
            >
              <button
                className="btn btn-primary"
                onClick={() => {
                  c.onPlayPlaylist(selectedPlaylist.id);
                  c.setSelectedPlaylist(null);
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
                    backgroundColor: c.savedMixIds.has(selectedPlaylist.id)
                      ? "var(--accent-secondary-a15)"
                      : undefined,
                    borderColor: c.savedMixIds.has(selectedPlaylist.id)
                      ? "var(--accent-secondary)"
                      : undefined,
                    color: c.savedMixIds.has(selectedPlaylist.id)
                      ? "var(--accent-secondary)"
                      : undefined,
                  }}
                  onClick={() => c.handleSaveMixToUserPlaylist(selectedPlaylist)}
                  disabled={c.savingMixId === selectedPlaylist.id}
                >
                  {c.savedMixIds.has(selectedPlaylist.id) ? (
                    <>
                      <Check size={14} color="var(--accent-secondary)" />
                      <span>Saved to Playlists</span>
                    </>
                  ) : (
                    <>
                      <Plus size={14} />
                      <span>
                        {c.savingMixId === selectedPlaylist.id
                          ? "Saving..."
                          : "Add to Playlists"}
                      </span>
                    </>
                  )}
                </button>
              )}

              <span
                style={{
                  fontSize: "var(--fs-14)",
                  color: "var(--text-dim)",
                  marginLeft: "auto",
                }}
              >
                {c.playlistTracks.length} tracks
              </span>
            </div>

            {c.loadingTracks ? (
              <div
                style={{
                  padding: "40px 0",
                  textAlign: "center",
                  color: "var(--text-muted)",
                }}
              >
                Loading tracks...
              </div>
            ) : c.playlistTracks.length === 0 ? (
              <div
                style={{
                  padding: "40px 0",
                  textAlign: "center",
                  color: "var(--text-dim)",
                }}
              >
                No tracks currently in this playlist.
              </div>
            ) : (
              <div style={{ overflowY: "auto", flex: 1 }}>
                <WindowedTable
                  className="track-table"
                  style={{ width: "100%" }}
                  items={c.playlistTracks}
                  rowHeight={72}
                  renderItem={(tr, idx) => (
                    <tr
                      key={tr.id}
                      className="track-row"
                      onDoubleClick={() => c.onPlayTrack(tr.id)}
                      style={{
                        backgroundColor:
                          c.loadingTrackId === tr.id
                            ? "var(--accent-a12)"
                            : undefined,
                      }}
                    >
                      <td
                        style={{
                          textAlign: "center",
                          color: "var(--text-dim)",
                        }}
                      >
                        {idx + 1}
                      </td>
                      <td className="primary">{tr.title}</td>
                      <td>{tr.artist_name || "Unknown Artist"}</td>
                      <td
                        style={{ textAlign: "right", fontFamily: "var(--font-mono)" }}
                      >
                        {c.formatSeconds(tr.duration_secs)}
                      </td>
                      <td>
                        <div
                          style={{
                            display: "flex",
                            justifyContent: "center",
                            gap: "6px",
                          }}
                        >
                          <button
                            className="player-icon-btn"
                            title="Play Track"
                            onClick={() => c.onPlayTrack(tr.id)}
                          >
                            {c.loadingTrackId === tr.id ? (
                              <RefreshCw
                                size={14}
                                className="spin-animation"
                                aria-label="Loading song"
                              />
                            ) : (
                              <Play size={14} />
                            )}
                          </button>
                          {(() => {
                            const isEnqueued = c.queuedTrackIds
                              ? c.queuedTrackIds.has(tr.id)
                              : false;
                            return (
                              <button
                                className="player-icon-btn"
                                title={
                                  isEnqueued
                                    ? "Already in queue"
                                    : "Add to queue"
                                }
                                onClick={() => c.onEnqueueTrack(tr.id)}
                                style={{
                                  color: isEnqueued
                                    ? "var(--success)"
                                    : "var(--text-muted)",
                                }}
                              >
                                {isEnqueued ? (
                                  <Check size={14} />
                                ) : (
                                  <ListPlus size={14} />
                                )}
                              </button>
                            );
                          })()}
                        </div>
                      </td>
                    </tr>
                  )}
                >
                  <thead>
                    <tr>
                      <th style={{ width: "40px", textAlign: "center" }}>#</th>
                      <th>Title</th>
                      <th>Artist</th>
                      <th style={{ width: "80px", textAlign: "right" }}>
                        Length
                      </th>
                      <th style={{ width: "80px", textAlign: "center" }}>
                        Actions
                      </th>
                    </tr>
                  </thead>
                </WindowedTable>
              </div>
            )}
          </div>
        </div>
      )}
    </>
  );
}
