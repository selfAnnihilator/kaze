import { ListMusic } from "lucide-react";
import type { OnlineSearchResultsSectionContext } from "./context";

export function PlaylistsTab({ c }: { c: OnlineSearchResultsSectionContext }) {
  return (
    <>
      {c.activeTab === "playlists" && (
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fill, minmax(200px, 1fr))",
            gap: "20px",
          }}
        >
          {c.matchingPlaylists.map((pl) => (
            <div
              key={pl.id}
              style={{
                backgroundColor: "var(--bg-card)",
                border: "1px solid var(--border)",
                borderRadius: "12px",
                padding: "16px",
                cursor: "pointer",
              }}
            >
              <div
                style={{
                  width: "100%",
                  aspectRatio: "1/1",
                  borderRadius: "8px",
                  backgroundColor: "rgba(243, 112, 30, 0.15)",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  marginBottom: "12px",
                }}
              >
                <ListMusic size={40} color="var(--accent-light)" />
              </div>
              <div
                style={{
                  fontWeight: 700,
                  fontSize: "0.95rem",
                  color: "#e8d8c9",
                }}
              >
                {pl.name}
              </div>
              <div
                style={{
                  fontSize: "0.8rem",
                  color: "var(--text-muted)",
                  marginTop: "4px",
                }}
              >
                {pl.track_count || 0} tracks
              </div>
            </div>
          ))}
        </div>
      )}
    </>
  );
}
