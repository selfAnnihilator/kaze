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
            gap: "var(--space-20)",
          }}
        >
          {c.matchingPlaylists.map((pl) => (
            <div
              key={pl.id}
              style={{
                backgroundColor: "var(--bg-card)",
                border: "1px solid var(--border)",
                borderRadius: "var(--radius-12)",
                padding: "var(--space-16)",
                cursor: "pointer",
              }}
            >
              <div
                style={{
                  width: "100%",
                  aspectRatio: "1/1",
                  borderRadius: "var(--radius-8)",
                  backgroundColor: "var(--accent-a15)",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  marginBottom: "var(--space-12)",
                }}
              >
                <ListMusic size={40} color="var(--accent-light)" />
              </div>
              <div
                style={{
                  fontWeight: "var(--weight-bold)",
                  fontSize: "var(--fs-15)",
                  color: "var(--text-main)",
                }}
              >
                {pl.name}
              </div>
              <div
                style={{
                  fontSize: "var(--fs-13)",
                  color: "var(--text-muted)",
                  marginTop: "var(--space-4)",
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
