
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
              marginBottom: "var(--space-8)",
              padding: "0 var(--space-12)",
            }}
          >
            <span
              style={{
                fontSize: "var(--fs-13)",
                fontWeight: "var(--weight-semibold)",
                color: "var(--text-dim)",
              }}
            >
              TRACK
            </span>
            <span
              style={{
                fontSize: "var(--fs-13)",
                fontWeight: "var(--weight-semibold)",
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
