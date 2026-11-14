import { ListMusic, Plus, Share2, Sparkles } from "lucide-react";
import type { PlaylistsViewContext } from "./context";

export function PlaylistsHeader({ c }: { c: PlaylistsViewContext }) {
  return (
    <div className="view-header">
      {c.viewMode === "smart_mixes" ? (
        <div>
          <h1
            className="view-title"
            style={{ display: "flex", alignItems: "center", gap: "var(--space-10)" }}
          >
            <Sparkles size={28} color="var(--accent-secondary)" />
            <span>Smart Mixes</span>
          </h1>
          <p
            style={{
              color: "var(--text-dim)",
              fontSize: "var(--fs-14)",
              marginTop: "var(--space-4)",
            }}
          >
            Auto-curated library blends, daily discovery, and genre mixes
            based on your library
          </p>
        </div>
      ) : (
        <div>
          <h1
            className="view-title"
            style={{ display: "flex", alignItems: "center", gap: "var(--space-10)" }}
          >
            <ListMusic size={28} color="var(--accent-light)" />
            <span>Playlists</span>
          </h1>
          <p
            style={{
              color: "var(--text-dim)",
              fontSize: "var(--fs-14)",
              marginTop: "var(--space-4)",
            }}
          >
            Your custom playlists and imported collections
          </p>
        </div>
      )}

      {c.viewMode !== "smart_mixes" && (
        <div style={{ display: "flex", flexWrap: "wrap", gap: "var(--space-10)" }}>
          {c.onRefreshPlaylists && <button className="btn btn-secondary" disabled={c.refreshingPlaylists} onClick={async () => {
            c.setRefreshingPlaylists(true); c.setRefreshError(null);
            try { await c.onRefreshPlaylists?.(); }
            catch (failure) { c.setRefreshError(failure instanceof Error ? failure.message : String(failure)); }
            finally { c.setRefreshingPlaylists(false); }
          }}>{c.refreshingPlaylists ? "Refreshing playlists…" : "Refresh playlists"}</button>}
          <button
            className="btn btn-secondary"
            onClick={() => c.setShowJioSaavnModal(true)}
            style={{ display: "flex", alignItems: "center", gap: "var(--space-8)" }}
          >
            <Share2 size={16} color="var(--accent-secondary)" />
            <span>Import playlist</span>
          </button>

          <button
            className="btn btn-primary"
            onClick={() => c.setShowCreateModal(true)}
          >
            <Plus size={16} />
            <span>New Playlist</span>
          </button>
        </div>
      )}
    </div>
  );
}
