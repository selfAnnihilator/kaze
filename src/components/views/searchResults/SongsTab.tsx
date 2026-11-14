
import type { OnlineSearchResultsSectionContext } from "./context";

export function SongsTab({ c }: { c: OnlineSearchResultsSectionContext }) {
  return (
    <>
      {c.activeTab === "songs" && (
        <div
          className="online-search-songs"
        >
          <div
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              marginBottom: "8px",
              padding: "0 12px",
            }}
          >
            <span
              style={{
                fontSize: "0.82rem",
                fontWeight: 600,
                color: "var(--text-dim)",
              }}
            >
              TRACK
            </span>
            <span
              style={{
                fontSize: "0.82rem",
                fontWeight: 600,
                color: "var(--text-dim)",
              }}
            >
              DURATION
            </span>
          </div>

          {c.songList}
        </div>
      )}
    </>
  );
}
