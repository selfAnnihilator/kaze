import { User } from "lucide-react";
import type { OnlineSearchResultsSectionContext } from "./context";

export function ArtistsTab({ c }: { c: OnlineSearchResultsSectionContext }) {
  return (
    <>
      {c.activeTab === "artists" && (
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fill, minmax(160px, 1fr))",
            gap: "var(--space-20)",
          }}
        >
          {c.distinctArtists.map((a) => (
            <div
              key={a.artist}
              onClick={() => c.onSelectArtist?.(a.artist)}
              style={{
                backgroundColor: "var(--bg-card)",
                border: "1px solid var(--border)",
                borderRadius: "var(--radius-12)",
                padding: "var(--space-16)",
                textAlign: "center",
                cursor: "pointer",
                transition: "transform var(--duration-quick) ease",
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.borderColor = "var(--accent-light)";
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.borderColor = "var(--border)";
              }}
            >
              <div
                style={{
                  width: "88px",
                  height: "88px",
                  borderRadius: "50%",
                  overflow: "hidden",
                  margin: "0 auto var(--space-12) auto",
                  backgroundColor: "var(--accent-a15)",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                }}
              >
                {a.cover ? (
                  <img decoding="async"
                    src={a.cover}
                    alt={a.artist}
                    style={{
                      width: "100%",
                      height: "100%",
                      objectFit: "cover",
                    }}
                  />
                ) : (
                  <User size={36} color="var(--accent-light)" />
                )}
              </div>
              <div
                style={{
                  fontWeight: "var(--weight-bold)",
                  fontSize: "var(--fs-15)",
                  color: "var(--text-main)",
                  whiteSpace: "nowrap",
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                }}
              >
                {a.artist}
              </div>
              <div
                style={{
                  fontSize: "var(--fs-12)",
                  color: "var(--text-muted)",
                  marginTop: "var(--space-4)",
                }}
              >
                Artist • {a.count} track{a.count === 1 ? "" : "s"}
              </div>
            </div>
          ))}
        </div>
      )}
    </>
  );
}
