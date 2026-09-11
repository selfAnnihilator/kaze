import React from "react";
import { Disc, Calendar } from "lucide-react";
import { Album } from "../../types";

interface AlbumsViewProps {
  albums: Album[];
  onSelectAlbum: (albumId: string) => void;
}

export const AlbumsView: React.FC<AlbumsViewProps> = ({ albums, onSelectAlbum }) => {
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
          <div
            key={album.id}
            onClick={() => onSelectAlbum(album.id)}
            style={{
              backgroundColor: "var(--bg-card)",
              borderRadius: "12px",
              padding: "16px",
              display: "flex",
              flexDirection: "column",
              cursor: "pointer",
              transition: "transform 0.15s, background-color 0.15s",
              border: "1px solid var(--border)",
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.backgroundColor = "var(--bg-card-hover)";
              e.currentTarget.style.transform = "translateY(-4px)";
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.backgroundColor = "var(--bg-card)";
              e.currentTarget.style.transform = "translateY(0)";
            }}
          >
            <div
              style={{
                width: "100%",
                aspectRatio: "1/1",
                borderRadius: "8px",
                backgroundColor: "var(--bg-sidebar)",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                marginBottom: "12px",
                overflow: "hidden",
              }}
            >
              <Disc size={48} color="var(--accent-light)" />
            </div>

            <div
              style={{
                fontWeight: 600,
                fontSize: "0.95rem",
                whiteSpace: "nowrap",
                overflow: "hidden",
                textOverflow: "ellipsis",
              }}
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
            >
              {album.artist_name || "Unknown Artist"}
            </div>
            {album.year && (
              <div
                style={{
                  fontSize: "0.75rem",
                  color: "var(--text-dim)",
                  marginTop: "6px",
                  display: "flex",
                  alignItems: "center",
                  gap: "4px",
                }}
              >
                <Calendar size={12} />
                <span>{album.year}</span>
              </div>
            )}
          </div>
        ))}
      </div>
    </div>
  );
};
