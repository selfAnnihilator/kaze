import React from "react";
import { Search, X, Globe, RefreshCw } from "lucide-react";

interface GlobalTopSearchBarProps {
  searchQuery: string;
  setSearchQuery: (query: string) => void;
  onSearch: (queryOverride?: string) => void;
  onRefresh?: () => void;
  isSearching?: boolean;
  isRefreshing?: boolean;
}

const SUGGESTIONS = [
  "Top 50 Global",
  "Top 50 India",
  "Malayalam Hits",
  "Pavizha Mazha",
  "Eminem",
  "Arijit Singh",
  "Coldplay",
  "Hip-Hop",
  "EDM",
  "Taylor Swift",
];

export const GlobalTopSearchBar: React.FC<GlobalTopSearchBarProps> = ({
  searchQuery,
  setSearchQuery,
  onSearch,
  onRefresh: _onRefresh,
  isSearching = false,
  isRefreshing: _isRefreshing = false,
}) => {
  return (
    <div
      className="content-card"
      style={{
        marginBottom: "24px",
        padding: "16px 20px",
        display: "flex",
        flexDirection: "column",
        gap: "12px",
        backgroundColor: "var(--bg-card)",
        border: "1px solid var(--border)",
        borderRadius: "12px",
        boxShadow: "0 4px 20px rgba(0, 0, 0, 0.25)",
      }}
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          onSearch();
        }}
        style={{ display: "flex", alignItems: "center", gap: "10px", width: "100%" }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: "10px",
            backgroundColor: "var(--bg-main)",
            border: "1px solid var(--border)",
            borderRadius: "8px",
            padding: "9px 14px",
            flex: 1,
            transition: "border-color 0.15s ease",
          }}
        >
          <Search size={18} color="var(--text-muted)" />
          <input
            type="text"
            placeholder="Search any music online (artist, song title, album, or genre)..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            style={{
              background: "transparent",
              border: "none",
              outline: "none",
              color: "#fff",
              fontSize: "0.95rem",
              width: "100%",
            }}
          />
          {searchQuery && (
            <button
              type="button"
              onClick={() => {
                setSearchQuery("");
                onSearch("");
              }}
              style={{
                background: "none",
                border: "none",
                color: "var(--text-muted)",
                cursor: "pointer",
                padding: "2px",
                display: "flex",
                alignItems: "center",
              }}
              title="Clear search"
            >
              <X size={16} />
            </button>
          )}
        </div>

        <button
          type="submit"
          className="btn btn-primary"
          disabled={isSearching}
          style={{
            padding: "9px 18px",
            fontSize: "0.9rem",
            display: "flex",
            alignItems: "center",
            gap: "8px",
            flexShrink: 0,
          }}
        >
          {isSearching ? (
            <RefreshCw size={16} className="animate-spin" />
          ) : (
            <Globe size={16} />
          )}
          <span>Search Online</span>
        </button>
      </form>

      {/* Quick Suggestion Chips */}
      <div style={{ display: "flex", alignItems: "center", gap: "8px", flexWrap: "wrap" }}>
        <span style={{ fontSize: "0.78rem", color: "var(--text-dim)", fontWeight: 500 }}>
          Try:
        </span>
        {SUGGESTIONS.map((suggestion) => (
          <button
            key={suggestion}
            type="button"
            onClick={() => {
              setSearchQuery(suggestion);
              onSearch(suggestion);
            }}
            style={{
              fontSize: "0.75rem",
              padding: "3px 10px",
              borderRadius: "14px",
              backgroundColor: "rgba(255, 255, 255, 0.05)",
              border: "1px solid rgba(255, 255, 255, 0.08)",
              color: "var(--text-muted)",
              cursor: "pointer",
              transition: "all 0.15s ease",
            }}
            onMouseEnter={(e) => {
              e.currentTarget.style.backgroundColor = "rgba(139, 92, 246, 0.15)";
              e.currentTarget.style.borderColor = "var(--accent-light)";
              e.currentTarget.style.color = "#fff";
            }}
            onMouseLeave={(e) => {
              e.currentTarget.style.backgroundColor = "rgba(255, 255, 255, 0.05)";
              e.currentTarget.style.borderColor = "rgba(255, 255, 255, 0.08)";
              e.currentTarget.style.color = "var(--text-muted)";
            }}
          >
            {suggestion}
          </button>
        ))}
      </div>
    </div>
  );
};
