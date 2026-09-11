import React from "react";
import { User } from "lucide-react";
import { Artist } from "../../types";

interface ArtistsViewProps {
  artists: Artist[];
  onSelectArtist: (artistId: string) => void;
}

export const ArtistsView: React.FC<ArtistsViewProps> = ({ artists, onSelectArtist }) => {
  return (
    <div>
      <div className="view-header">
        <div>
          <h1 className="view-title">Artists</h1>
          <p style={{ color: "var(--text-dim)", fontSize: "0.88rem", marginTop: "4px" }}>
            {artists.length} artist{artists.length === 1 ? "" : "s"} in library
          </p>
        </div>
      </div>

      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fill, minmax(180px, 1fr))",
          gap: "20px",
        }}
      >
        {artists.map((artist) => (
          <div
            key={artist.id}
            onClick={() => onSelectArtist(artist.id)}
            style={{
              backgroundColor: "var(--bg-card)",
              borderRadius: "12px",
              padding: "20px",
              display: "flex",
              flexDirection: "column",
              alignItems: "center",
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
                width: "100px",
                height: "100px",
                borderRadius: "50%",
                backgroundColor: "var(--bg-sidebar)",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                marginBottom: "14px",
                border: "2px solid var(--border-light)",
              }}
            >
              <User size={40} color="var(--accent-light)" />
            </div>

            <div style={{ fontWeight: 600, fontSize: "0.95rem", textAlign: "center" }}>
              {artist.name}
            </div>
            <div style={{ fontSize: "0.78rem", color: "var(--text-dim)", marginTop: "4px" }}>
              Artist
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};
