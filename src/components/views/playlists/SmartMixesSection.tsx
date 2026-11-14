import { WindowedGallery } from "../../../presentation/WindowedGallery";
import { Check, Play, Plus, Sparkles } from "lucide-react";
import type { PlaylistsViewContext } from "./context";

export function SmartMixesSection({ c }: { c: PlaylistsViewContext }) {
  return (
    <>
      {(c.viewMode === "smart_mixes" || c.viewMode === "all") && (
        <div style={{ marginBottom: c.viewMode === "all" ? "40px" : "0" }}>
          {c.viewMode === "all" && (
            <div
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                marginBottom: "16px",
              }}
            >
              <h2
                style={{
                  fontSize: "1.15rem",
                  fontWeight: 600,
                  display: "flex",
                  alignItems: "center",
                  gap: "8px",
                }}
              >
                <Sparkles size={18} color="var(--accent-light)" />
                <span>Curated Smart Mixes & Recommendations</span>
              </h2>
              <span style={{ fontSize: "0.82rem", color: "var(--text-dim)" }}>
                Automatically derived from your genres and library
              </span>
            </div>
          )}

          {c.smartMixes.length === 0 ? (
            <div
              style={{
                padding: "40px 20px",
                borderRadius: "10px",
                backgroundColor: "var(--bg-card)",
                border: "1px dashed var(--border)",
                textAlign: "center",
                color: "var(--text-dim)",
              }}
            >
              <Sparkles
                size={36}
                color="var(--accent-light)"
                style={{ margin: "0 auto 12px" }}
              />
              <div
                style={{
                  fontWeight: 600,
                  color: "var(--text-main)",
                  marginBottom: "4px",
                }}
              >
                Generating Smart Mixes...
              </div>
              <p style={{ fontSize: "0.85rem" }}>
                Auto-curating mixes based on your music library folders and
                genres.
              </p>
            </div>
          ) : (
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(auto-fill, minmax(250px, 1fr))",
                gap: "18px",
              }}
            >
              <WindowedGallery
                items={c.smartMixes}
                label="smartMixes"
                renderItem={(mix) => {
                  const isSaved = c.savedMixIds.has(mix.id);
                  const isSaving = c.savingMixId === mix.id;

                  return (
                    <div
                      key={mix.id}
                      onClick={() => c.handleOpenPlaylistDetails(mix)}
                      style={{
                        backgroundColor: "var(--bg-card)",
                        borderRadius: "12px",
                        padding: "18px",
                        border: "1px solid var(--border)",
                        cursor: "pointer",
                        display: "flex",
                        flexDirection: "column",
                        justifyContent: "space-between",
                        transition:
                          "transform 0.15s, background-color 0.15s, border-color 0.15s",
                      }}
                      onMouseEnter={(e) => {
                        e.currentTarget.style.backgroundColor =
                          "var(--bg-card-hover)";
                        e.currentTarget.style.borderColor =
                          "var(--accent-light)";
                      }}
                      onMouseLeave={(e) => {
                        e.currentTarget.style.backgroundColor =
                          "var(--bg-card)";
                        e.currentTarget.style.borderColor = "var(--border)";
                      }}
                      title="Click to view tracks in this mix"
                    >
                      <div>
                        <div
                          style={{
                            display: "flex",
                            justifyContent: "space-between",
                            alignItems: "flex-start",
                            marginBottom: "12px",
                          }}
                        >
                          <div
                            style={{
                              width: "48px",
                              height: "48px",
                              borderRadius: "10px",
                              background:
                                "linear-gradient(135deg, rgba(243, 112, 30, 0.35), rgba(139, 124, 246, 0.15))",
                              display: "flex",
                              alignItems: "center",
                              justifyContent: "center",
                              overflow: "hidden",
                              flexShrink: 0,
                            }}
                          >
                            <img decoding="async"
                              src={
                                mix.cover_art_url ||
                                "/kaze-playlist-default.svg"
                              }
                              alt={mix.name}
                              style={{
                                width: "100%",
                                height: "100%",
                                objectFit: "cover",
                              }}
                            />
                          </div>
                          <span className="badge badge-exact">Smart Mix</span>
                        </div>

                        <div
                          style={{
                            fontWeight: 700,
                            fontSize: "1.05rem",
                            color: "#e8d8c9",
                            marginBottom: "6px",
                          }}
                        >
                          {mix.name}
                        </div>
                        <div
                          style={{
                            fontSize: "0.82rem",
                            color: "var(--text-dim)",
                            lineHeight: 1.4,
                            marginBottom: "16px",
                          }}
                        >
                          {mix.description ||
                            mix.generation_reason ||
                            "Curated blend of local tracks"}
                        </div>
                      </div>

                      <div
                        style={{
                          display: "flex",
                          gap: "8px",
                          borderTop: "1px solid var(--border)",
                          paddingTop: "14px",
                        }}
                      >
                        <button
                          className="btn btn-primary"
                          style={{
                            flex: 1,
                            justifyContent: "center",
                            fontSize: "0.85rem",
                            padding: "8px 12px",
                          }}
                          onClick={(e) => {
                            e.stopPropagation();
                            c.onPlayPlaylist(mix.id);
                          }}
                        >
                          <Play size={15} fill="currentColor" />
                          <span>Play Mix</span>
                        </button>

                        <button
                          className="btn btn-secondary"
                          style={{
                            fontSize: "0.82rem",
                            padding: "8px 12px",
                            display: "flex",
                            alignItems: "center",
                            gap: "6px",
                            backgroundColor: isSaved
                              ? "rgba(139, 124, 246, 0.15)"
                              : undefined,
                            borderColor: isSaved
                              ? "var(--accent-secondary)"
                              : undefined,
                            color: isSaved
                              ? "var(--accent-secondary)"
                              : undefined,
                          }}
                          onClick={(e) => {
                            e.stopPropagation();
                            c.handleSaveMixToUserPlaylist(mix);
                          }}
                          disabled={isSaving}
                          title="Add this mix as a permanent playlist in your library"
                        >
                          {isSaved ? (
                            <>
                              <Check
                                size={14}
                                color="var(--accent-secondary)"
                              />
                              <span>Saved</span>
                            </>
                          ) : (
                            <>
                              <Plus size={14} />
                              <span>
                                {isSaving ? "Saving..." : "Add to Playlist"}
                              </span>
                            </>
                          )}
                        </button>
                      </div>
                    </div>
                  );
                }}
              />
            </div>
          )}
        </div>
      )}
    </>
  );
}
