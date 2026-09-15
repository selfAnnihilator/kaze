import React, { useState } from "react";
import {
  Bell,
  CheckCircle2,
  AlertTriangle,
  Info,
  Trash2,
  Clock,
  DownloadCloud,
  Check,
} from "lucide-react";
import { AppNotification } from "../../types";

interface NotificationsViewProps {
  notifications: AppNotification[];
  onClearAll: () => void;
  onRemoveNotification: (id: string) => void;
  onMarkAllRead?: () => void;
  onMarkAsRead?: (id: string) => void;
}

export const NotificationsView: React.FC<NotificationsViewProps> = ({
  notifications,
  onClearAll,
  onRemoveNotification,
  onMarkAllRead,
  onMarkAsRead,
}) => {
  const [filter, setFilter] = useState<"all" | "downloads" | "errors">("all");

  const filtered = notifications.filter((n) => {
    if (filter === "errors") return n.type === "error";
    if (filter === "downloads")
      return (
        n.title.toLowerCase().includes("download") ||
        n.message.toLowerCase().includes("download")
      );
    return true;
  });

  const formatTimeAgo = (timestamp: number) => {
    const diff = Math.floor((Date.now() - timestamp) / 1000);
    if (diff < 60) return "Just now";
    if (diff < 3600) return `${Math.floor(diff / 60)} mins ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)} hours ago`;
    return `${Math.floor(diff / 86400)} days ago`;
  };

  return (
    <div
      style={{
        display: "flex",
        flexDirection: "column",
        gap: "24px",
        paddingBottom: "40px",
      }}
    >
      {/* Header */}
      <div
        style={{
          display: "flex",
          justifyContent: "space-between",
          alignItems: "center",
          flexWrap: "wrap",
          gap: "12px",
        }}
      >
        <div>
          <h1
            style={{
              fontSize: "1.75rem",
              fontWeight: 800,
              color: "#e8d8c9",
              display: "flex",
              alignItems: "center",
              gap: "10px",
            }}
          >
            <Bell size={26} color="var(--accent-light)" />
            <span>Notifications</span>
          </h1>
          <p style={{ fontSize: "0.88rem", color: "var(--text-muted)", marginTop: "4px" }}>
            History of download events, library updates, and app alerts
          </p>
        </div>

        {notifications.length > 0 && (
          <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
            {onMarkAllRead && (
              <button
                type="button"
                className="btn btn-secondary"
                onClick={onMarkAllRead}
                style={{ fontSize: "0.82rem", display: "flex", alignItems: "center", gap: "6px" }}
              >
                <Check size={14} />
                <span>Mark All as Read</span>
              </button>
            )}
            <button
              type="button"
              className="btn btn-secondary"
              onClick={onClearAll}
              style={{
                fontSize: "0.82rem",
                display: "flex",
                alignItems: "center",
                gap: "6px",
                color: "#ef4444",
                borderColor: "rgba(239, 68, 68, 0.3)",
              }}
              title="Clear all notifications"
            >
              <Trash2 size={14} />
              <span>Clear All</span>
            </button>
          </div>
        )}
      </div>

      {/* Filter Tabs */}
      <div style={{ display: "flex", alignItems: "center", gap: "8px" }}>
        <button
          type="button"
          onClick={() => setFilter("all")}
          className={`btn ${filter === "all" ? "btn-primary" : "btn-secondary"}`}
          style={{ padding: "6px 14px", fontSize: "0.82rem" }}
        >
          All ({notifications.length})
        </button>
        <button
          type="button"
          onClick={() => setFilter("downloads")}
          className={`btn ${filter === "downloads" ? "btn-primary" : "btn-secondary"}`}
          style={{ padding: "6px 14px", fontSize: "0.82rem" }}
        >
          Downloads (
          {
            notifications.filter(
              (n) =>
                n.title.toLowerCase().includes("download") ||
                n.message.toLowerCase().includes("download")
            ).length
          }
          )
        </button>
        <button
          type="button"
          onClick={() => setFilter("errors")}
          className={`btn ${filter === "errors" ? "btn-primary" : "btn-secondary"}`}
          style={{ padding: "6px 14px", fontSize: "0.82rem" }}
        >
          Errors ({notifications.filter((n) => n.type === "error").length})
        </button>
      </div>

      {/* Notifications List */}
      {filtered.length === 0 ? (
        <div
          className="content-card"
          style={{
            textAlign: "center",
            padding: "60px 20px",
            borderStyle: "dashed",
            color: "var(--text-dim)",
          }}
        >
          <Bell size={42} color="var(--accent-light)" style={{ marginBottom: "12px", opacity: 0.6 }} />
          <h3 style={{ fontSize: "1.05rem", fontWeight: 700, color: "var(--text-main)", marginBottom: "4px" }}>
            No notifications
          </h3>
          <p style={{ fontSize: "0.85rem", color: "var(--text-muted)", maxWidth: "400px", margin: "0 auto" }}>
            You have no notifications in this category. Download events and errors will appear here automatically.
          </p>
        </div>
      ) : (
        <div style={{ display: "flex", flexDirection: "column", gap: "10px" }}>
          {filtered.map((item) => {
            const isError = item.type === "error";
            const isSuccess = item.type === "success";

            const stateBorder = isError
              ? "1px solid rgba(239, 68, 68, 0.45)"
              : isSuccess
              ? "1px solid rgba(139, 124, 246, 0.45)"
              : "1px solid rgba(243, 112, 30, 0.45)";

            return (
              <div
                key={item.id}
                className="content-card"
                style={{
                  margin: 0,
                  padding: "16px 18px",
                  display: "flex",
                  alignItems: "flex-start",
                  justifyContent: "space-between",
                  gap: "14px",
                  border: stateBorder,
                  backgroundColor: item.read ? "var(--bg-card)" : "rgba(243, 112, 30, 0.05)",
                  cursor: "pointer",
                }}
                onClick={() => onMarkAsRead && onMarkAsRead(item.id)}
              >
                <div style={{ display: "flex", alignItems: "flex-start", gap: "12px", flex: 1 }}>
                  <div style={{ marginTop: "2px", flexShrink: 0 }}>
                    {isError ? (
                      <AlertTriangle size={20} color="#ef4444" />
                    ) : isSuccess ? (
                      <CheckCircle2 size={20} color="var(--accent-secondary)" />
                    ) : item.title.toLowerCase().includes("download") ? (
                      <DownloadCloud size={20} color="var(--accent-light)" />
                    ) : (
                      <Info size={20} color="var(--accent-light)" />
                    )}
                  </div>

                  <div style={{ flex: 1 }}>
                    <div
                      style={{
                        fontSize: "0.95rem",
                        fontWeight: 700,
                        color: "#e8d8c9",
                        display: "flex",
                        alignItems: "center",
                        gap: "8px",
                        marginBottom: "4px",
                      }}
                    >
                      <span>{item.title}</span>
                      {!item.read && (
                        <span
                          style={{
                            width: "7px",
                            height: "7px",
                            borderRadius: "50%",
                            backgroundColor: "var(--accent-light)",
                            display: "inline-block",
                          }}
                        />
                      )}
                    </div>
                    <p style={{ fontSize: "0.85rem", color: "var(--text-muted)", lineHeight: 1.4 }}>
                      {item.message}
                    </p>
                    <div
                      style={{
                        display: "flex",
                        alignItems: "center",
                        gap: "6px",
                        fontSize: "0.75rem",
                        color: "var(--text-dim)",
                        marginTop: "8px",
                      }}
                    >
                      <Clock size={12} />
                      <span>{formatTimeAgo(item.timestamp)}</span>
                    </div>
                  </div>
                </div>

                <button
                  type="button"
                  onClick={() => onRemoveNotification(item.id)}
                  style={{
                    background: "none",
                    border: "none",
                    color: "var(--text-dim)",
                    cursor: "pointer",
                    padding: "6px",
                    display: "flex",
                    alignItems: "center",
                    borderRadius: "6px",
                    transition: "all 0.15s ease",
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.color = "#ef4444";
                    e.currentTarget.style.backgroundColor = "rgba(239, 68, 68, 0.1)";
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.color = "var(--text-dim)";
                    e.currentTarget.style.backgroundColor = "transparent";
                  }}
                  title="Remove notification"
                >
                  <Trash2 size={15} />
                </button>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};
