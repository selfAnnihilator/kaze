import React, { useState, useRef, useEffect } from "react";
import { List, RowComponentProps } from "react-window";
import { Play, Search, RotateCw, Heart, Check, Bookmark, BookmarkCheck, ListPlus } from "lucide-react";
import { Track } from "../../types";

interface LibraryViewProps {
  tracks: Track[];
  queuedTrackIds?: Set<string>;
  onPlayTrack: (trackId: string) => void;
  loadingTrackId?: string | null;
  onEnqueueTrack: (trackId: string) => void;
  onLikeTrack: (trackId: string) => void;
  onRemoveFeedback: (trackId: string) => void;
  onRescan: () => void;
  onSearch: (query: string) => void;
  trackPlaylistMap?: Record<string, string[]>;
  onOpenAddToPlaylistModal?: (track: Track) => void;
}

const formatSeconds = (secs: number) => {
  const m = Math.floor(secs / 60);
  const s = Math.floor(secs % 60);
  return `${m}:${s < 10 ? "0" : ""}${s}`;
};

const ROW_HEIGHT = 48;

type LibraryRowProps = {
  tracks: Track[];
  queuedTrackIds?: Set<string>;
  trackPlaylistMap?: Record<string, string[]>;
  onPlayTrack: (trackId: string) => void;
  loadingTrackId?: string | null;
  onEnqueueTrack: (trackId: string) => void;
  onLikeTrack: (trackId: string) => void;
  onRemoveFeedback: (trackId: string) => void;
  onOpenAddToPlaylistModal?: (track: Track) => void;
};

const LibraryRow = ({
  index,
  style,
  tracks,
  queuedTrackIds,
  trackPlaylistMap,
  onPlayTrack,
  loadingTrackId,
  onEnqueueTrack,
  onLikeTrack,
  onRemoveFeedback,
  onOpenAddToPlaylistModal,
}: RowComponentProps<LibraryRowProps>): React.ReactElement | null => {
  const track = tracks[index];
  if (!track) return null;
  const isLiked = track.manual_like === 1;
  const isEnqueued = queuedTrackIds ? queuedTrackIds.has(track.id) : false;
  const playlistIds = trackPlaylistMap?.[track.id] || [];
  const isInPlaylist = playlistIds.length > 0;
  const isLoading = loadingTrackId === track.id;

  return (
    <div
      style={{
        ...style,
        display: "grid",
        gridTemplateColumns: "48px minmax(180px, 2fr) minmax(130px, 1.3fr) minmax(130px, 1.3fr) 68px 70px 120px",
        alignItems: "center",
        padding: "0 14px",
        fontSize: "0.9rem",
        color: "var(--text-muted)",
        backgroundColor: isLoading ? "rgba(243, 112, 30, 0.12)" : undefined,
        borderBottom: "1px solid var(--border)",
        boxSizing: "border-box",
      }}
      className="track-row"
      onDoubleClick={() => onPlayTrack(track.id)}
    >
      <div style={{ textAlign: "center", color: "var(--text-dim)" }}>{index + 1}</div>
      <div className="primary" style={{ display: "flex", alignItems: "center", gap: "8px", overflow: "hidden" }}>
        <button
          className="player-icon-btn"
          onClick={() => onPlayTrack(track.id)}
          style={{ color: "var(--accent-light)", flexShrink: 0 }}
        >
          {isLoading ? <RotateCw size={14} className="spin-animation" aria-label="Loading song" /> : <Play size={14} />}
        </button>
        <span style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
          {track.title}
        </span>
      </div>
      <div style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
        {track.artist_name || "Unknown Artist"}
      </div>
      <div style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
        {track.album_title || "Unknown Album"}
      </div>
      <div>
        <span
          className="badge"
          style={{
            backgroundColor: "var(--border)",
            color: "var(--text-muted)",
            fontSize: "0.72rem",
            padding: "2px 6px",
            borderRadius: "4px",
          }}
        >
          {track.format.toUpperCase()}
        </span>
      </div>
      <div style={{ textAlign: "right" }}>{formatSeconds(track.duration_secs)}</div>
      <div style={{ display: "flex", justifyContent: "center", gap: "8px" }}>
        <button
          className="player-icon-btn"
          title={isInPlaylist ? "In playlist (click to manage)" : "Add to playlist"}
          onClick={() => onOpenAddToPlaylistModal && onOpenAddToPlaylistModal(track)}
          style={{ color: isInPlaylist ? "var(--accent-secondary)" : "var(--text-muted)" }}
        >
          {isInPlaylist ? (
            <BookmarkCheck size={16} color="var(--accent-secondary)" />
          ) : (
            <Bookmark size={16} />
          )}
        </button>
        <button
          className="player-icon-btn"
          title={isEnqueued ? "Already in queue" : "Add to queue"}
          onClick={() => onEnqueueTrack(track.id)}
          style={{ color: isEnqueued ? "var(--success)" : "var(--text-muted)" }}
        >
          {isEnqueued ? <Check size={16} /> : <ListPlus size={16} />}
        </button>
        <button
          className="player-icon-btn"
          title={isLiked ? "Unlike track" : "Like track"}
          onClick={() => (isLiked ? onRemoveFeedback(track.id) : onLikeTrack(track.id))}
          style={{ color: isLiked ? "#ef4444" : "var(--text-muted)" }}
        >
          <Heart size={16} fill={isLiked ? "#ef4444" : "none"} />
        </button>
      </div>
    </div>
  );
};

function useContainerDimensions(ref: React.RefObject<HTMLDivElement | null>) {
  const [dimensions, setDimensions] = useState({ width: 0, height: 500 });

  useEffect(() => {
    if (!ref.current) return;
    const update = () => {
      if (ref.current) {
        const rect = ref.current.getBoundingClientRect();
        if (rect.height > 0) {
          setDimensions({ width: rect.width, height: rect.height });
        }
      }
    };
    update();
    const observer = new ResizeObserver(() => update());
    observer.observe(ref.current);
    window.addEventListener("resize", update);
    return () => {
      observer.disconnect();
      window.removeEventListener("resize", update);
    };
  }, [ref]);

  return dimensions;
}

export const LibraryView: React.FC<LibraryViewProps> = React.memo(({
  tracks,
  queuedTrackIds,
  onPlayTrack,
  loadingTrackId,
  onEnqueueTrack,
  onLikeTrack,
  onRemoveFeedback,
  onRescan,
  onSearch,
  trackPlaylistMap,
  onOpenAddToPlaylistModal,
}) => {
  const [searchTerm, setSearchTerm] = useState("");
  const debounceTimerRef = useRef<any>(null);
  const listContainerRef = useRef<HTMLDivElement | null>(null);
  const { height: containerHeight } = useContainerDimensions(listContainerRef);

  const handleSearchChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value;
    setSearchTerm(val);
    if (debounceTimerRef.current) {
      clearTimeout(debounceTimerRef.current);
    }
    debounceTimerRef.current = setTimeout(() => {
      onSearch(val);
    }, 200);
  };

  useEffect(() => {
    return () => {
      if (debounceTimerRef.current) clearTimeout(debounceTimerRef.current);
    };
  }, []);

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", minHeight: 0 }}>
      <div className="view-header" style={{ flexShrink: 0 }}>
        <div>
          <h1 className="view-title">Music Library</h1>
          <p style={{ color: "var(--text-dim)", fontSize: "0.88rem", marginTop: "4px" }}>
            {tracks.length} local audio track{tracks.length === 1 ? "" : "s"} indexed
          </p>
        </div>

        <div style={{ display: "flex", gap: "12px", alignItems: "center" }}>
          <div
            style={{
              display: "flex",
              alignItems: "center",
              gap: "8px",
              backgroundColor: "var(--bg-card)",
              border: "1px solid var(--border)",
              borderRadius: "8px",
              padding: "6px 12px",
              width: "260px",
            }}
          >
            <Search size={16} color="var(--text-dim)" />
            <input
              type="text"
              placeholder="Search title, artist, album..."
              value={searchTerm}
              onChange={handleSearchChange}
              style={{
                background: "none",
                border: "none",
                outline: "none",
                color: "#e8d8c9",
                fontSize: "0.88rem",
                width: "100%",
              }}
            />
          </div>

          <button className="btn btn-secondary" onClick={onRescan} title="Rescan library for new files">
            <RotateCw size={16} />
          </button>
        </div>
      </div>

      {tracks.length === 0 ? (
        <div style={{ textAlign: "center", padding: "60px 0", color: "var(--text-dim)" }}>
          <p style={{ fontSize: "1.1rem", marginBottom: "8px" }}>No tracks found in library</p>
          <p style={{ fontSize: "0.88rem" }}>Add music files to your configured folder and click Rescan.</p>
        </div>
      ) : (
        <div
          ref={listContainerRef}
          style={{
            flex: 1,
            minHeight: "400px",
            display: "flex",
            flexDirection: "column",
            border: "1px solid var(--border)",
            borderRadius: "8px",
            backgroundColor: "var(--bg-main)",
            overflow: "hidden",
          }}
        >
          {/* Virtualized List Header */}
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "48px minmax(180px, 2fr) minmax(130px, 1.3fr) minmax(130px, 1.3fr) 68px 70px 120px",
              padding: "10px 14px",
              color: "var(--text-dim)",
              fontSize: "0.8rem",
              fontWeight: 600,
              borderBottom: "1px solid var(--border)",
              textTransform: "uppercase",
              backgroundColor: "var(--bg-card)",
              alignItems: "center",
              flexShrink: 0,
            }}
          >
            <div style={{ textAlign: "center" }}>#</div>
            <div>Title</div>
            <div>Artist</div>
            <div>Album</div>
            <div>Format</div>
            <div style={{ textAlign: "right" }}>Duration</div>
            <div style={{ textAlign: "center" }}>Actions</div>
          </div>

          {/* Virtualized Body */}
          <div style={{ flex: 1, minHeight: 0 }}>
            <List<LibraryRowProps>
              rowCount={tracks.length}
              rowHeight={ROW_HEIGHT}
              rowComponent={LibraryRow}
              rowProps={{
                tracks,
                queuedTrackIds,
                trackPlaylistMap,
                onPlayTrack,
                loadingTrackId,
                onEnqueueTrack,
                onLikeTrack,
                onRemoveFeedback,
                onOpenAddToPlaylistModal,
              }}
              style={{ height: Math.max(200, containerHeight - 42), width: "100%" }}
              overscanCount={8}
            />
          </div>
        </div>
      )}
    </div>
  );
});
