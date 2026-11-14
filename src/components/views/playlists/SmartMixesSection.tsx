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
                marginBottom: "var(--space-16)",
              }}
            >
              <h2
                style={{
                  fontSize: "var(--fs-18)",
                  fontWeight: "var(--weight-semibold)",
                  display: "flex",
                  alignItems: "center",
                  gap: "var(--space-8)",
                }}
              >
                <Sparkles size={18} color="var(--accent-light)" />
                <span>Curated Smart Mixes & Recommendations</span>
              </h2>
              <span style={{ fontSize: "var(--fs-13)", color: "var(--text-dim)" }}>
                Automatically derived from your genres and library
              </span>
            </div>
          )}

          {c.smartMixes.length === 0 ? (
            <div
              style={{
                padding: "var(--space-40) var(--space-20)",
                borderRadius: "var(--radius-10)",
                backgroundColor: "var(--bg-card)",
                border: "1px dashed var(--border)",
                textAlign: "center",
                color: "var(--text-dim)",
              }}
            >
              <Sparkles
                size={36}
                color="var(--accent-light)"
                style={{ margin: "0 auto var(--space-12)" }}
              />
              <div
                style={{
                  fontWeight: "var(--weight-semibold)",
                  color: "var(--text-main)",
                  marginBottom: "var(--space-4)",
                }}
              >
                Generating Smart Mixes...
              </div>
              <p style={{ fontSize: "var(--fs-14)" }}>
                Auto-curating mixes based on your music library folders and
                genres.
              </p>
            </div>
          ) : (
            <div
              style={{
                display: "grid",
                gridTemplateColumns: "repeat(auto-fill, minmax(250px, 1fr))",
                gap: "var(--space-18)",
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
                        borderRadius: "var(--radius-12)",
                        padding: "var(--space-18)",
                        border: "1px solid var(--border)",
                        cursor: "pointer",
                        display: "flex",
                        flexDirection: "column",
                        justifyContent: "space-between",
                        transition:
                          "transform var(--duration-fast), background-color var(--duration-fast), border-color var(--duration-fast)",
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
                            marginBottom: "var(--space-12)",
                          }}
                        >
                          <div
                            style={{
                              width: "48px",
                              height: "48px",
                              borderRadius: "var(--radius-10)",
                              background:
                                "linear-gradient(135deg, var(--accent-a35), var(--accent-secondary-a15))",
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
                            fontWeight: "var(--weight-bold)",
                            fontSize: "var(--fs-16)",
                            color: "var(--text-main)",
                            marginBottom: "var(--space-6)",
                          }}
                        >
                          {mix.name}
                        </div>
                        <div
                          style={{
                            fontSize: "var(--fs-13)",
                            color: "var(--text-dim)",
                            lineHeight: 1.4,
                            marginBottom: "var(--space-16)",
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
                          gap: "var(--space-8)",
                          borderTop: "1px solid var(--border)",
                          paddingTop: "var(--space-14)",
                        }}
                      >
                        <button
                          className="btn btn-primary"
                          style={{
                            flex: 1,
                            justifyContent: "center",
                            fontSize: "var(--fs-14)",
                            padding: "var(--space-8) var(--space-12)",
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
                            fontSize: "var(--fs-13)",
                            padding: "var(--space-8) var(--space-12)",
                            display: "flex",
                            alignItems: "center",
                            gap: "var(--space-6)",
                            backgroundColor: isSaved
                              ? "var(--accent-secondary-a15)"
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
