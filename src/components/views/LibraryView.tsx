import React, { useState } from "react";
import { Play, Search, RotateCw, Heart, ThumbsDown, Plus, Check } from "lucide-react";
import { Track } from "../../types";

interface LibraryViewProps {
  tracks: Track[];
  queuedTrackIds?: Set<string>;
  onPlayTrack: (trackId: string) => void;
  onEnqueueTrack: (trackId: string) => void;
  onDequeueTrack?: (trackId: string) => void;
  onLikeTrack: (trackId: string) => void;
  onDislikeTrack: (trackId: string) => void;
  onRemoveFeedback: (trackId: string) => void;
  onRescan: () => void;
  onSearch: (query: string) => void;
}

const formatSeconds = (secs: number) => {
  const m = Math.floor(secs / 60);
  const s = Math.floor(secs % 60);
  return `${m}:${s < 10 ? "0" : ""}${s}`;
};

export const LibraryView: React.FC<LibraryViewProps> = ({
  tracks,
  queuedTrackIds,
  onPlayTrack,
  onEnqueueTrack,
  onDequeueTrack,
  onLikeTrack,
  onDislikeTrack,
  onRemoveFeedback,
  onRescan,
  onSearch,
}) => {
  const [searchTerm, setSearchTerm] = useState("");

  const handleSearchChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const val = e.target.value;
    setSearchTerm(val);
    onSearch(val);
  };

  const handleQueueToggle = (trackId: string, isEnqueued: boolean) => {
    if (isEnqueued) {
      if (onDequeueTrack) onDequeueTrack(trackId);
    } else {
      onEnqueueTrack(trackId);
    }
  };

  return (
    <div>
      <div className="view-header">
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
                color: "#fff",
                fontSize: "0.88rem",
                width: "100%",
              }}
            />
          </div>

          <button className="btn btn-secondary" onClick={onRescan} title="Scan library for new files">
            <RotateCw size={16} />
            <span>Rescan</span>
          </button>
        </div>
      </div>

      {tracks.length === 0 ? (
        <div style={{ textAlign: "center", padding: "60px 0", color: "var(--text-dim)" }}>
          <p style={{ fontSize: "1.1rem", marginBottom: "8px" }}>No tracks found in library</p>
          <p style={{ fontSize: "0.88rem" }}>Add music files to your configured folder and click Rescan.</p>
        </div>
      ) : (
        <table className="track-table">
          <thead>
            <tr>
              <th style={{ width: "48px" }}>#</th>
              <th>Title</th>
              <th>Artist</th>
              <th>Album</th>
              <th>Format</th>
              <th style={{ textAlign: "right" }}>Duration</th>
              <th style={{ textAlign: "center", width: "120px" }}>Actions</th>
            </tr>
          </thead>
          <tbody>
            {tracks.map((track, idx) => {
              const isLiked = track.manual_like === 1;
              const isDisliked = track.manual_like === -1;
              const isEnqueued = queuedTrackIds ? queuedTrackIds.has(track.id) : false;

              return (
                <tr key={track.id} className="track-row" onDoubleClick={() => onPlayTrack(track.id)}>
                  <td style={{ textAlign: "center", color: "var(--text-dim)" }}>{idx + 1}</td>
                  <td className="primary">
                    <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
                      <button
                        className="player-icon-btn"
                        onClick={() => onPlayTrack(track.id)}
                        style={{ color: "var(--accent-light)" }}
                      >
                        <Play size={14} />
                      </button>
                      <span>{track.title}</span>
                    </div>
                  </td>
                  <td>{track.artist_name || "Unknown Artist"}</td>
                  <td>{track.album_title || "Unknown Album"}</td>
                  <td>
                    <span className="badge" style={{ backgroundColor: "var(--border)", color: "var(--text-muted)" }}>
                      {track.format.toUpperCase()}
                    </span>
                  </td>
                  <td style={{ textAlign: "right" }}>{formatSeconds(track.duration_secs)}</td>
                  <td>
                    <div style={{ display: "flex", justifyContent: "center", gap: "8px" }}>
                      <button
                        className="player-icon-btn"
                        title={isEnqueued ? "In queue (click to remove)" : "Add to queue"}
                        onClick={() => handleQueueToggle(track.id, isEnqueued)}
                        style={{ color: isEnqueued ? "var(--success)" : "var(--text-muted)" }}
                      >
                        {isEnqueued ? <Check size={16} /> : <Plus size={16} />}
                      </button>
                      <button
                        className="player-icon-btn"
                        title={isLiked ? "Unlike track" : "Like track"}
                        onClick={() => (isLiked ? onRemoveFeedback(track.id) : onLikeTrack(track.id))}
                        style={{ color: isLiked ? "#ef4444" : "var(--text-muted)" }}
                      >
                        <Heart size={16} fill={isLiked ? "#ef4444" : "none"} />
                      </button>
                      <button
                        className="player-icon-btn"
                        title={isDisliked ? "Remove dislike" : "Dislike track"}
                        onClick={() => (isDisliked ? onRemoveFeedback(track.id) : onDislikeTrack(track.id))}
                        style={{ color: isDisliked ? "#f59e0b" : "var(--text-muted)" }}
                      >
                        <ThumbsDown size={16} fill={isDisliked ? "#f59e0b" : "none"} />
                      </button>
                    </div>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
    </div>
  );
};
