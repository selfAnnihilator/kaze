import { DownloadButton } from "../DownloadButton";
import { CoverOrDisc } from "./searchResults/CoverOrDisc";
import { NotInterestedButton } from "../NotInterestedButton";
import { QueueButton } from "../QueueButton";
import { queueIdOf } from "../../trackKind";
import { WindowedList } from "../../presentation/WindowedList";
import React, { useState, useMemo } from "react";
import { Play, Pause, Plus, Check, Music, RefreshCw } from "lucide-react";
import { DiscoveryRecommendation, Track, Playlist } from "../../types";
import type { OnlineSearchResultsSectionContext, SearchCategoryTab } from "./searchResults/context";
import { SearchResultsHeader } from "./searchResults/SearchResultsHeader";
import { AllResultsTab } from "./searchResults/AllResultsTab";
import { SongsTab } from "./searchResults/SongsTab";
import { ArtistsTab } from "./searchResults/ArtistsTab";
import { AlbumsTab } from "./searchResults/AlbumsTab";
import { PlaylistsTab } from "./searchResults/PlaylistsTab";

interface OnlineSearchResultsSectionProps {
  searchQuery: string;
  searchResults: DiscoveryRecommendation[];
  isSearching?: boolean;
  onClearSearch: () => void;
  onPlayOnlineTrack: (rec: DiscoveryRecommendation) => void;
  onStopTrack?: (rec: DiscoveryRecommendation) => void;
  activeOnlineTrackId?: string | null;
  isOnlinePlaying: boolean;
  isOnlineLoading: boolean;
  loadingTrackId?: string | null;
  currentLocalTrack?: Track | null;
  isLocalPlaying?: boolean;
  onAddToPlaylist?: (rec: DiscoveryRecommendation) => void;
  onAddToWishlist?: (rec: DiscoveryRecommendation) => void;
  trackPlaylistMap?: Record<string, string[]>;
  playlists?: Playlist[];
  onSelectArtist?: (artist: string) => void;
  queuedTrackIds?: Set<string>;
  onEnqueueTrack?: (rec: DiscoveryRecommendation) => void;
}


export const OnlineSearchResultsSection: React.FC<OnlineSearchResultsSectionProps> = ({
  searchQuery,
  searchResults,
  isSearching = false,
  onClearSearch,
  onPlayOnlineTrack,
  onStopTrack,
  activeOnlineTrackId = null,
  isOnlinePlaying,
  isOnlineLoading,
  loadingTrackId = null,
  currentLocalTrack,
  isLocalPlaying = false,
  onAddToPlaylist,
  onAddToWishlist: _onAddToWishlist,
  trackPlaylistMap,
  playlists = [],
  onSelectArtist,
  queuedTrackIds,
  onEnqueueTrack,
}) => {
  const [activeTab, setActiveTab] = useState<SearchCategoryTab>("all");

  const formatDuration = (secs?: number) => {
    if (!secs || secs <= 0) return "--:--";
    const m = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    return `${m}:${s < 10 ? "0" : ""}${s}`;
  };

  const isTrackPlaying = (rec: DiscoveryRecommendation) => {
    const hasRealLocalMatch =
      !!rec.matched_local_track_id &&
      !rec.matched_local_track_id.startsWith("itunes:") &&
      !rec.matched_local_track_id.startsWith("online:");

    const isPlayingLocal =
      rec.provider !== "jiosaavn" && isLocalPlaying &&
      !!currentLocalTrack &&
      currentLocalTrack.format !== "online" &&
      !currentLocalTrack.file_path.startsWith("online://") &&
      ((hasRealLocalMatch &&
        rec.matched_local_track_id === currentLocalTrack.id) ||
        rec.external_track_id === currentLocalTrack.id ||
        (rec.title.toLowerCase().trim() ===
          currentLocalTrack.title.toLowerCase().trim() &&
          currentLocalTrack.artist_name &&
          rec.artist.toLowerCase().trim() ===
            currentLocalTrack.artist_name.toLowerCase().trim()));

    const isPlayingOnline =
      activeOnlineTrackId === rec.external_track_id && isOnlinePlaying;

    return isPlayingLocal || isPlayingOnline;
  };

  const isTrackLoading = (rec: DiscoveryRecommendation) => {
    return (
      isOnlineLoading &&
      (loadingTrackId === rec.external_track_id ||
        (!!rec.matched_local_track_id &&
          loadingTrackId === rec.matched_local_track_id))
    );
  };

  const handleTogglePlay = (rec: DiscoveryRecommendation) => {
    if (isTrackLoading(rec)) return;
    if (isTrackPlaying(rec)) {
      if (onStopTrack) {
        onStopTrack(rec);
      } else {
        onPlayOnlineTrack(rec);
      }
    } else {
      onPlayOnlineTrack(rec);
    }
  };

  // Top result: first scored match
  const topResult = searchResults.length > 0 ? searchResults[0] : null;

  // Extract distinct artists
  const distinctArtists = useMemo(() => {
    const map = new Map<
      string,
      { artist: string; cover?: string; count: number }
    >();
    for (const r of searchResults) {
      const lower = r.artist.toLowerCase().trim();
      if (!map.has(lower)) {
        map.set(lower, { artist: r.artist, cover: r.cover_art_url, count: 1 });
      } else {
        const item = map.get(lower)!;
        item.count++;
        if (!item.cover && r.cover_art_url) item.cover = r.cover_art_url;
      }
    }
    return Array.from(map.values());
  }, [searchResults]);

  // Extract distinct albums
  const distinctAlbums = useMemo(() => {
    const map = new Map<
      string,
      { album: string; artist: string; cover?: string }
    >();
    for (const r of searchResults) {
      if (r.album && r.album.trim()) {
        const key = `${r.artist.toLowerCase().trim()}:${r.album.toLowerCase().trim()}`;
        if (!map.has(key)) {
          map.set(key, {
            album: r.album,
            artist: r.artist,
            cover: r.cover_art_url,
          });
        }
      }
    }
    return Array.from(map.values());
  }, [searchResults]);

  // Filter user's playlists matching search query
  const matchingPlaylists = useMemo(() => {
    const qLower = searchQuery.toLowerCase().trim();
    if (!qLower) return [];
    return playlists.filter(
      (p) =>
        p.name.toLowerCase().includes(qLower) ||
        (p.description && p.description.toLowerCase().includes(qLower)),
    );
  }, [playlists, searchQuery]);

  const songList = (<WindowedList
                items={searchResults}
                label="Search songs"
                fill
                rowHeight={80}
                renderItem={(rec) => {
                  const isPlaying = isTrackPlaying(rec);
                  const isLoading = isTrackLoading(rec);
                  const trackId =
                    rec.matched_local_track_id || rec.external_track_id;
                  const inPlaylist =
                    trackId && trackPlaylistMap?.[trackId]?.length
                      ? true
                      : false;

                  return (
                    <div
                      key={rec.external_track_id}
                      className="online-search-song"
                      data-track-id={rec.external_track_id}
                      role="button"
                      tabIndex={0}
                      aria-label={`${isPlaying ? "Pause" : "Play"} ${rec.title} by ${rec.artist}`}
                      onKeyDown={(event) => {
                        if (event.target === event.currentTarget && (event.key === "Enter" || event.key === " ")) {
                          event.preventDefault(); handleTogglePlay(rec);
                        }
                      }}
                      onClick={() => handleTogglePlay(rec)}
                      style={{
                        display: "flex",
                        alignItems: "center",
                        gap: "var(--space-12)",
                        padding: "var(--space-8) var(--space-12)",
                        borderRadius: "var(--radius-8)",
                        // Hover is CSS (.online-search-song:hover in App.css).
                        backgroundColor:
                          isPlaying || isLoading
                            ? "var(--accent-secondary-a10)"
                            : undefined,
                        cursor: "pointer",

                      }}
                    >
                      <div
                        style={{
                          width: "40px",
                          height: "40px",
                          borderRadius: "var(--radius-6)",
                          overflow: "hidden",
                          backgroundColor: "var(--overlay-5)",
                          position: "relative",
                          flexShrink: 0,
                        }}
                      >
                        <CoverOrDisc src={rec.cover_art_url} alt={rec.title} iconSize={18} />

                        <div
                          className={`online-search-overlay${isPlaying || isLoading ? " is-active" : ""}`}
                          style={{
                            position: "absolute",
                            inset: 0,
                            backgroundColor: "var(--shade-55)",
                            alignItems: "center",
                            justifyContent: "center",
                          }}
                        >
                          {isLoading ? (
                            <RefreshCw size={16} className="spin-animation" />
                          ) : isPlaying ? (
                            <Pause
                              size={16}
                              fill="var(--accent-secondary)"
                              color="var(--accent-secondary)"
                            />
                          ) : (
                            <Play size={16} fill="var(--text-main)" color="var(--text-main)" />
                          )}
                        </div>
                      </div>

                      <div style={{ flex: 1, minWidth: 0 }}>
                        <div
                          style={{
                            fontSize: "var(--fs-15)",
                            fontWeight: "var(--weight-semibold)",
                            color: isPlaying
                              ? "var(--accent-secondary)"
                              : "var(--text-main)",
                            whiteSpace: "nowrap",
                            overflow: "hidden",
                            textOverflow: "ellipsis",
                          }}
                        >
                          {rec.title}
                        </div>
                        <div
                          style={{
                            fontSize: "var(--fs-13)",
                            color: "var(--text-muted)",
                            whiteSpace: "nowrap",
                            overflow: "hidden",
                            textOverflow: "ellipsis",
                            marginTop: "var(--space-2)",
                          }}
                        >
                          {rec.artist} {rec.album ? `• ${rec.album}` : ""}
                        </div>
                      </div>

                      {onAddToPlaylist && (
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            onAddToPlaylist(rec);
                          }}
                          style={{
                            background: "none",
                            border: "none",
                            color: inPlaylist
                              ? "var(--accent-secondary)"
                              : "var(--text-dim)",
                            cursor: "pointer",
                            padding: "var(--space-6)",
                            display: "flex",
                            alignItems: "center",
                            justifyContent: "center",
                          }}
                          title={
                            inPlaylist ? "Added to playlist" : "Add to playlist"
                          }
                        >
                          {inPlaylist ? (
                            <Check size={16} />
                          ) : (
                            <Plus size={16} />
                          )}
                        </button>
                      )}

                      {onEnqueueTrack && (
                        <QueueButton title={rec.title} queued={!!queuedTrackIds?.has(queueIdOf(rec))} onEnqueue={() => onEnqueueTrack(rec)} />
                      )}
                      <DownloadButton key={rec.external_track_id} trackId={rec.external_track_id} sourceId={rec.playback_source?.track_id} title={rec.title} compact />
                          <NotInterestedButton trackId={rec.external_track_id} title={rec.title} />

                      <div
                        style={{
                          fontSize: "var(--fs-13)",
                          color: "var(--text-dim)",
                          minWidth: "38px",
                          textAlign: "right",
                        }}
                      >
                        {formatDuration(rec.duration_secs)}
                      </div>
                    </div>
                  );
                }}
              />);

  const c: OnlineSearchResultsSectionContext = {
    setActiveTab, activeTab, searchResults, distinctArtists, distinctAlbums, matchingPlaylists,
    searchQuery, onClearSearch, topResult, handleTogglePlay, isTrackPlaying, isTrackLoading,
    onSelectArtist, onAddToPlaylist, songList, queuedTrackIds, onEnqueueTrack,
  };
  return (
    <div className="online-search-results" data-category={activeTab}
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "var(--space-18)",
        minHeight: 0,
        flex: "1 1 0",
        overflow: "hidden",
        backgroundColor: "var(--palette-slate-900-a75)",
        border: "1px solid var(--overlay-8)",
        borderRadius: "var(--radius-14)",
        padding: "var(--space-20) var(--space-24)",
        boxShadow: "0 8px 32px var(--shade-35)",
      }}
    >
      {/* Top Header: Category Filter Pills + Clear Button */}
      <SearchResultsHeader c={c} />

      {/* Empty State if 0 Results */}
      {isSearching ? (
        <p role="status" style={{ padding: "var(--space-32) var(--space-24)", textAlign: "center", color: "var(--text-muted)" }}>Searching music…</p>
      ) : searchResults.length === 0 ? (
        <div
          style={{
            padding: "var(--space-48) var(--space-24)",
            textAlign: "center",
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            gap: "var(--space-12)",
          }}
        >
          <Music size={40} color="var(--text-dim)" />
          <h3 style={{ fontSize: "var(--fs-18)", fontWeight: "var(--weight-semibold)", color: "var(--text-main)" }}>
            No songs found for “{searchQuery}”
          </h3>
          <p
            style={{
              fontSize: "var(--fs-14)",
              color: "var(--text-muted)",
              maxWidth: "420px",
            }}
          >
            Try another song title or artist. Only songs available for full playback appear here.
          </p>
        </div>
      ) : (
        <>
          {/* TAB: ALL */}
          <AllResultsTab c={c} />

          {/* TAB: SONGS (Full List) */}
          <SongsTab c={c} />

          {/* TAB: ARTISTS */}
          <ArtistsTab c={c} />

          {/* TAB: ALBUMS */}
          <AlbumsTab c={c} />

          {/* TAB: PLAYLISTS */}
          <PlaylistsTab c={c} />
        </>
      )}
    </div>
  );
};
