import { X } from "lucide-react";
import type { OnlineSearchResultsSectionContext } from "./context";

export function SearchResultsHeader({ c }: { c: OnlineSearchResultsSectionContext }) {
  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        flexWrap: "wrap",
        gap: "var(--space-12)",
        borderBottom: "1px solid var(--overlay-8)",
        paddingBottom: "var(--space-14)",
      }}
    >
      {/* Category Filter Pills (Spotify Desktop style) */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: "var(--space-8)",
          flexWrap: "wrap",
        }}
      >
        <button
          type="button"
          onClick={() => c.setActiveTab("all")}
          style={{
            padding: "var(--space-6) var(--space-14)",
            borderRadius: "9999px",
            fontSize: "var(--fs-13)",
            fontWeight: "var(--weight-semibold)",
            cursor: "pointer",
            border: "none",
            backgroundColor:
              c.activeTab === "all" ? "var(--text-main)" : "var(--overlay-8)",
            color: c.activeTab === "all" ? "var(--bg-main)" : "var(--text-main)",
            transition: "opacity 0.15s ease",
          }}
        >
          All
        </button>
        <button
          type="button"
          onClick={() => c.setActiveTab("songs")}
          style={{
            padding: "var(--space-6) var(--space-14)",
            borderRadius: "9999px",
            fontSize: "var(--fs-13)",
            fontWeight: "var(--weight-semibold)",
            cursor: "pointer",
            border: "none",
            backgroundColor:
              c.activeTab === "songs" ? "var(--text-main)" : "var(--overlay-8)",
            color: c.activeTab === "songs" ? "var(--bg-main)" : "var(--text-main)",
            transition: "opacity 0.15s ease",
          }}
        >
          Songs ({c.searchResults.length})
        </button>
        {c.distinctArtists.length > 0 && (
          <button
            type="button"
            onClick={() => c.setActiveTab("artists")}
            style={{
              padding: "var(--space-6) var(--space-14)",
              borderRadius: "9999px",
              fontSize: "var(--fs-13)",
              fontWeight: "var(--weight-semibold)",
              cursor: "pointer",
              border: "none",
              backgroundColor:
                c.activeTab === "artists"
                  ? "var(--text-main)"
                  : "var(--overlay-8)",
              color: c.activeTab === "artists" ? "var(--bg-main)" : "var(--text-main)",
              transition: "opacity 0.15s ease",
            }}
          >
            Artists ({c.distinctArtists.length})
          </button>
        )}
        {c.distinctAlbums.length > 0 && (
          <button
            type="button"
            onClick={() => c.setActiveTab("albums")}
            style={{
              padding: "var(--space-6) var(--space-14)",
              borderRadius: "9999px",
              fontSize: "var(--fs-13)",
              fontWeight: "var(--weight-semibold)",
              cursor: "pointer",
              border: "none",
              backgroundColor:
                c.activeTab === "albums"
                  ? "var(--text-main)"
                  : "var(--overlay-8)",
              color: c.activeTab === "albums" ? "var(--bg-main)" : "var(--text-main)",
              transition: "opacity 0.15s ease",
            }}
          >
            Albums ({c.distinctAlbums.length})
          </button>
        )}
        {c.matchingPlaylists.length > 0 && (
          <button
            type="button"
            onClick={() => c.setActiveTab("playlists")}
            style={{
              padding: "var(--space-6) var(--space-14)",
              borderRadius: "9999px",
              fontSize: "var(--fs-13)",
              fontWeight: "var(--weight-semibold)",
              cursor: "pointer",
              border: "none",
              backgroundColor:
                c.activeTab === "playlists"
                  ? "var(--text-main)"
                  : "var(--overlay-8)",
              color: c.activeTab === "playlists" ? "var(--bg-main)" : "var(--text-main)",
              transition: "opacity 0.15s ease",
            }}
          >
            Playlists ({c.matchingPlaylists.length})
          </button>
        )}
      </div>

      {/* Clear Search & Results info */}
      <div style={{ display: "flex", alignItems: "center", gap: "var(--space-12)" }}>
        <span style={{ fontSize: "var(--fs-13)", color: "var(--text-dim)" }}>
          Search results for &ldquo;
          <span style={{ color: "var(--text-main)" }}>{c.searchQuery}</span>&rdquo;
        </span>
        <button
          type="button"
          onClick={c.onClearSearch}
          className="btn btn-secondary"
          style={{
            padding: "var(--space-6) var(--space-12)",
            fontSize: "var(--fs-13)",
            display: "inline-flex",
            alignItems: "center",
            gap: "var(--space-6)",
            borderRadius: "8px",
          }}
          title="Clear search results"
        >
          <X size={14} />
          <span>Clear Search</span>
        </button>
      </div>
    </div>
  );
}
