import React, { useState } from "react";
import { Bookmark, Plus, Search, ExternalLink } from "lucide-react";
import type { WishlistItem } from "../../types";
import { useDebouncedValue } from "../../presentation/useDebouncedValue";
import { WindowedList } from "../../presentation/WindowedList";

interface WishlistViewProps {
  wishlist: WishlistItem[];
  onAddToWishlist: (title: string, artist: string, album?: string,
  ) => Promise<void>;
  onUpdateStatus: (
    id: string,
    status: "want" | "ignore" | "already_own" | "downloaded",
  ) => Promise<void>;
  onSearchDirect: (artist: string, title: string, album?: string) => void;
  onLaunchSoulseek?: (query?: string, filter?: string) => Promise<void>;
}
const statuses = ["WANT", "DOWNLOADED", "ALREADY_OWN", "IGNORE"] as const;
const label = (status: string) => status.toLowerCase().replace(/_/g, " ");

export const WishlistView = React.memo(function WishlistView({
  wishlist,
  onAddToWishlist,
  onUpdateStatus,
  onSearchDirect,
  onLaunchSoulseek,
}: WishlistViewProps) {
  const [filter, setFilter] = useState("ALL");
  const [search, setSearch] = useState("");
  const query = useDebouncedValue(search).toLowerCase();
  const [adding, setAdding] = useState(false);
  const [saving, setSaving] = useState(false);

  const [error, setError] = useState<string | null>(null);
  const filtered = wishlist.filter((item) =>
      (filter === "ALL" || item.status === filter) &&
      `${item.title} ${item.artist} ${item.album || ""}`
        .toLowerCase()
        .includes(query),
  );
  return (
    <section className="wishlist-view">
      <header className="view-header">
        <div>
          <p className="interface-eyebrow">Find your next record</p>
          <h1 className="view-title">
            <Bookmark size={26} /> Wishlist
          </h1>
          <p>{wishlist.length} tracks saved for later</p>
        </div>
        <button
          className="btn btn-primary"
          onClick={() => setAdding((value) => !value)}
        >
          <Plus size={16} /> Add music
        </button>
      </header>
      {error && <p role="alert">{error}</p>}
      {adding && (
        <form
          className="wishlist-form content-card"
          onSubmit={async (e) => {
            e.preventDefault();
            const data = new FormData(e.currentTarget);
            setSaving(true);
            setError(null);
            try {
              await onAddToWishlist(
                String(data.get("title")).trim(),
                String(data.get("artist")).trim(),
                String(data.get("album") || "").trim() || undefined,
              );
              setAdding(false);
            } catch {
              setError("Couldn’t add this track. Try again.");
            } finally {
              setSaving(false);
            }
          }}
        >
          <label>
            Title
            <input name="title" required className="input-field" />
          </label>
          <label>
            Artist
            <input name="artist" required className="input-field" />
          </label>
          <label>
            Album
            <input name="album" className="input-field" />
          </label>
          <button
            type="button"
            className="btn btn-secondary"
            onClick={() => setAdding(false)}
          >
            Cancel
          </button>
          <button className="btn btn-primary" disabled={saving}>
            Save to wishlist
          </button>
        </form>
      )}
      <div className="wishlist-filters">
        <div className="tab-bar">
          {["ALL", ...statuses].map((status) => (
            <button
              className={`tab-btn ${filter === status ? "active" : ""}`}
              key={status}
              onClick={() => setFilter(status)}
            >
              {label(status)}
            </button>
          ))}
        </div>
        <label className="wishlist-search">
          <Search size={16} />
          <input
            aria-label="Search wishlist"
            className="input-field"
            placeholder="Search your wishlist"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </label>
      </div>
      {!filtered.length ? (
        <p className="interface-empty">
          No wishlist items found. Save missing music from Discovery or add a
          track here.
        </p>
      ) : (
        <WindowedList
          items={filtered}
          rowHeight={100}
          label="Wishlist tracks"
          renderItem={(item) => (
            <div className="wishlist-row" key={item.id}>
              <div className="wishlist-song">
                <strong>{item.title}</strong>
                <span>
                  {item.artist}
                  {item.album ? ` · ${item.album}` : ""}
                </span>
              </div>
              <select
                aria-label={`Status for ${item.title}`}
                value={item.status}
                onChange={async (e) => {
                  setError(null);
                  try {
                    await onUpdateStatus(
                      item.id,
                      e.target.value.toLowerCase() as
                        "want" | "ignore" | "already_own" | "downloaded",
                    );
                  } catch {
                    setError("Couldn’t update this track’s status. Try again.");
                  }
                }}
              >
                {statuses.map((status) => (
                  <option key={status} value={status}>
                    {label(status)}
                  </option>
                ))}
              </select>
              <button
                className="btn btn-secondary"
                title={`Search downloads for ${item.title}`}
                onClick={() =>
                  onSearchDirect(item.artist, item.title, item.album)
                }
              >
                <Search size={15} /> Find
              </button>
              {onLaunchSoulseek && (
                <button
                  className="player-icon-btn"
                  aria-label={`Find ${item.title} in Soulseek`}
                  onClick={() =>
                    void onLaunchSoulseek(item.title, item.artist).catch(() =>
                      setError("Couldn’t launch Soulseek. Check Settings."),
                    )
                  }
                >
                  <ExternalLink size={16} />
                </button>
              )}
            </div>
          )}
        />
      )}
    </section>
  );
});
