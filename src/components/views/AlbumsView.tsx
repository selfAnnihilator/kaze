import React, { useState, useEffect } from "react";
import { Disc, Calendar } from "lucide-react";
import { Album, Track } from "../../types";
import { executeQuery } from "../../services/api";

interface AlbumCardProps {
  album: Album;
  onSelect: () => void;
  tracks?: Track[];
}

const AlbumCard: React.FC<AlbumCardProps> = ({ album, onSelect, tracks }) => {
  const [coverUrl, setCoverUrl] = useState<string | null>(album.cover_art_path || null);
  const [isHovered, setIsHovered] = useState(false);

  useEffect(() => {
    if (coverUrl) return;

    let targetTrackId = album.first_track_id;
    if (!targetTrackId && tracks && tracks.length > 0) {
      const found = tracks.find(
        (t) =>
          (t.album_title && t.album_title.toLowerCase() === album.title.toLowerCase()) ||
          t.id === album.first_track_id
      );
      if (found) targetTrackId = found.id;
    }

    if (!targetTrackId) return;

    let isMounted = true;
    executeQuery({
      query: "GetTrackCoverArt",
      payload: { track_id: targetTrackId },
    })
      .then((res) => {
        if (isMounted && res.data) {
          setCoverUrl(res.data);
        }
      })
      .catch((_err) => {});

    return () => {
      isMounted = false;
    };
  }, [album.id, album.title, album.first_track_id, album.cover_art_path, coverUrl, tracks]);

  return (
    <div
      onClick={onSelect}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      style={{
        backgroundColor: "var(--bg-card)",
        borderRadius: "12px",
        padding: "14px",
        display: "flex",
        flexDirection: "column",
        cursor: "pointer",
        transition: "all 0.2s cubic-bezier(0.4, 0, 0.2, 1)",
        transform: isHovered ? "translateY(-4px)" : "none",
        boxShadow: isHovered ? "0 10px 24px rgba(0, 0, 0, 0.4)" : "0 2px 8px rgba(0, 0, 0, 0.15)",
        border: isHovered ? "1px solid rgba(255, 255, 255, 0.22)" : "1px solid var(--border)",
      }}
    >
      <div
        style={{
          width: "100%",
          aspectRatio: "1/1",
          borderRadius: "8px",
          backgroundColor: "rgba(255, 255, 255, 0.04)",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          marginBottom: "12px",
          overflow: "hidden",
          position: "relative",
        }}
      >
        {coverUrl ? (
          <img
            src={coverUrl}
            alt={album.title}
            style={{
              width: "100%",
              height: "100%",
              objectFit: "cover",
              display: "block",
            }}
          />
        ) : (
          <div
            style={{
              width: "100%",
              height: "100%",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              background: "linear-gradient(135deg, rgba(99, 102, 241, 0.15), rgba(168, 85, 247, 0.1))",
            }}
          >
            <Disc size={44} color="var(--accent-light)" />
          </div>
        )}
      </div>

      <div
        style={{
          fontWeight: 700,
          fontSize: "0.95rem",
          whiteSpace: "nowrap",
          overflow: "hidden",
          textOverflow: "ellipsis",
        }}
        title={album.title}
      >
        {album.title}
      </div>
      <div
        style={{
          fontSize: "0.82rem",
          color: "var(--text-muted)",
          marginTop: "2px",
          whiteSpace: "nowrap",
          overflow: "hidden",
          textOverflow: "ellipsis",
        }}
        title={album.artist_name || "Unknown Artist"}
      >
        {album.artist_name || "Unknown Artist"}
      </div>
      <div
        style={{
          fontSize: "0.75rem",
          color: "var(--text-dim)",
          marginTop: "6px",
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
        }}
      >
        {album.track_count !== undefined ? (
          <span>
            {album.track_count} {album.track_count === 1 ? "track" : "tracks"}
          </span>
        ) : (
          <span />
        )}
        {album.year && (
          <div style={{ display: "flex", alignItems: "center", gap: "4px" }}>
            <Calendar size={12} />
            <span>{album.year}</span>
          </div>
        )}
      </div>
    </div>
  );
};

interface AlbumsViewProps {
  albums: Album[];
  tracks?: Track[];
  onSelectAlbum: (albumId: string) => void;
}

export const AlbumsView: React.FC<AlbumsViewProps> = ({ albums, tracks, onSelectAlbum }) => {
  return (
    <div>
      <div className="view-header">
        <div>
          <h1 className="view-title">Albums</h1>
          <p style={{ color: "var(--text-dim)", fontSize: "0.88rem", marginTop: "4px" }}>
            {albums.length} album{albums.length === 1 ? "" : "s"} in library
          </p>
        </div>
      </div>

      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fill, minmax(200px, 1fr))",
          gap: "24px",
        }}
      >
        {albums.map((album) => (
          <AlbumCard
            key={album.id}
            album={album}
            tracks={tracks}
            onSelect={() => onSelectAlbum(album.id)}
          />
        ))}
      </div>
    </div>
  );
};
