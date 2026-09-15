import React, { useState } from "react";
import {
  Music,
  Disc,
  ListMusic,
  Compass,
  Bell,
  Settings,
  User,
  ChevronDown,
  Heart,
} from "lucide-react";
import { Playlist } from "../types";

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
  playlists?: Playlist[];
  onSelectPlaylist?: (playlist: Playlist) => void;
  activePlaylistId?: string | null;
}

export const Sidebar: React.FC<SidebarProps> = ({
  currentView,
  onSelectView,
  unreadNotificationsCount = 0,
  playlists = [],
  onSelectPlaylist,
  activePlaylistId,
}) => {
  const [collapsed, setCollapsed] = useState(false);
  const [playlistsOpen, setPlaylistsOpen] = useState(
    currentView === "playlists"
  );

  const userPlaylists = playlists.filter((p) => !p.is_smart_mix);

  const handlePlaylistsClick = () => {
    if (collapsed) {
      // Stay collapsed — just navigate, no dropdown
      onSelectView("playlists");
    } else {
      onSelectView("playlists");
      setPlaylistsOpen(true);
    }
  };

  const handleChevronClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    setPlaylistsOpen((prev) => !prev);
  };

  const navBtn = (view: ViewType, icon: React.ReactNode, label: string, extra?: React.ReactNode) => (
    <button
      className={`nav-button ${currentView === view ? "active" : ""}`}
      onClick={() => onSelectView(view)}
      title={collapsed ? label : undefined}
    >
      {icon}
      {!collapsed && <span>{label}</span>}
      {!collapsed && extra}
    </button>
  );

  return (
    <aside className={`sidebar${collapsed ? " sidebar--collapsed" : ""}`}>
      {/* Logo row — click icon to toggle collapse */}
      <div className="logo-container">
        <img
          className="kage-logo"
          src="/kaze-icon.png"
          alt="Kaze"
          onClick={() => setCollapsed((prev) => !prev)}
          title={collapsed ? "Expand sidebar" : "Collapse sidebar"}
          style={{ cursor: "pointer", flexShrink: 0 }}
        />
        {!collapsed && <span className="logo-title">KAZE</span>}
      </div>

      {/* Discover */}
      <div className="nav-section" style={{ marginTop: "6px" }}>
        {navBtn("discovery", <Compass size={18} />, "Discover Music")}
      </div>

      {/* Local Library */}
      <div className="nav-section">
        {!collapsed && <span className="nav-section-title">Local Library</span>}
        {navBtn("library", <Music size={18} />, "Downloaded Tracks")}
        {navBtn("albums", <Disc size={18} />, "Albums")}
      </div>

      {/* Playlists & Mixes */}
      <div className="nav-section">
        {!collapsed && <span className="nav-section-title">Playlists &amp; Mixes</span>}

        {collapsed ? (
          /* Collapsed: plain button like all other nav buttons — no flex group, no min-width: 0 */
          <button
            className={`nav-button ${currentView === "playlists" ? "active" : ""}`}
            onClick={handlePlaylistsClick}
            title="Playlists"
            style={{ justifyContent: "center" }}
          >
            <ListMusic size={20} />
          </button>
        ) : (
          /* Expanded: button + chevron in a row */
          <div className="nav-button-group">
            <button
              className={`nav-button nav-button--expand ${currentView === "playlists" ? "active" : ""}`}
              onClick={handlePlaylistsClick}
            >
              <ListMusic size={20} />
              <span>Playlists</span>
            </button>

            {/* Chevron only shown when expanded */}
            {userPlaylists.length > 0 && (
              <button
                className={`playlist-chevron ${playlistsOpen ? "open" : ""}`}
                onClick={handleChevronClick}
                aria-label={playlistsOpen ? "Collapse playlists" : "Expand playlists"}
                title={playlistsOpen ? "Collapse playlists" : "Expand playlists"}
              >
                <ChevronDown size={14} />
              </button>
            )}
          </div>
        )}

        {/* Dropdown — only when expanded AND open */}
        {!collapsed && userPlaylists.length > 0 && playlistsOpen && (
          <div className="sidebar-playlist-dropdown">
            {userPlaylists.map((playlist) => (
              <button
                key={playlist.id}
                className={`sidebar-playlist${activePlaylistId === playlist.id ? " active" : ""}`}
                onClick={() => onSelectPlaylist?.(playlist)}
              >
                {playlist.name === "Liked Songs" ? (
                  <span className="sidebar-liked-thumbnail" aria-hidden="true"><Heart size={20} fill="#ec4899" /></span>
                ) : (
                  <img src={playlist.cover_art_url || "/kaze-playlist-default.svg"} alt="" />
                )}
                <span>
                  {playlist.name}
                  <small>{playlist.track_count ?? 0} tracks</small>
                </span>
              </button>
            ))}
          </div>
        )}
      </div>

      {/* Activity */}
      <div className="nav-section">
        {!collapsed && <span className="nav-section-title">Activity</span>}
        {navBtn(
          "notifications",
          <Bell size={18} />,
          "Notifications",
          unreadNotificationsCount > 0 && !collapsed ? (
            <span
              style={{
                marginLeft: "auto",
                backgroundColor: "#ef4444",
                color: "#e8d8c9",
                fontSize: "10px",
                fontWeight: 700,
                borderRadius: "9999px",
                padding: "1px 7px",
                lineHeight: "15px",
              }}
            >
              {unreadNotificationsCount}
            </span>
          ) : undefined
        )}
        {navBtn("stats", <User size={18} />, "Profile")}
      </div>

      {/* Settings pinned to bottom */}
      <div style={{ marginTop: "auto", display: "flex", flexDirection: "column", gap: "6px", paddingBottom: "16px" }}>
        {navBtn("settings", <Settings size={18} />, "Settings")}
      </div>
    </aside>
  );
};
