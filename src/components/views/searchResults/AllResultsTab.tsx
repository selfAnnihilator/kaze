import { DownloadButton } from "../../DownloadButton";
import { NotInterestedButton } from "../../NotInterestedButton";
import { CoverOrDisc } from "./CoverOrDisc";
import { Pause, Play, Plus } from "lucide-react";
import type { OnlineSearchResultsSectionContext } from "./context";

export function AllResultsTab({ c }: { c: OnlineSearchResultsSectionContext }) {
  const topResult = c.topResult;
  return (
    <>
      {c.activeTab === "all" && (
        <div
          className="online-search-all"
        >
          <div className="online-search-overview">
            {/* Column 1: Top Result Card */}
            {topResult && (
              <div className="online-search-top-result">
                <h3
                  style={{
                    fontSize: "1.2rem",
                    fontWeight: 700,
                    color: "#e8d8c9",
                    marginBottom: "14px",
                  }}
                >
                  Top result
                </h3>
                <div
                  onClick={() => c.handleTogglePlay(topResult)}
                  style={{
                    backgroundColor:
                      c.isTrackPlaying(topResult) || c.isTrackLoading(topResult)
                        ? "rgba(139, 124, 246, 0.14)"
                        : "var(--bg-card)",
                    border:
                      c.isTrackPlaying(topResult) || c.isTrackLoading(topResult)
                        ? "1.5px solid var(--accent-secondary)"
                        : "1px solid var(--border)",
                    borderRadius: "12px",
                    padding: "20px",
                    cursor: "pointer",
                    display: "flex",
                    flexDirection: "column",
                    gap: "12px",
                    position: "relative",
                    transition: "transform 0.2s cubic-bezier(0.4, 0, 0.2, 1)",
                    boxShadow: "0 6px 20px rgba(0, 0, 0, 0.25)",
                  }}
                >
                  {/* Top Result Artwork */}
                  <div
                    style={{
                      width: "96px",
                      height: "96px",
                      borderRadius: "8px",
                      overflow: "hidden",
                      backgroundColor: "rgba(255, 255, 255, 0.05)",
                      boxShadow: "0 8px 24px rgba(0, 0, 0, 0.4)",
                      position: "relative",
                      flexShrink: 0,
                    }}
                  >
                    <CoverOrDisc src={topResult.cover_art_url} alt={topResult.title} iconSize={40} background="linear-gradient(135deg, rgba(243, 112, 30, 0.2), rgba(232, 216, 201, 0.15))" />
                  </div>

                  {/* Title */}
                  <div
                    style={{
                      fontSize: "1.45rem",
                      fontWeight: 700,
                      color: c.isTrackPlaying(topResult)
                        ? "var(--accent-secondary)"
                        : "#e8d8c9",
                      whiteSpace: "nowrap",
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      marginTop: "6px",
                    }}
                    title={topResult.title}
                  >
                    {topResult.title}
                  </div>

                  {/* Subtitle with Badge */}
                  <div
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: "8px",
                      fontSize: "0.86rem",
                      color: "var(--text-muted)",
                    }}
                  >
                    <span
                      style={{
                        padding: "2px 8px",
                        borderRadius: "10px",
                        backgroundColor: "rgba(255, 255, 255, 0.1)",
                        color: "#e8d8c9",
                        fontSize: "0.72rem",
                        fontWeight: 600,
                      }}
                    >
                      Song
                    </span>
                    <span>•</span>
                    <span
                      onClick={(e) => {
                        e.stopPropagation();
                        c.onSelectArtist?.(topResult.artist);
                      }}
                      style={{
                        cursor: "pointer",
                        color: "var(--text-main)",
                        fontWeight: 500,
                      }}
                      onMouseEnter={(e) =>
                        (e.currentTarget.style.textDecoration = "underline")
                      }
                      onMouseLeave={(e) =>
                        (e.currentTarget.style.textDecoration = "none")
                      }
                    >
                      {topResult.artist}
                    </span>
                  </div>

                  {/* Bottom action row: buttons + large circular play button */}
                  <div
                    style={{
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "space-between",
                      marginTop: "12px",
                    }}
                  >
                    <div
                      style={{
                        display: "flex",
                        alignItems: "center",
                        gap: "8px",
                      }}
                    >
                      {c.onAddToPlaylist && (
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            c.onAddToPlaylist?.(topResult);
                          }}
                          style={{
                            width: "34px",
                            height: "34px",
                            borderRadius: "50%",
                            border: "1px solid rgba(255, 255, 255, 0.15)",
                            backgroundColor: "rgba(255, 255, 255, 0.06)",
                            color: "#e8d8c9",
                            display: "flex",
                            alignItems: "center",
                            justifyContent: "center",
                            cursor: "pointer",
                            transition: "opacity 0.15s ease",
                          }}
                          title="Add to playlist"
                        >
                          <Plus size={16} />
                        </button>
                      )}
                      <DownloadButton key={topResult.external_track_id} trackId={topResult.external_track_id} sourceId={topResult.playback_source?.track_id} title={topResult.title} compact />
                      <NotInterestedButton trackId={topResult.external_track_id} title={topResult.title} />
                    </div>

                    {/* Prominent Play Button (Spotify green circular) */}
                    <button
                      type="button"
                      onClick={(e) => {
                        e.stopPropagation();
                        c.handleTogglePlay(topResult);
                      }}
                      style={{
                        width: "48px",
                        height: "48px",
                        borderRadius: "50%",
                        backgroundColor: "var(--accent-secondary)",
                        border: "none",
                        color: "#e8d8c9",
                        display: "flex",
                        alignItems: "center",
                        justifyContent: "center",
                        cursor: "pointer",
                        boxShadow: "0 8px 18px rgba(139, 124, 246, 0.35)",
                        transition:
                          "transform 0.15s ease",
                      }}
                      title={c.isTrackPlaying(topResult) ? "Pause" : "Play"}
                    >
                      {c.isTrackLoading(topResult) ? (
                        <div
                          className="spin-animation"
                          style={{
                            width: "18px",
                            height: "18px",
                            border: "2px solid #e8d8c9",
                            borderTopColor: "transparent",
                            borderRadius: "50%",
                          }}
                        />
                      ) : c.isTrackPlaying(topResult) ? (
                        <Pause size={22} fill="#e8d8c9" />
                      ) : (
                        <Play
                          size={22}
                          fill="#e8d8c9"
                          style={{ marginLeft: "2px" }}
                        />
                      )}
                    </button>
                  </div>
                </div>
              </div>
            )}

            <section className="online-search-songs">
              <h3>Songs ({c.searchResults.length})</h3>
              {c.songList}
            </section>
          </div>
        </div>
      )}
    </>
  );
}
