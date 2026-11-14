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
            gap: "20px",
          }}
        >
          {c.distinctAlbums.map((alb) => (
            <div
              key={`${alb.artist}:${alb.album}`}
              onClick={() => c.onSelectArtist?.(`${alb.artist} ${alb.album}`)}
              style={{
                backgroundColor: "var(--bg-card)",
                border: "1px solid var(--border)",
                borderRadius: "12px",
                padding: "14px",
                cursor: "pointer",
                transition: "transform 0.18s ease",
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
                  borderRadius: "8px",
                  overflow: "hidden",
                  backgroundColor: "var(--overlay-5)",
                  marginBottom: "10px",
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
                  marginTop: "2px",
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
