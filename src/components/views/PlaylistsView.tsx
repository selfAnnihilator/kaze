import React, { useState } from "react";
import { ListMusic, Sparkles, Plus } from "lucide-react";
import { Playlist } from "../../types";

interface PlaylistsViewProps {
  playlists: Playlist[];
  onSelectPlaylist: (playlistId: string) => void;
  onCreatePlaylist: (name: string, description?: string) => void;
  onGenerateSmartMix: (mixType: string) => void;
}

export const PlaylistsView: React.FC<PlaylistsViewProps> = ({
  playlists,
  onSelectPlaylist,
  onCreatePlaylist,
  onGenerateSmartMix,
}) => {
  const [showCreateModal, setShowCreateModal] = useState(false);
  const [playlistName, setPlaylistName] = useState("");

  const handleCreate = (e: React.FormEvent) => {
    e.preventDefault();
    if (playlistName.trim()) {
      onCreatePlaylist(playlistName.trim());
      setPlaylistName("");
      setShowCreateModal(false);
    }
  };

  const smartMixPresets = [
    { type: "daily", name: "Daily Mix", desc: "Balanced blend of your current favorites & affinities" },
    { type: "on_repeat", name: "On Repeat", desc: "Your most frequently played tracks right now" },
    { type: "forgotten_favorites", name: "Forgotten Favorites", desc: "High-scoring tracks you haven't heard recently" },
    { type: "discovery", name: "Discovery Mix", desc: "Fresh unplayed local music matching your tastes" },
    { type: "late_night", name: "Late Night Mix", desc: "Downtempo, acoustic, and ambient listening" },
  ];

  return (
    <div>
      <div className="view-header">
        <div>
          <h1 className="view-title">Playlists & Smart Mixes</h1>
          <p style={{ color: "var(--text-dim)", fontSize: "0.88rem", marginTop: "4px" }}>
            Curated collections and dynamically generated local mixes
          </p>
        </div>

        <button className="btn btn-primary" onClick={() => setShowCreateModal(true)}>
          <Plus size={16} />
          <span>New Playlist</span>
        </button>
      </div>

      {/* Smart Mix Generators */}
      <div style={{ marginBottom: "36px" }}>
        <h2 style={{ fontSize: "1.1rem", fontWeight: 600, marginBottom: "16px", display: "flex", alignItems: "center", gap: "8px" }}>
          <Sparkles size={18} color="var(--accent-light)" />
          <span>Generate Smart Local Mixes</span>
        </h2>

        <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(220px, 1fr))", gap: "16px" }}>
          {smartMixPresets.map((preset) => (
            <div
              key={preset.type}
              style={{
                backgroundColor: "var(--bg-card)",
                borderRadius: "10px",
                padding: "16px",
                border: "1px solid var(--border)",
                display: "flex",
                flexDirection: "column",
                justifyContent: "space-between",
              }}
            >
              <div>
                <div style={{ fontWeight: 600, fontSize: "0.95rem", marginBottom: "4px" }}>{preset.name}</div>
                <div style={{ fontSize: "0.8rem", color: "var(--text-dim)", lineHeight: 1.4, marginBottom: "14px" }}>
                  {preset.desc}
                </div>
              </div>
              <button
                className="btn btn-secondary"
                style={{ width: "100%", justifyContent: "center", fontSize: "0.8rem" }}
                onClick={() => onGenerateSmartMix(preset.type)}
              >
                <Sparkles size={14} color="var(--accent-light)" />
                <span>Generate Mix</span>
              </button>
            </div>
          ))}
        </div>
      </div>

      {/* Saved Playlists */}
      <div>
        <h2 style={{ fontSize: "1.1rem", fontWeight: 600, marginBottom: "16px" }}>Your Playlists</h2>
        {playlists.length === 0 ? (
          <div style={{ textAlign: "center", padding: "40px 0", color: "var(--text-dim)" }}>
            <p>No playlists created yet. Create a new playlist or generate a smart mix above.</p>
          </div>
        ) : (
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(200px, 1fr))", gap: "20px" }}>
            {playlists.map((pl) => (
              <div
                key={pl.id}
                onClick={() => onSelectPlaylist(pl.id)}
                style={{
                  backgroundColor: "var(--bg-card)",
                  borderRadius: "10px",
                  padding: "16px",
                  border: "1px solid var(--border)",
                  cursor: "pointer",
                  transition: "background-color 0.15s",
                }}
                onMouseEnter={(e) => (e.currentTarget.style.backgroundColor = "var(--bg-card-hover)")}
                onMouseLeave={(e) => (e.currentTarget.style.backgroundColor = "var(--bg-card)")}
              >
                <div
                  style={{
                    width: "100%",
                    aspectRatio: "1/1",
                    borderRadius: "8px",
                    backgroundColor: "var(--bg-sidebar)",
                    display: "flex",
                    alignItems: "center",
                    justifyContent: "center",
                    marginBottom: "12px",
                  }}
                >
                  <ListMusic size={36} color={pl.is_smart_mix ? "#a78bfa" : "#9ca3af"} />
                </div>
                <div style={{ fontWeight: 600, fontSize: "0.95rem", whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" }}>
                  {pl.name}
                </div>
                <div style={{ fontSize: "0.8rem", color: "var(--text-dim)", marginTop: "4px" }}>
                  {pl.is_smart_mix ? "Smart Mix" : "Playlist"}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Modal for Creating Playlist */}
      {showCreateModal && (
        <div className="modal-overlay">
          <div className="modal-content" style={{ maxWidth: "400px" }}>
            <h3 style={{ fontSize: "1.2rem", fontWeight: 700, marginBottom: "16px" }}>Create Playlist</h3>
            <form onSubmit={handleCreate}>
              <input
                type="text"
                placeholder="Playlist name"
                value={playlistName}
                onChange={(e) => setPlaylistName(e.target.value)}
                style={{
                  width: "100%",
                  padding: "10px 14px",
                  borderRadius: "8px",
                  backgroundColor: "var(--bg-card)",
                  border: "1px solid var(--border-light)",
                  color: "#fff",
                  fontSize: "0.9rem",
                  marginBottom: "20px",
                }}
                autoFocus
              />
              <div style={{ display: "flex", gap: "12px", justifyContent: "flex-end" }}>
                <button
                  type="button"
                  className="btn btn-secondary"
                  onClick={() => setShowCreateModal(false)}
                >
                  Cancel
                </button>
                <button type="submit" className="btn btn-primary" disabled={!playlistName.trim()}>
                  Create
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
};
