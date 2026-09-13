import React from "react";
import {
  Music,
  Disc,
  ListMusic,
  Compass,
  Bookmark,
  Bell,
  Settings,
  Sparkles,
} from "lucide-react";

export type ViewType =
  | "library"
  | "albums"
  | "playlists"
  | "smart_mixes"
  | "discovery"
  | "wishlist"
  | "notifications"
  | "settings";

interface SidebarProps {
  currentView: ViewType;
  onSelectView: (view: ViewType) => void;
  unreadNotificationsCount?: number;
}

export const Sidebar: React.FC<SidebarProps> = ({
  currentView,
  onSelectView,
  unreadNotificationsCount = 0,
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
        <span className="nav-section-title">Activity</span>
        <button
          className={`nav-button ${currentView === "wishlist" ? "active" : ""}`}
          onClick={() => onSelectView("wishlist")}
        >
          <Bookmark size={18} />
          <span>Wishlist</span>
        </button>
        <button
          className={`nav-button ${currentView === "notifications" ? "active" : ""}`}
          onClick={() => onSelectView("notifications")}
        >
          <Bell size={18} />
          <span>Notifications</span>
          {unreadNotificationsCount > 0 && (
            <span
              style={{
                marginLeft: "auto",
                backgroundColor: "#ef4444",
                color: "#fff",
                fontSize: "10px",
                fontWeight: 700,
                borderRadius: "9999px",
                padding: "1px 7px",
                lineHeight: "15px",
              }}
            >
              {unreadNotificationsCount}
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

