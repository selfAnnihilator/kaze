import React, { useEffect, useRef, useState } from "react";
import { Search, X, RefreshCw } from "lucide-react";

interface GlobalTopSearchBarProps {
  provider?: "jiosaavn" | "alternative";
  onProviderChange?: (provider: "jiosaavn" | "alternative") => void;
  placeholder?: string;
  searchQuery: string;
  setSearchQuery: (query: string) => void;
  onSearch: (queryOverride?: string) => void;
  onRefresh?: () => void;
  isSearching?: boolean;
  isRefreshing?: boolean;
}

export const GlobalTopSearchBar: React.FC<GlobalTopSearchBarProps> = React.memo(({
  searchQuery,
  provider = "jiosaavn",
  onProviderChange,
  placeholder = "Search any music online (artist, song title, album, or genre)...",
  setSearchQuery,
  onSearch,
  onRefresh: _onRefresh,
  isSearching = false,
  isRefreshing: _isRefreshing = false,
}) => {
    const [draft, setDraft] = useState(searchQuery);
    const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
    useEffect(() => {
      setDraft(searchQuery);
    }, [searchQuery]);
    useEffect(
      () => () => {
        if (timer.current) clearTimeout(timer.current);
      },
      [],
    );
    return (
    <div
      className="content-card global-search"
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
            if (timer.current) clearTimeout(timer.current);
            setSearchQuery(draft);
            onSearch(draft);
        }}
        style={{ display: "flex", alignItems: "center", gap: "10px", width: "100%",
          }}
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
          <input
            type="text"
            placeholder={placeholder}
            value={draft}
              aria-label="Search music online"
              onChange={(e) => {
                const value = e.target.value;
                setDraft(value);
                if (timer.current) clearTimeout(timer.current);
                timer.current = setTimeout(() => setSearchQuery(value), 200);
              }}
            style={{
              background: "transparent",
              border: "none",
              outline: "none",
              color: "#e8d8c9",
              fontSize: "0.95rem",
              width: "100%",
            }}
          />
          {draft && (
            <button
              type="button"
              onClick={() => {
                  if (timer.current) clearTimeout(timer.current);
                  setDraft("");
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

        {onProviderChange && <select aria-label="Online music source" value={provider} onChange={e => onProviderChange(e.target.value as "jiosaavn" | "alternative")}>
          <option value="jiosaavn">JioSaavn</option>
          <option value="alternative">Alternative sources</option>
        </select>}
        <button
          type="submit"
          className="btn btn-primary"
          disabled={isSearching}
          title="Search Online"
          style={{
            padding: "9px 12px",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            flexShrink: 0,
          }}
        >
          {isSearching ? (
            <RefreshCw size={16} className="animate-spin" />
          ) : (
            <Search size={16} />
          )}
        </button>
      </form>

    </div>
  );
},
);
