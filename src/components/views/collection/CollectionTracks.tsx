import { Artwork } from "../../../presentation/Artwork";
import { WindowedList } from "../../../presentation/WindowedList";
import { DownloadButton } from "../../DownloadButton";
import { NotInterestedButton } from "../../NotInterestedButton";
import { Bookmark, BookmarkCheck, Check, Clock, Heart, ListPlus, Music2, Pause, Play, RefreshCw, Search } from "lucide-react";
import type { CollectionDetailViewContext } from "./context";

export function CollectionTracks({ c }: { c: CollectionDetailViewContext }) {
  return (
    <>
      {c.isLoadingTracks ? (
        <div
          key="collection-search-loading"
          style={{
            textAlign: "center",
            padding: "60px 20px",
            color: "var(--text-dim)",
          }}
        >
          <RefreshCw
            size={36}
            className="animate-spin"
            color="var(--accent-light)"
            style={{ margin: "0 auto 12px" }}
          />
          <p
            style={{
              fontSize: "1rem",
              fontWeight: 600,
              color: "var(--text-main)",
            }}
          >
            Loading collection songs...
          </p>
        </div>
      ) : c.tracks.length === 0 ? (
        c.rawTracks.length > 0 ? (
          <div
            key="collection-search-empty"
            className="content-card"
            style={{
              textAlign: "center",
              padding: "50px 20px",
              color: "var(--text-dim)",
              borderStyle: "dashed",
            }}
          >
            <Search
              size={36}
              color="var(--accent-light)"
              style={{ margin: "0 auto 12px" }}
            />
            <p
              style={{
                fontSize: "1.05rem",
                fontWeight: 600,
                color: "var(--text-main)",
                marginBottom: "6px",
              }}
            >
              No songs found matching "{c.searchQuery}"
            </p>
            <p
              style={{
                fontSize: "0.84rem",
                color: "var(--text-muted)",
                marginBottom: "16px",
              }}
            >
              Check your spelling or try searching for a different track title
              or artist.
            </p>
            <button
              type="button"
              className="btn btn-secondary"
              onClick={() => c.setSearchQuery("")}
              style={{ fontSize: "0.82rem" }}
            >
              Clear Search
            </button>
          </div>
        ) : (
          <div
            key="collection-empty"
            className="content-card"
            style={{
              textAlign: "center",
              padding: "50px 20px",
              color: "var(--text-dim)",
              borderStyle: "dashed",
            }}
          >
            <Music2
              size={36}
              color="var(--accent-light)"
              style={{ marginBottom: "12px" }}
            />
            <p
              style={{
                fontSize: "1.05rem",
                fontWeight: 600,
                color: "var(--text-main)",
                marginBottom: "6px",
              }}
            >
              No songs in this collection yet
            </p>
          </div>
        )
      ) : (
        <div
          key="collection-search-results"
          className="collection-track-sheet"
          style={{
            backgroundColor: "var(--bg-card)",
            borderRadius: "12px",
            border: "1px solid var(--border)",
            overflow: "hidden",
          }}
        >
          {/* Table Header */}
          <div
            className="collection-track-header"
            style={{
              display: "grid",
              gridTemplateColumns: "48px 1fr 1fr 64px 184px",
              columnGap: "12px",
              padding: "12px 18px",
              borderBottom: "1px solid var(--border)",
              fontSize: "0.78rem",
              fontWeight: 600,
              letterSpacing: "0.6px",
              color: "var(--text-dim)",
              textTransform: "uppercase",
            }}
          >
            <div>#</div>
            <div>Title</div>
            <div>Album</div>
            <div style={{ textAlign: "right" }}>
              <Clock size={14} style={{ display: "inline-block" }} />
            </div>
            <div style={{ textAlign: "right" }}>Action</div>
          </div>

          {/* Table Rows */}
          <WindowedList
            items={c.tracks}
            rowHeight={72}
            label="Collection tracks"
            renderItem={(track, idx) => {
              const isHovered = c.hoveredRowId === track.id;
              const isPlayingThis =
                c.isPlaying &&
                (c.currentPlayingTrackId === track.id ||
                  c.currentPlayingTrackId === track.matched_local_track_id);
              const isLoadingThis =
                c.loadingTrackId === track.id ||
                (!!track.matched_local_track_id &&
                  c.loadingTrackId === track.matched_local_track_id);

              const effectiveTrackId = track.matched_local_track_id || track.id;
              const isLiked = c.likedTrackIds
                ? c.likedTrackIds.has(effectiveTrackId) ||
                  c.likedTrackIds.has(track.id)
                : c.isLikedSongs;
              const isQueued = c.queuedTrackIds?.has(effectiveTrackId) || false;

              return (
                <div
                  className="collection-track-row"
                  key={c.visibleTrackEntries[idx].rowKey}
                  onMouseEnter={() => c.setHoveredRowId(track.id)}
                  onMouseLeave={() => c.setHoveredRowId(null)}
                  onClick={() => c.onPlayTrack(track)}
                  style={{
                    height: "100%",
                    display: "grid",
                    gridTemplateColumns: "48px 1fr 1fr 64px 184px",
                    columnGap: "12px",
                    padding: "10px 18px",
                    alignItems: "center",
                    fontSize: "0.88rem",
                    color:
                      isPlayingThis || isLoadingThis
                        ? "var(--accent-secondary)"
                        : "var(--text-main)",
                    backgroundColor:
                      isPlayingThis || isLoadingThis
                        ? "var(--accent-secondary-a8)"
                        : isHovered
                          ? "var(--overlay-5)"
                          : "transparent",
                    borderBottom: "1px solid var(--overlay-3)",
                    cursor: "pointer",
                    transition: "background-color 0.15s ease",
                  }}
                >
                  {/* Index / Play icon */}
                  <div
                    style={{
                      color:
                        isPlayingThis || isLoadingThis
                          ? "var(--accent-secondary)"
                          : "var(--text-dim)",
                      fontSize: "0.85rem",
                      fontWeight: 500,
                    }}
                  >
                    {isLoadingThis ? (
                      <RefreshCw
                        size={15}
                        className="spin-animation"
                        aria-label="Loading song"
                      />
                    ) : isPlayingThis ? (
                      isHovered ? (
                        <Pause size={15} fill="var(--accent-secondary)" />
                      ) : (
                        <div
                          className="discovery-eq-container"
                          style={{ height: "13px" }}
                        >
                          <div
                            className="discovery-eq-bar"
                            style={{
                              width: "2.5px",
                              backgroundColor: "var(--accent-secondary)",
                            }}
                          />
                          <div
                            className="discovery-eq-bar"
                            style={{
                              width: "2.5px",
                              backgroundColor: "var(--accent-secondary)",
                            }}
                          />
                          <div
                            className="discovery-eq-bar"
                            style={{
                              width: "2.5px",
                              backgroundColor: "var(--accent-secondary)",
                            }}
                          />
                        </div>
                      )
                    ) : isHovered ? (
                      <Play size={15} fill="var(--text-main)" />
                    ) : (
                      idx + 1
                    )}
                  </div>

                  {/* Title & Artist & Artwork */}
                  <div
                    style={{
                      display: "flex",
                      alignItems: "center",
                      gap: "12px",
                      minWidth: 0,
                    }}
                  >
                    <div
                      style={{
                        width: "42px",
                        height: "42px",
                        borderRadius: "6px",
                        overflow: "hidden",
                        backgroundColor: "var(--overlay-5)",
                        flexShrink: 0,
                      }}
                    >
                      <Artwork src={track.cover_art_url} alt={track.title} />
                    </div>

                    <div style={{ overflow: "hidden", minWidth: 0 }}>
                      <div
                        style={{
                          fontWeight: 600,
                          fontSize: "0.92rem",
                          whiteSpace: "nowrap",
                          overflow: "hidden",
                          textOverflow: "ellipsis",
                          color: isPlayingThis
                            ? "var(--accent-secondary)"
                            : "var(--text-main)",
                        }}
                      >
                        {track.title}
                      </div>
                      <div
                        style={{
                          fontSize: "0.78rem",
                          color: "var(--text-muted)",
                          whiteSpace: "nowrap",
                          overflow: "hidden",
                          textOverflow: "ellipsis",
                          marginTop: "2px",
                        }}
                      >
                        {track.artist}
                      </div>
                    </div>
                  </div>

                  {/* Album */}
                  <div
                    style={{
                      color: "var(--text-muted)",
                      fontSize: "0.84rem",
                      whiteSpace: "nowrap",
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      paddingRight: "16px",
                    }}
                  >
                    {track.album || "—"}
                  </div>

                  {/* Duration */}
                  <div
                    style={{
                      textAlign: "right",
                      color: "var(--text-dim)",
                      fontSize: "0.82rem",
                    }}
                  >
                    {c.formatDuration(track.duration_secs)}
                  </div>

                  {/* Action Buttons */}
                  <div
                    style={{
                      display: "flex",
                      alignItems: "center",
                      justifyContent: "flex-end",
                      gap: "6px",
                    }}
                    onClick={(e) => e.stopPropagation()}
                  >
                    {/* Local songs have nothing to download; online songs show only Kaze's own download state. */}
                    {!track.rawLocalTrack && <DownloadButton key={track.id} sourceId={track.rawRecommendation?.playback_source?.track_id} trackId={track.id} title={track.title} compact />}

                    <button
                      type="button"
                      onClick={(e) => {
                        e.stopPropagation();
                        c.onEnqueueTrack?.(track);
                      }}
                      style={{
                        background: isQueued
                          ? "var(--accent-secondary-a15)"
                          : "none",
                        border: isQueued
                          ? "1px solid var(--accent-secondary-a40)"
                          : "1px solid transparent",
                        borderRadius: "50%",
                        color: isQueued
                          ? "var(--accent-secondary)"
                          : "var(--text-dim)",
                        cursor: "pointer",
                        padding: "4px",
                        display: "flex",
                      }}
                      title={isQueued ? "Already in queue" : "Add to queue"}
                    >
                      {isQueued ? <Check size={15} /> : <ListPlus size={15} />}
                    </button>

                    <NotInterestedButton trackId={track.id} title={track.title} />
                    <button
                      type="button"
                      onClick={(e) => {
                        e.stopPropagation();
                        c.onToggleLike?.(track, !!isLiked);
                      }}
                      style={{
                        background: "none",
                        border: "none",
                        color: isLiked ? "var(--palette-pink-400)" : "var(--text-dim)",
                        cursor: "pointer",
                        padding: "4px",
                        display: "flex",
                      }}
                      title={isLiked ? "Unlike track" : "Like track"}
                      aria-label={isLiked ? "Unlike track" : "Like track"}
                    >
                      <Heart size={15} fill={isLiked ? "var(--palette-pink-400)" : "none"} />
                    </button>

                    {(() => {
                      const trackId = track.matched_local_track_id || track.id;
                      const allPlaylistIds =
                        (trackId && c.trackPlaylistMap?.[trackId]) || [];
                      // Only count user-created playlists (exclude smart mixes and the current collection itself)
                      const playlistIds = c.userPlaylistIds
                        ? allPlaylistIds.filter((pid) =>
                            c.userPlaylistIds?.has(pid),
                          )
                        : allPlaylistIds;
                      const isInPlaylist = playlistIds.length > 0;

                      return (
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            if (c.onAddToPlaylist) {
                              c.onAddToPlaylist(track);
                            } else if (c.onAddToWishlist) {
                              c.onAddToWishlist(track);
                            }
                          }}
                          style={{
                            background: "none",
                            border: "none",
                            color: isInPlaylist
                              ? "var(--accent-secondary)"
                              : "var(--text-dim)",
                            cursor: "pointer",
                            padding: "4px",
                            display: "flex",
                          }}
                          title={
                            isInPlaylist
                              ? "In playlist (click to manage)"
                              : "Add to playlist"
                          }
                        >
                          {isInPlaylist ? (
                            <BookmarkCheck
                              size={15}
                              color="var(--accent-secondary)"
                            />
                          ) : (
                            <Bookmark size={15} />
                          )}
                        </button>
                      );
                    })()}
                  </div>
                </div>
              );
            }}
          />
        </div>
      )}
    </>
  );
}
