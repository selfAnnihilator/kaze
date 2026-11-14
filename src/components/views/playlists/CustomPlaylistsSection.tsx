import { WindowedGallery } from "../../../presentation/WindowedGallery";
import { Heart, ListMusic, MoreVertical, Pencil, Play, Plus, Share2, Trash2 } from "lucide-react";
import type { PlaylistsViewContext } from "./context";

export function CustomPlaylistsSection({ c }: { c: PlaylistsViewContext }) {
  return (
    <>
      {(c.viewMode === "playlists" || c.viewMode === "all") && (
        <div>
          {c.viewMode === "all" && (
            <div
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                marginBottom: "var(--space-16)",
              }}
            >
              <h2 style={{ fontSize: "var(--fs-18)", fontWeight: "var(--weight-semibold)" }}>
                Your Custom Playlists
              </h2>
              <span style={{ fontSize: "var(--fs-13)", color: "var(--text-dim)" }}>
                {c.customPlaylists.length} custom{" "}
                {c.customPlaylists.length === 1 ? "playlist" : "playlists"}
              </span>
            </div>
          )}

          {!c.currentUser ? (
            <div
              style={{
                padding: "var(--space-48) var(--space-20)",
                borderRadius: "var(--radius-10)",
                backgroundColor: "var(--bg-card)",
                border: "1px solid var(--border)",
                textAlign: "center",
                color: "var(--text-dim)",
              }}
            >
              <ListMusic
                size={40}
                color="var(--text-dim)"
                style={{ margin: "0 auto var(--space-12)" }}
              />
              <div
                style={{
                  fontWeight: "var(--weight-semibold)",
                  fontSize: "var(--fs-16)",
                  color: "var(--text-main)",
                  marginBottom: "var(--space-6)",
                }}
              >
                Sign In to Manage Playlists
              </div>
              <p style={{ fontSize: "var(--fs-14)", marginBottom: "var(--space-20)" }}>
                Sign in to your account to create, customize, and synchronize
                your personal playlists across devices.
              </p>
              {c.onOpenAuthModal && (
                <div
                  style={{
                    display: "flex",
                    justifyContent: "center",
                    gap: "var(--space-12)",
                  }}
                >
                  <button className="btn btn-primary" onClick={c.onOpenAuthModal}>
                    <span>Sign In / Sign Up</span>
                  </button>
                </div>
              )}
            </div>
          ) : c.customPlaylists.length === 0 ? (
            <div
              style={{
                padding: "var(--space-48) var(--space-20)",
                borderRadius: "var(--radius-10)",
                backgroundColor: "var(--bg-card)",
                border: "1px solid var(--border)",
                textAlign: "center",
                color: "var(--text-dim)",
              }}
            >
              <ListMusic
                size={40}
                color="var(--text-dim)"
                style={{ margin: "0 auto var(--space-12)" }}
              />
              <div
                style={{
                  fontWeight: "var(--weight-semibold)",
                  fontSize: "var(--fs-16)",
                  color: "var(--text-main)",
                  marginBottom: "var(--space-6)",
                }}
              >
                No custom playlists yet
              </div>
              <p style={{ fontSize: "var(--fs-14)", marginBottom: "var(--space-20)" }}>
                Create custom playlists manually, save a Smart Mix as a
                permanent playlist, or import a playlist link.
              </p>
              <div
                style={{
                  display: "flex",
                  justifyContent: "center",
                  gap: "var(--space-12)",
                }}
              >
                <button
                  className="btn btn-primary"
                  onClick={() => c.setShowCreateModal(true)}
                >
                  <Plus size={16} />
                  <span>Create Playlist</span>
                </button>
                <button
                  className="btn btn-secondary"
                  onClick={() => c.setShowJioSaavnModal(true)}
                >
                  <Share2 size={16} color="var(--accent-secondary)" />
                  <span>Import playlist</span>
                </button>
              </div>
            </div>
          ) : (
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(auto-fill, minmax(220px, 1fr))",
                gap: "var(--space-18)",
              }}
            >
              <WindowedGallery
                items={c.customPlaylists}
                label="customPlaylists"
                layout={c.interfaceStyle === "spatial" ? "rows" : "index"}
                renderItem={(pl, index) => (
                  <div
                    key={pl.id}
                    className="playlist-card"
                    role="button"
                    tabIndex={0}
                    onKeyDown={(event) => {
                      if (event.target !== event.currentTarget || (event.key !== "Enter" && event.key !== " ")) return;
                      event.preventDefault();
                      c.handleOpenPlaylistDetails(pl);
                    }}
                    onClick={() => c.handleOpenPlaylistDetails(pl)}
                    style={{
                      backgroundColor: "var(--bg-card)",
                      borderRadius: "var(--radius-12)",
                      padding: "var(--space-16)",
                      border: "1px solid var(--border)",
                      cursor: "pointer",
                      display: "flex",
                      flexDirection: "column",
                      justifyContent: "space-between",
                      transition:
                        "background-color var(--duration-fast), border-color var(--duration-fast), transform var(--duration-fast)",
                    }}
                    onMouseEnter={(e) => {
                      e.currentTarget.style.backgroundColor =
                        "var(--bg-card-hover)";
                      e.currentTarget.style.borderColor = "var(--accent-light)";
                    }}
                    onMouseLeave={(e) => {
                      e.currentTarget.style.backgroundColor = "var(--bg-card)";
                      e.currentTarget.style.borderColor = "var(--border)";
                    }}
                    title="Click to view tracks in this playlist"
                  >
                    {c.interfaceStyle === "brutalist" && <span className="press-playlist-number">{String(index + 1).padStart(2, "0")}</span>}
                    <div>
                      <div
                        style={{
                          position: "relative",
                          width: "100%",
                          aspectRatio: "1/1",
                          borderRadius: "var(--radius-8)",
                          backgroundColor: "var(--bg-sidebar)",
                          display: "flex",
                          alignItems: "center",
                          justifyContent: "center",
                          marginBottom: "var(--space-12)",
                          overflow: "hidden",
                        }}
                      >
                        <details
                          style={{
                            position: "absolute",
                            top: "8px",
                            right: "8px",
                            zIndex: 4,
                          }}
                          onClick={(e) => e.stopPropagation()}
                        >
                          <summary
                            className="player-icon-btn"
                            aria-label={`Playlist options for ${pl.name}`}
                            style={{
                              listStyle: "none",
                              width: "32px",
                              height: "32px",
                              display: "flex",
                              alignItems: "center",
                              justifyContent: "center",
                              borderRadius: "50%",
                              background: "var(--shade-70)",
                              color: "var(--text-main)",
                              cursor: "pointer",
                            }}
                          >
                            <MoreVertical size={18} />
                          </summary>
                          <div
                            style={{
                              position: "absolute",
                              top: "38px",
                              right: 0,
                              minWidth: "150px",
                              padding: "var(--space-6)",
                              borderRadius: "var(--radius-8)",
                              border: "1px solid var(--border)",
                              background: "var(--bg-sidebar)",
                              boxShadow: "0 12px 30px var(--shade-45)",
                            }}
                          >
                            {pl.name === "Liked Songs" ? (
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
                                  onClick={(e) => {
                                    e.stopPropagation();
                                    (
                                      e.currentTarget.closest(
                                        "details",
                                      ) as HTMLDetailsElement
                                    )?.removeAttribute("open");
                                    c.handleOpenRenameModal(pl);
                                  }}
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
                                  onClick={(e) => {
                                    e.stopPropagation();
                                    (
                                      e.currentTarget.closest(
                                        "details",
                                      ) as HTMLDetailsElement
                                    )?.removeAttribute("open");
                                    c.handleOpenDeleteModal(pl);
                                  }}
                                >
                                  <Trash2 size={14} /> Delete
                                </button>
                              </>
                            )}
                          </div>
                        </details>
                        {pl.name === "Liked Songs" ? (
                          <div
                            style={{
                              width: "100%",
                              height: "100%",
                              display: "flex",
                              alignItems: "center",
                              justifyContent: "center",
                              background:
                                "linear-gradient(145deg, var(--palette-pink-400-a30), var(--accent-secondary-a20))",
                            }}
                          >
                            <Heart
                              size={82}
                              color="var(--palette-pink-300)"
                              fill="var(--palette-pink-400)"
                              strokeWidth={1.5}
                            />
                          </div>
                        ) : (
                          <img decoding="async"
                            src={
                              pl.cover_art_url || "/kaze-playlist-default.svg"
                            }
                            alt={pl.name}
                            style={{
                              width: "100%",
                              height: "100%",
                              objectFit: "cover",
                            }}
                          />
                        )}
                      </div>
                      <div
                        style={{
                          fontWeight: "var(--weight-semibold)",
                          fontSize: "var(--fs-16)",
                          whiteSpace: "nowrap",
                          overflow: "hidden",
                          textOverflow: "ellipsis",
                        }}
                      >
                        {pl.name}
                      </div>
                      <div
                        style={{
                          fontSize: "var(--fs-13)",
                          color: "var(--text-dim)",
                          marginTop: "var(--space-4)",
                        }}
                      >
                        {pl.description || "User Playlist"}
                      </div>
                    </div>

                    <div
                      style={{
                        display: "flex",
                        justifyContent: "space-between",
                        alignItems: "center",
                        marginTop: "var(--space-14)",
                        borderTop: "1px solid var(--border)",
                        paddingTop: "var(--space-10)",
                      }}
                    >
                      <button
                        className="btn btn-primary"
                        style={{
                          fontSize: "var(--fs-13)",
                          padding: "var(--space-6) var(--space-12)",
                          display: "flex",
                          alignItems: "center",
                          gap: "var(--space-6)",
                        }}
                        onClick={(e) => {
                          e.stopPropagation();
                          c.onPlayPlaylist(pl.id);
                        }}
                      >
                        <Play size={13} fill="currentColor" />
                        <span>Play</span>
                      </button>
                      <span
                        style={{ fontSize: "var(--fs-13)", color: "var(--text-dim)" }}
                      >
                        {pl.track_count ?? 0} tracks
                      </span>
                    </div>
                  </div>
                )}
              />
            </div>
          )}
        </div>
      )}
    </>
  );
}
