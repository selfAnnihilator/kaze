import React from "react";
import {
  Music,
  Disc,
  ListMusic,
  Compass,
  Bell,
  Settings,
  Sparkles,
  BarChart2,
  User,
  LogOut,
} from "lucide-react";
import { UserProfile } from "../types";

export type ViewType =
  | "library"
  | "albums"
  | "playlists"
  | "smart_mixes"
  | "discovery"
  | "notifications"
  | "stats"
  | "settings";

interface SidebarProps {
  currentView: ViewType;
  onSelectView: (view: ViewType) => void;
  unreadNotificationsCount?: number;
  currentUser?: UserProfile | null;
  onOpenAuthModal?: () => void;
  onLogout?: () => void;
}

export const Sidebar: React.FC<SidebarProps> = ({
  currentView,
  onSelectView,
  unreadNotificationsCount = 0,
  currentUser = null,
  onOpenAuthModal,
  onLogout,
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
        <button
          className={`nav-button ${currentView === "stats" ? "active" : ""}`}
          onClick={() => onSelectView("stats")}
        >
          <BarChart2 size={18} />
          <span>Stats</span>
        </button>
      </div>

      <div style={{ marginTop: "auto", display: "flex", flexDirection: "column", gap: "6px" }}>
        <button
          className={`nav-button ${currentView === "settings" ? "active" : ""}`}
          onClick={() => onSelectView("settings")}
        >
          <Settings size={18} />
          <span>Settings</span>
        </button>

        {currentUser ? (
          <div
            style={{
              padding: "8px 12px",
              background: "rgba(255, 255, 255, 0.04)",
              borderRadius: "10px",
              border: "1px solid rgba(255, 255, 255, 0.08)",
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              marginTop: "2px",
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: "8px", minWidth: 0 }}>
              <div
                style={{
                  width: "28px",
                  height: "28px",
                  borderRadius: "50%",
                  background: "linear-gradient(135deg, #8b5cf6, #ec4899)",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  fontSize: "12px",
                  fontWeight: 700,
                  color: "#fff",
                  flexShrink: 0,
                }}
              >
                {currentUser.username[0].toUpperCase()}
              </div>
              <div style={{ minWidth: 0, overflow: "hidden" }}>
                <div
                  style={{
                    fontSize: "13px",
                    fontWeight: 600,
                    color: "#fff",
                    textOverflow: "ellipsis",
                    overflow: "hidden",
                    whiteSpace: "nowrap",
                  }}
                >
                  {currentUser.username}
                </div>
                <div style={{ fontSize: "10px", color: "rgba(255, 255, 255, 0.5)" }}>
                  Member
                </div>
              </div>
            </div>
            {onLogout && (
              <button
                onClick={onLogout}
                title="Log Out"
                style={{
                  background: "transparent",
                  border: "none",
                  color: "rgba(255, 255, 255, 0.5)",
                  cursor: "pointer",
                  padding: "4px",
                  display: "flex",
                  alignItems: "center",
                  borderRadius: "4px",
                }}
                onMouseEnter={(e) => (e.currentTarget.style.color = "#ef4444")}
                onMouseLeave={(e) => (e.currentTarget.style.color = "rgba(255, 255, 255, 0.5)")}
              >
                <LogOut size={16} />
              </button>
            )}
          </div>
        ) : (
          onOpenAuthModal && (
            <button
              className="nav-button"
              onClick={onOpenAuthModal}
              style={{
                background: "rgba(139, 92, 246, 0.12)",
                color: "#c084fc",
                border: "1px solid rgba(139, 92, 246, 0.25)",
                marginTop: "2px",
              }}
            >
              <User size={18} />
              <span>Sign In / Sign Up</span>
            </button>
          )
        )}
      </div>
    </aside>
  );
};

