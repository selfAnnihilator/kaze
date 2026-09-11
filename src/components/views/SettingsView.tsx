import React, { useState } from "react";
import { AppSettings } from "../../types";
import {
  Settings as SettingsIcon,
  FolderPlus,
  Trash2,
  RefreshCw,
  Folder,
  Volume2,
  Globe,
  DownloadCloud,
  Database,
  CheckCircle,
  Sparkles,
  ExternalLink,
} from "lucide-react";

interface SettingsViewProps {
  settings: AppSettings | null;
  configuredFolders: Array<{ id: string; path: string; track_count?: number }>;
  onAddFolder: (path: string) => Promise<void>;
  onRemoveFolder: (folderId: string) => Promise<void>;
  onRescanLibrary: () => Promise<void>;
  onRerunOnboarding: () => void;
  onLaunchSoulseek: (query?: string) => Promise<void>;
  onImportSoulseek: () => Promise<void>;
  isScanning?: boolean;
}

export const SettingsView: React.FC<SettingsViewProps> = ({
  settings,
  configuredFolders,
  onAddFolder,
  onRemoveFolder,
  onRescanLibrary,
  onRerunOnboarding,
  onLaunchSoulseek,
  onImportSoulseek,
  isScanning = false,
}) => {
  const [newFolderPath, setNewFolderPath] = useState("");
  const [addingFolder, setAddingFolder] = useState(false);
  const [activeSection, setActiveSection] = useState<"folders" | "audio" | "metadata" | "soulseek" | "system">("folders");

  const handleAddFolderSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newFolderPath.trim()) return;
    setAddingFolder(true);
    try {
      await onAddFolder(newFolderPath.trim());
      setNewFolderPath("");
    } finally {
      setAddingFolder(false);
    }
  };

  return (
    <div>
      {/* Header */}
      <div className="view-header">
        <div>
          <h1 className="view-title" style={{ display: "flex", alignItems: "center", gap: "10px" }}>
            <SettingsIcon size={28} color="var(--text-muted)" />
            <span>Settings & Configuration</span>
          </h1>
          <p style={{ color: "var(--text-dim)", fontSize: "0.88rem", marginTop: "4px" }}>
            Manage music root directories, audio engine parameters, metadata providers, and Soulseek integration
          </p>
        </div>
      </div>

      {/* Navigation Tabs */}
      <div className="tab-bar">
        <button
          onClick={() => setActiveSection("folders")}
          className={`tab-btn ${activeSection === "folders" ? "active" : ""}`}
        >
          <Folder size={16} />
          <span>Music Folders</span>
        </button>
        <button
          onClick={() => setActiveSection("audio")}
          className={`tab-btn ${activeSection === "audio" ? "active" : ""}`}
        >
          <Volume2 size={16} />
          <span>Audio Engine</span>
        </button>
        <button
          onClick={() => setActiveSection("metadata")}
          className={`tab-btn ${activeSection === "metadata" ? "active" : ""}`}
        >
          <Globe size={16} />
          <span>Metadata Providers</span>
        </button>
        <button
          onClick={() => setActiveSection("soulseek")}
          className={`tab-btn ${activeSection === "soulseek" ? "active" : ""}`}
        >
          <DownloadCloud size={16} />
          <span>Soulseek / Slskd</span>
        </button>
        <button
          onClick={() => setActiveSection("system")}
          className={`tab-btn ${activeSection === "system" ? "active" : ""}`}
        >
          <Database size={16} />
          <span>System & Storage</span>
        </button>
      </div>

      {/* SECTION: FOLDERS */}
      {activeSection === "folders" && (
        <div className="content-card">
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "16px" }}>
            <div>
              <h3 style={{ fontSize: "1.1rem", fontWeight: 600, color: "#fff" }}>Configured Music Folders</h3>
              <p style={{ fontSize: "0.82rem", color: "var(--text-muted)", marginTop: "4px" }}>
                All child directories and subfolders are scanned recursively. Unapproved paths and outside symlinks are strictly rejected.
              </p>
            </div>

            <div style={{ display: "flex", gap: "10px" }}>
              <button
                onClick={() => onRerunOnboarding()}
                className="btn btn-secondary"
                title="Test initial onboarding flow with default or custom music directory"
              >
                <Sparkles size={15} color="var(--accent-light)" />
                <span>Re-run Onboarding Setup</span>
              </button>

              <button
                onClick={() => onRescanLibrary()}
                disabled={isScanning}
                className="btn btn-secondary"
              >
                <RefreshCw size={15} className={isScanning ? "animate-spin" : ""} color={isScanning ? "var(--accent)" : "currentColor"} />
                <span>{isScanning ? "Scanning Library..." : "Rescan Library"}</span>
              </button>
            </div>
          </div>

          {/* Folder List */}
          <div style={{ marginBottom: "20px" }}>
            {configuredFolders.map((folder) => (
              <div key={folder.id} className="folder-item">
                <div style={{ display: "flex", alignItems: "center", gap: "12px", overflow: "hidden" }}>
                  <Folder color="var(--accent-light)" size={20} style={{ flexShrink: 0 }} />
                  <div>
                    <div style={{ fontFamily: "monospace", fontSize: "0.9rem", color: "var(--text-main)" }}>
                      {folder.path}
                    </div>
                    <div style={{ fontSize: "0.78rem", color: "var(--text-dim)", marginTop: "2px" }}>
                      {folder.track_count ?? 0} tracks indexed
                    </div>
                  </div>
                </div>

                <button
                  onClick={() => onRemoveFolder(folder.id)}
                  style={{ background: "none", border: "none", color: "var(--danger)", cursor: "pointer", padding: "6px" }}
                  title="Remove folder"
                >
                  <Trash2 size={16} />
                </button>
              </div>
            ))}
          </div>

          {/* Add Folder Form */}
          <form onSubmit={handleAddFolderSubmit} style={{ display: "flex", gap: "10px", borderTop: "1px solid var(--border)", paddingTop: "16px" }}>
            <input
              type="text"
              value={newFolderPath}
              onChange={(e) => setNewFolderPath(e.target.value)}
              placeholder="/path/to/music/folder"
              className="input-field"
              style={{ flex: 1, fontFamily: "monospace" }}
            />
            <button
              type="submit"
              disabled={addingFolder || !newFolderPath.trim()}
              className="btn btn-primary"
            >
              <FolderPlus size={16} />
              <span>{addingFolder ? "Adding..." : "Add Folder"}</span>
            </button>
          </form>
        </div>
      )}

      {/* SECTION: AUDIO */}
      {activeSection === "audio" && (
        <div className="content-card">
          <h3 style={{ fontSize: "1.1rem", fontWeight: 600, color: "#fff", marginBottom: "16px" }}>
            Audio & Playback Parameters
          </h3>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "20px" }}>
            <div>
              <label style={{ fontSize: "0.85rem", fontWeight: 600, color: "var(--text-muted)", display: "block" }}>
                Default Volume
              </label>
              <div style={{ fontSize: "1.2rem", fontWeight: 700, color: "var(--accent-light)", marginTop: "4px" }}>
                {settings ? Math.round(settings.audio.default_volume * 100) : 80}%
              </div>
            </div>

            <div>
              <label style={{ fontSize: "0.85rem", fontWeight: 600, color: "var(--text-muted)", display: "block" }}>
                Audio Output Device
              </label>
              <div style={{ fontSize: "0.95rem", color: "var(--text-main)", marginTop: "4px" }}>
                {settings?.audio.output_device || "System Default (CPAL)"}
              </div>
            </div>

            <div>
              <label style={{ fontSize: "0.85rem", fontWeight: 600, color: "var(--text-muted)", display: "block" }}>
                Meaningful Play Threshold
              </label>
              <div style={{ fontSize: "0.95rem", color: "var(--text-main)", marginTop: "4px" }}>
                {settings?.history.min_meaningful_seconds || 30} seconds
              </div>
            </div>

            <div>
              <label style={{ fontSize: "0.85rem", fontWeight: 600, color: "var(--text-muted)", display: "block" }}>
                Completion Threshold
              </label>
              <div style={{ fontSize: "0.95rem", color: "var(--text-main)", marginTop: "4px" }}>
                {settings ? Math.round(settings.history.min_meaningful_percentage * 100) : 50}%
              </div>
            </div>
          </div>
        </div>
      )}

      {/* SECTION: METADATA */}
      {activeSection === "metadata" && (
        <div className="content-card">
          <h3 style={{ fontSize: "1.1rem", fontWeight: 600, color: "#fff", marginBottom: "16px" }}>
            External Metadata Providers
          </h3>
          <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
            <div className="folder-item">
              <div>
                <div style={{ fontWeight: 600, fontSize: "0.95rem" }}>MusicBrainz & Cover Art Archive</div>
                <div style={{ fontSize: "0.8rem", color: "var(--text-dim)", marginTop: "2px" }}>
                  1 req/sec rate-limiting with permanent disk artwork caching
                </div>
              </div>
              <span className="badge badge-exact">Enabled</span>
            </div>

            <div className="folder-item">
              <div>
                <div style={{ fontWeight: 600, fontSize: "0.95rem" }}>Spotify Web API</div>
                <div style={{ fontSize: "0.8rem", color: "var(--text-dim)", marginTop: "2px" }}>
                  Optional Client Credentials OAuth for external recommendations
                </div>
              </div>
              <span className="badge" style={{ backgroundColor: "rgba(255,255,255,0.06)", color: "var(--text-muted)" }}>
                {settings?.metadata.enable_spotify ? "Configured" : "Optional"}
              </span>
            </div>
          </div>
        </div>
      )}

      {/* SECTION: SOULSEEK */}
      {activeSection === "soulseek" && (
        <div className="content-card">
          <h3 style={{ fontSize: "1.1rem", fontWeight: 600, color: "#fff", marginBottom: "16px" }}>
            Soulseek & SoulseekQt Integration
          </h3>
          <p style={{ fontSize: "0.85rem", color: "var(--text-muted)", marginBottom: "20px" }}>
            SoundFlow integrates directly with your system's SoulseekQt desktop installation and local download repository.
          </p>

          {/* SoulseekQt Native Integration Card */}
          <div
            style={{
              padding: "16px",
              borderRadius: "8px",
              backgroundColor: "rgba(139, 92, 246, 0.08)",
              border: "1px solid var(--accent-light)",
              marginBottom: "24px",
            }}
          >
            <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", flexWrap: "wrap", gap: "12px" }}>
              <div>
                <div style={{ fontWeight: 600, fontSize: "1rem", display: "flex", alignItems: "center", gap: "8px" }}>
                  <DownloadCloud size={18} color="var(--accent-light)" />
                  <span>SoulseekQt Desktop Application</span>
                  <span className="badge badge-exact">Detected</span>
                </div>
                <div style={{ fontSize: "0.82rem", color: "var(--text-dim)", fontFamily: "monospace", marginTop: "6px" }}>
                  Binary: /home/abhi/Applications/SoulseekQt-2024-6-30.AppImage
                </div>
                <div style={{ fontSize: "0.82rem", color: "var(--text-dim)", fontFamily: "monospace", marginTop: "2px" }}>
                  Downloads: ~/Soulseek Downloads/complete
                </div>
              </div>

              <div style={{ display: "flex", gap: "10px" }}>
                <button
                  type="button"
                  className="btn btn-primary"
                  onClick={() => onLaunchSoulseek()}
                >
                  <ExternalLink size={15} />
                  <span>Launch SoulseekQt</span>
                </button>
                <button
                  type="button"
                  className="btn btn-secondary"
                  onClick={() => onImportSoulseek()}
                >
                  <RefreshCw size={15} />
                  <span>Import Completed Downloads</span>
                </button>
              </div>
            </div>
          </div>

          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: "20px" }}>
            <div>
              <label style={{ fontSize: "0.82rem", fontWeight: 600, color: "var(--text-dim)", textTransform: "uppercase" }}>
                Slskd Daemon Fallback Host & Port
              </label>
              <div style={{ fontFamily: "monospace", fontSize: "0.95rem", marginTop: "4px" }}>
                {settings?.downloads.slskd_host || "localhost"}:{settings?.downloads.slskd_port || 5030}
              </div>
            </div>

            <div>
              <label style={{ fontSize: "0.82rem", fontWeight: 600, color: "var(--text-dim)", textTransform: "uppercase" }}>
                Automatic Library Import
              </label>
              <div style={{ color: "var(--success)", display: "flex", alignItems: "center", gap: "6px", fontSize: "0.9rem", marginTop: "4px" }}>
                <CheckCircle size={16} /> Auto-import on download completion
              </div>
            </div>
          </div>
        </div>
      )}

      {/* SECTION: SYSTEM */}
      {activeSection === "system" && (
        <div className="content-card">
          <h3 style={{ fontSize: "1.1rem", fontWeight: 600, color: "#fff", marginBottom: "16px" }}>
            System, Database & Cache
          </h3>
          <div style={{ display: "flex", flexDirection: "column", gap: "12px" }}>
            <div className="folder-item">
              <div>
                <div style={{ fontSize: "0.78rem", color: "var(--text-dim)", textTransform: "uppercase", fontWeight: 600 }}>
                  SQLite Database Path
                </div>
                <div style={{ fontFamily: "monospace", fontSize: "0.85rem", marginTop: "4px" }}>
                  {settings?.database_path || "Default user data dir / music_player.db"}
                </div>
              </div>
            </div>

            <div className="folder-item">
              <div>
                <div style={{ fontSize: "0.78rem", color: "var(--text-dim)", textTransform: "uppercase", fontWeight: 600 }}>
                  Cover Art Cache Directory
                </div>
                <div style={{ fontFamily: "monospace", fontSize: "0.85rem", marginTop: "4px" }}>
                  {settings?.cache_dir || "Default user cache dir / artwork"}
                </div>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
