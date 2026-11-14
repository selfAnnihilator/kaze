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
        gap: "12px",
        borderBottom: "1px solid rgba(255, 255, 255, 0.07)",
        paddingBottom: "14px",
      }}
    >
      {/* Category Filter Pills (Spotify Desktop style) */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: "8px",
          flexWrap: "wrap",
        }}
      >
        <button
          type="button"
          onClick={() => c.setActiveTab("all")}
          style={{
            padding: "6px 14px",
            borderRadius: "9999px",
            fontSize: "0.82rem",
            fontWeight: 600,
            cursor: "pointer",
            border: "none",
            backgroundColor:
              c.activeTab === "all" ? "#e8d8c9" : "rgba(255, 255, 255, 0.08)",
            color: c.activeTab === "all" ? "#1a1714" : "#e8d8c9",
            transition: "opacity 0.15s ease",
          }}
        >
          All
        </button>
        <button
          type="button"
          onClick={() => c.setActiveTab("songs")}
          style={{
            padding: "6px 14px",
            borderRadius: "9999px",
            fontSize: "0.82rem",
            fontWeight: 600,
            cursor: "pointer",
            border: "none",
            backgroundColor:
              c.activeTab === "songs" ? "#e8d8c9" : "rgba(255, 255, 255, 0.08)",
            color: c.activeTab === "songs" ? "#1a1714" : "#e8d8c9",
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
              padding: "6px 14px",
              borderRadius: "9999px",
              fontSize: "0.82rem",
              fontWeight: 600,
              cursor: "pointer",
              border: "none",
              backgroundColor:
                c.activeTab === "artists"
                  ? "#e8d8c9"
                  : "rgba(255, 255, 255, 0.08)",
              color: c.activeTab === "artists" ? "#1a1714" : "#e8d8c9",
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
              padding: "6px 14px",
              borderRadius: "9999px",
              fontSize: "0.82rem",
              fontWeight: 600,
              cursor: "pointer",
              border: "none",
              backgroundColor:
                c.activeTab === "albums"
                  ? "#e8d8c9"
                  : "rgba(255, 255, 255, 0.08)",
              color: c.activeTab === "albums" ? "#1a1714" : "#e8d8c9",
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
              padding: "6px 14px",
              borderRadius: "9999px",
              fontSize: "0.82rem",
              fontWeight: 600,
              cursor: "pointer",
              border: "none",
              backgroundColor:
                c.activeTab === "playlists"
                  ? "#e8d8c9"
                  : "rgba(255, 255, 255, 0.08)",
              color: c.activeTab === "playlists" ? "#1a1714" : "#e8d8c9",
              transition: "opacity 0.15s ease",
            }}
          >
            Playlists ({c.matchingPlaylists.length})
          </button>
        )}
      </div>

      {/* Clear Search & Results info */}
      <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
        <span style={{ fontSize: "0.82rem", color: "var(--text-dim)" }}>
          Search results for &ldquo;
          <span style={{ color: "#e8d8c9" }}>{c.searchQuery}</span>&rdquo;
        </span>
        <button
          type="button"
          onClick={c.onClearSearch}
          className="btn btn-secondary"
          style={{
            padding: "5px 12px",
            fontSize: "0.8rem",
            display: "inline-flex",
            alignItems: "center",
            gap: "6px",
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
