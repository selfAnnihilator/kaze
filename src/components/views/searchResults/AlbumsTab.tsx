import { CoverOrDisc } from "./CoverOrDisc";
import type { OnlineSearchResultsSectionContext } from "./context";

export function AlbumsTab({ c }: { c: OnlineSearchResultsSectionContext }) {
  return (
    <>
      {c.activeTab === "albums" && (
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fill, minmax(180px, 1fr))",
            gap: "var(--space-20)",
          }}
        >
          {c.distinctAlbums.map((alb) => (
            <div
              key={`${alb.artist}:${alb.album}`}
              onClick={() => c.onSelectArtist?.(`${alb.artist} ${alb.album}`)}
              style={{
                backgroundColor: "var(--bg-card)",
                border: "1px solid var(--border)",
                borderRadius: "var(--radius-12)",
                padding: "var(--space-14)",
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
                  width: "100%",
                  aspectRatio: "1/1",
                  borderRadius: "var(--radius-8)",
                  overflow: "hidden",
                  backgroundColor: "var(--overlay-5)",
                  marginBottom: "var(--space-10)",
                }}
              >
                <CoverOrDisc src={alb.cover} alt={alb.album} iconSize={36} />
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
                {alb.album}
              </div>
              <div
                style={{
                  fontSize: "var(--fs-13)",
                  color: "var(--text-muted)",
                  marginTop: "var(--space-2)",
                  whiteSpace: "nowrap",
                  overflow: "hidden",
                  textOverflow: "ellipsis",
                }}
              >
                {alb.artist}
              </div>
            </div>
          ))}
        </div>
      )}
    </>
  );
}
