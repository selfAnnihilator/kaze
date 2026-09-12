import React from "react";
import {
  Music,
  User,
  Disc,
  ListMusic,
  Compass,
  Bookmark,
  Download,
  Settings,
  Sparkles,
} from "lucide-react";

export type ViewType =
  | "library"
  | "artists"
  | "albums"
  | "playlists"
  | "smart_mixes"
  | "discovery"
  | "wishlist"
  | "downloads"
  | "settings";

interface SidebarProps {
  currentView: ViewType;
  onSelectView: (view: ViewType) => void;
  activeDownloadsCount?: number;
}

export const Sidebar: React.FC<SidebarProps> = ({
  currentView,
  onSelectView,
  activeDownloadsCount = 0,
}) => {
  return (
    <aside className="sidebar">
      <div className="logo-container">
        <Sparkles size={24} color="#8b5cf6" />
        <span className="logo-title">SoundFlow</span>
      </div>

      <div className="nav-section" style={{ marginTop: "6px" }}>
        <button
          className={`nav-button ${currentView === "discovery" ? "active" : ""}`}
          onClick={() => onSelectView("discovery")}
        >
          <Compass size={18} />
          <span>Discover Music</span>
        </button>
      </div>

      <div className="nav-section">
        <span className="nav-section-title">Local Library</span>
        <button
          className={`nav-button ${currentView === "library" ? "active" : ""}`}
          onClick={() => onSelectView("library")}
        >
          <Music size={18} />
          <span>Downloaded Tracks</span>
        </button>
        <button
          className={`nav-button ${currentView === "artists" ? "active" : ""}`}
          onClick={() => onSelectView("artists")}
        >
          <User size={18} />
          <span>Artists</span>
        </button>
        <button
          className={`nav-button ${currentView === "albums" ? "active" : ""}`}
          onClick={() => onSelectView("albums")}
        >
          <Disc size={18} />
          <span>Albums</span>
        </button>
      </div>

      <div className="nav-section">
        <span className="nav-section-title">Playlists & Mixes</span>
        <button
          className={`nav-button ${currentView === "playlists" ? "active" : ""}`}
          onClick={() => onSelectView("playlists")}
        >
          <ListMusic size={18} />
          <span>Playlists</span>
        </button>
      </div>

      <div className="nav-section">
        <span className="nav-section-title">Acquisition</span>
        <button
          className={`nav-button ${currentView === "wishlist" ? "active" : ""}`}
          onClick={() => onSelectView("wishlist")}
        >
          <Bookmark size={18} />
          <span>Wishlist</span>
        </button>
        <button
          className={`nav-button ${currentView === "downloads" ? "active" : ""}`}
          onClick={() => onSelectView("downloads")}
        >
          <Download size={18} />
          <span>Downloads</span>
          {activeDownloadsCount > 0 && (
            <span
              style={{
                marginLeft: "auto",
                backgroundColor: "#06b6d4",
                color: "#000",
                fontSize: "10px",
                fontWeight: 700,
                borderRadius: "9999px",
                padding: "1px 7px",
                lineHeight: "15px",
                animation: "pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite",
              }}
            >
              {activeDownloadsCount}
            </span>
          )}
        </button>
      </div>

      <div style={{ marginTop: "auto" }}>
        <button
          className={`nav-button ${currentView === "settings" ? "active" : ""}`}
          onClick={() => onSelectView("settings")}
        >
          <Settings size={18} />
          <span>Settings</span>
        </button>
      </div>
    </aside>
  );
};
