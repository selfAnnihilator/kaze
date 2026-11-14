import React, { useEffect, useRef, useState } from "react";
import { Search, X, RefreshCw } from "lucide-react";

interface GlobalTopSearchBarProps {
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
  placeholder = "Search music (artist, song title, or album)...",
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
        marginBottom: "var(--space-24)",
        padding: "var(--space-16) var(--space-20)",
        display: "flex",
        flexDirection: "column",
        gap: "var(--space-12)",
        backgroundColor: "var(--bg-card)",
        border: "1px solid var(--border)",
        borderRadius: "var(--radius-12)",
        boxShadow: "0 4px 20px var(--shade-25)",
      }}
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
            if (timer.current) clearTimeout(timer.current);
            setSearchQuery(draft);
            onSearch(draft);
        }}
        style={{ display: "flex", alignItems: "center", gap: "var(--space-10)", width: "100%",
          }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "center",
            gap: "var(--space-10)",
            backgroundColor: "var(--bg-main)",
            border: "1px solid var(--border)",
            borderRadius: "var(--radius-8)",
            padding: "var(--space-10) var(--space-14)",
            flex: 1,
            transition: "border-color var(--duration-fast) ease",
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
              color: "var(--text-main)",
              fontSize: "var(--fs-15)",
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
                padding: "var(--space-2)",
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
          title="Search Online"
          style={{
            padding: "var(--space-10) var(--space-12)",
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
