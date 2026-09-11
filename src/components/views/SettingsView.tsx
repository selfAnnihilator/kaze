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
} from "lucide-react";

interface SettingsViewProps {
  settings: AppSettings | null;
  configuredFolders: Array<{ id: string; path: string; track_count: number }>;
  onAddFolder: (path: string) => Promise<void>;
  onRemoveFolder: (folderId: string) => Promise<void>;
  onRescanLibrary: () => Promise<void>;
  isScanning?: boolean;
}

export const SettingsView: React.FC<SettingsViewProps> = ({
  settings,
  configuredFolders,
  onAddFolder,
  onRemoveFolder,
  onRescanLibrary,
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
    <div className="p-8 space-y-6">
      {/* Header */}
      <div>
        <h1 className="text-3xl font-bold flex items-center gap-3">
          <SettingsIcon className="text-zinc-400" /> Settings & Configuration
        </h1>
        <p className="text-zinc-400 text-sm mt-1">
          Manage music root folders, audio engine defaults, external metadata providers, and Soulseek integration.
        </p>
      </div>

      {/* Navigation Tabs */}
      <div className="flex items-center gap-2 border-b border-zinc-800 pb-2 overflow-x-auto">
        <button
          onClick={() => setActiveSection("folders")}
          className={`px-3 py-1.5 rounded-lg text-sm font-semibold transition flex items-center gap-2 whitespace-nowrap ${
            activeSection === "folders" ? "bg-zinc-800 text-white" : "text-zinc-400 hover:text-white"
          }`}
        >
          <Folder size={16} /> Music Folders
        </button>
        <button
          onClick={() => setActiveSection("audio")}
          className={`px-3 py-1.5 rounded-lg text-sm font-semibold transition flex items-center gap-2 whitespace-nowrap ${
            activeSection === "audio" ? "bg-zinc-800 text-white" : "text-zinc-400 hover:text-white"
          }`}
        >
          <Volume2 size={16} /> Audio Engine
        </button>
        <button
          onClick={() => setActiveSection("metadata")}
          className={`px-3 py-1.5 rounded-lg text-sm font-semibold transition flex items-center gap-2 whitespace-nowrap ${
            activeSection === "metadata" ? "bg-zinc-800 text-white" : "text-zinc-400 hover:text-white"
          }`}
        >
          <Globe size={16} /> Metadata Providers
        </button>
        <button
          onClick={() => setActiveSection("soulseek")}
          className={`px-3 py-1.5 rounded-lg text-sm font-semibold transition flex items-center gap-2 whitespace-nowrap ${
            activeSection === "soulseek" ? "bg-zinc-800 text-white" : "text-zinc-400 hover:text-white"
          }`}
        >
          <DownloadCloud size={16} /> Soulseek / Slskd
        </button>
        <button
          onClick={() => setActiveSection("system")}
          className={`px-3 py-1.5 rounded-lg text-sm font-semibold transition flex items-center gap-2 whitespace-nowrap ${
            activeSection === "system" ? "bg-zinc-800 text-white" : "text-zinc-400 hover:text-white"
          }`}
        >
          <Database size={16} /> System & Storage
        </button>
      </div>

      {/* SECTION: FOLDERS */}
      {activeSection === "folders" && (
        <div className="space-y-6">
          <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6 space-y-4">
            <div className="flex flex-col sm:flex-row sm:items-center justify-between gap-4">
              <div>
                <h3 className="text-lg font-semibold text-white">Configured Music Folders</h3>
                <p className="text-xs text-zinc-400 mt-0.5">
                  Only audio files inside these verified directories are scanned into your local library. Subdirectories are traversed recursively, but external paths and outside symlinks are strictly rejected.
                </p>
              </div>

              <button
                onClick={() => onRescanLibrary()}
                disabled={isScanning}
                className="flex items-center gap-2 px-4 py-2 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 text-xs font-semibold rounded-lg transition border border-zinc-700 disabled:opacity-50"
              >
                <RefreshCw size={14} className={isScanning ? "animate-spin text-emerald-400" : ""} />
                <span>{isScanning ? "Scanning..." : "Rescan Library"}</span>
              </button>
            </div>

            {/* Folder List */}
            <div className="space-y-2 pt-2">
              {configuredFolders.map((folder) => (
                <div
                  key={folder.id}
                  className="flex items-center justify-between p-3.5 bg-zinc-800/60 border border-zinc-700/60 rounded-lg hover:border-zinc-600 transition"
                >
                  <div className="flex items-center gap-3 overflow-hidden">
                    <Folder className="text-emerald-400 flex-shrink-0" size={18} />
                    <div className="truncate">
                      <div className="font-mono text-sm text-zinc-200 truncate">{folder.path}</div>
                      <div className="text-xs text-zinc-500">{folder.track_count} tracks indexed</div>
                    </div>
                  </div>

                  <button
                    onClick={() => onRemoveFolder(folder.id)}
                    className="p-1.5 text-zinc-500 hover:text-rose-400 hover:bg-rose-500/10 rounded-md transition ml-2 flex-shrink-0"
                    title="Remove folder"
                  >
                    <Trash2 size={16} />
                  </button>
                </div>
              ))}
            </div>

            {/* Add Folder Form */}
            <form onSubmit={handleAddFolderSubmit} className="pt-4 border-t border-zinc-800 flex gap-3">
              <input
                type="text"
                value={newFolderPath}
                onChange={(e) => setNewFolderPath(e.target.value)}
                placeholder="/path/to/music/folder"
                className="flex-1 px-3 py-2 bg-zinc-800 border border-zinc-700 rounded-lg text-white text-sm font-mono focus:outline-none focus:border-emerald-500"
              />
              <button
                type="submit"
                disabled={addingFolder || !newFolderPath.trim()}
                className="flex items-center gap-2 px-4 py-2 bg-emerald-500 hover:bg-emerald-400 text-black font-semibold text-sm rounded-lg transition disabled:opacity-50"
              >
                <FolderPlus size={16} />
                <span>{addingFolder ? "Adding..." : "Add Folder"}</span>
              </button>
            </form>
          </div>
        </div>
      )}

      {/* SECTION: AUDIO */}
      {activeSection === "audio" && (
        <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6 space-y-6">
          <h3 className="text-lg font-semibold text-white">Audio & Playback Settings</h3>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div className="space-y-1">
              <label className="text-sm font-semibold text-zinc-300">Default Startup Volume</label>
              <div className="text-xs text-zinc-500">Initial playback volume level on launch.</div>
              <div className="text-base font-mono text-emerald-400 pt-1">
                {settings ? Math.round(settings.audio.default_volume * 100) : 80}%
              </div>
            </div>

            <div className="space-y-1">
              <label className="text-sm font-semibold text-zinc-300">Audio Output Device</label>
              <div className="text-xs text-zinc-500">Selected hardware playback device.</div>
              <div className="text-base font-mono text-zinc-300 pt-1">
                {settings?.audio.output_device || "System Default (CPAL)"}
              </div>
            </div>

            <div className="space-y-1">
              <label className="text-sm font-semibold text-zinc-300">Meaningful Play Threshold</label>
              <div className="text-xs text-zinc-500">
                Minimum seconds played before a session counts towards rankings and stats.
              </div>
              <div className="text-base font-mono text-zinc-300 pt-1">
                {settings?.history.min_meaningful_seconds || 30} seconds
              </div>
            </div>

            <div className="space-y-1">
              <label className="text-sm font-semibold text-zinc-300">Completion Threshold</label>
              <div className="text-xs text-zinc-500">
                Minimum completion percentage for full ranking credit.
              </div>
              <div className="text-base font-mono text-zinc-300 pt-1">
                {settings ? Math.round(settings.history.min_meaningful_percentage * 100) : 50}%
              </div>
            </div>
          </div>
        </div>
      )}

      {/* SECTION: METADATA */}
      {activeSection === "metadata" && (
        <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6 space-y-6">
          <h3 className="text-lg font-semibold text-white">External Metadata Providers</h3>
          <div className="space-y-4">
            <div className="flex items-center justify-between p-4 bg-zinc-800/50 rounded-lg border border-zinc-700/50">
              <div>
                <div className="font-semibold text-zinc-200">MusicBrainz & Cover Art Archive</div>
                <div className="text-xs text-zinc-400">
                  Free, open-source metadata with automated 1 req/sec rate-limiting and local disk artwork caching.
                </div>
              </div>
              <span className="inline-flex items-center gap-1 text-xs font-semibold text-emerald-400">
                <CheckCircle size={14} /> Enabled
              </span>
            </div>

            <div className="flex items-center justify-between p-4 bg-zinc-800/50 rounded-lg border border-zinc-700/50">
              <div>
                <div className="font-semibold text-zinc-200">Spotify Web API</div>
                <div className="text-xs text-zinc-400">
                  Optional OAuth Client Credentials for enhanced artist discovery. (Offline-tolerant).
                </div>
              </div>
              <span className="text-xs font-semibold text-zinc-500">
                {settings?.metadata.enable_spotify ? "Configured" : "Optional / Disabled"}
              </span>
            </div>

            <div className="p-4 bg-zinc-800/30 rounded-lg border border-zinc-800">
              <div className="text-xs text-zinc-400 uppercase font-semibold mb-2">Provider Priority Chain</div>
              <div className="flex items-center gap-2">
                {["embedded", "musicbrainz", "spotify"].map((provider, i) => (
                  <React.Fragment key={provider}>
                    <span className="px-2.5 py-1 bg-zinc-800 rounded text-xs font-mono text-zinc-300">
                      {i + 1}. {provider}
                    </span>
                    {i < 2 && <span className="text-zinc-600">&rarr;</span>}
                  </React.Fragment>
                ))}
              </div>
            </div>
          </div>
        </div>
      )}

      {/* SECTION: SOULSEEK */}
      {activeSection === "soulseek" && (
        <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6 space-y-6">
          <h3 className="text-lg font-semibold text-white">Soulseek & Slskd Integration</h3>
          <p className="text-xs text-zinc-400">
            Connect to your local Slskd daemon to enable decentralized P2P search, wishlist auto-discovery, and automatic track imports.
          </p>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div className="space-y-1">
              <label className="text-xs font-semibold uppercase text-zinc-400">Slskd Host & Port</label>
              <div className="font-mono text-sm text-zinc-200">
                {settings?.downloads.slskd_host || "localhost"}:{settings?.downloads.slskd_port || 5030}
              </div>
            </div>

            <div className="space-y-1">
              <label className="text-xs font-semibold uppercase text-zinc-400">Automatic Library Import</label>
              <div className="font-semibold text-sm text-emerald-400 flex items-center gap-1.5">
                <CheckCircle size={14} /> Auto-import on download completion
              </div>
            </div>

            <div className="space-y-1">
              <label className="text-xs font-semibold uppercase text-zinc-400">Max Concurrent Transfers</label>
              <div className="font-mono text-sm text-zinc-200">
                {settings?.downloads.max_concurrent_downloads || 2} simultaneous downloads
              </div>
            </div>
          </div>
        </div>
      )}

      {/* SECTION: SYSTEM */}
      {activeSection === "system" && (
        <div className="bg-zinc-900 border border-zinc-800 rounded-xl p-6 space-y-6">
          <h3 className="text-lg font-semibold text-white">System, Database & Cache</h3>
          <div className="space-y-4">
            <div className="p-4 bg-zinc-800/40 rounded-lg border border-zinc-800 space-y-1">
              <label className="text-xs font-semibold uppercase text-zinc-400">SQLite Database Path</label>
              <div className="font-mono text-xs text-zinc-300 break-all">
                {settings?.database_path || "Default user data dir / music_player.db"}
              </div>
            </div>

            <div className="p-4 bg-zinc-800/40 rounded-lg border border-zinc-800 space-y-1">
              <label className="text-xs font-semibold uppercase text-zinc-400">Cover Art Cache Directory</label>
              <div className="font-mono text-xs text-zinc-300 break-all">
                {settings?.cache_dir || "Default user cache dir / artwork"}
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
