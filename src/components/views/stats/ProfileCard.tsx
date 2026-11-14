import { AlertCircle, Camera, Loader2, Pencil, Trash2, User } from "lucide-react";
import type { StatsViewContext } from "./context";

export function ProfileCard({ c }: { c: StatsViewContext }) {
  return (
    <div
      style={{
        background: "linear-gradient(135deg, rgba(30, 27, 75, 0.45) 0%, rgba(15, 23, 42, 0.65) 100%)",
        border: "1px solid rgba(255, 255, 255, 0.08)",
        borderRadius: "20px",
        padding: "24px 28px",
        marginBottom: "36px",
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        flexWrap: "wrap",
        gap: "20px",
        boxShadow: "0 8px 32px -8px rgba(0, 0, 0, 0.5)",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: "20px", flexWrap: "wrap" }}>
        {/* Profile Photo / Avatar with Bottom-Left Pencil Edit Trigger */}
        <div style={{ position: "relative", width: "72px", height: "72px" }} ref={c.avatarContainerRef}>
          <div
            onClick={() => {
              if (!c.currentUser) {
                c.onOpenAuthModal?.();
                return;
              }
              if (c.currentUser.avatar_data_url) {
                c.setShowAvatarMenu((prev) => !prev);
              } else {
                c.handlePickAvatar();
              }
            }}
            title={c.currentUser ? (c.currentUser.avatar_data_url ? "Click to manage photo" : "Click to upload photo") : undefined}
            style={{
              width: "72px",
              height: "72px",
              borderRadius: "50%",
              background: c.currentUser?.avatar_data_url
                ? "transparent"
                : "linear-gradient(135deg, #f3701e 0%, #4b607f 100%)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              fontSize: "28px",
              fontWeight: 800,
              color: "#e8d8c9",
              border: "3px solid rgba(232, 216, 201, 0.5)",
              boxShadow: "0 0 20px rgba(232, 216, 201, 0.25)",
              overflow: "hidden",
              cursor: c.currentUser ? "pointer" : "default",
              position: "relative",
              flexShrink: 0,
            }}
          >
            {c.currentUser?.avatar_data_url ? (
              <img
                src={c.currentUser.avatar_data_url}
                alt={c.currentUser.username}
                style={{
                  width: "100%",
                  height: "100%",
                  objectFit: "cover",
                }}
              />
            ) : c.currentUser ? (
              <span>{c.currentUser.username[0].toUpperCase()}</span>
            ) : (
              <User size={36} color="rgba(255, 255, 255, 0.5)" />
            )}

            {/* Uploading overlay */}
            {c.isUploadingAvatar && (
              <div
                style={{
                  position: "absolute",
                  inset: 0,
                  background: "rgba(0, 0, 0, 0.75)",
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                }}
              >
                <Loader2 size={24} className="animate-spin" color="var(--accent-light)" />
              </div>
            )}
          </div>

          {/* Pencil Icon Edit Button at Bottom-Left of Profile Circle */}
          {c.currentUser && (
            <button
              onClick={(e) => {
                e.stopPropagation();
                if (c.currentUser?.avatar_data_url) {
                  c.setShowAvatarMenu((prev) => !prev);
                } else {
                  c.handlePickAvatar();
                }
              }}
              disabled={c.isUploadingAvatar}
              title={c.currentUser.avatar_data_url ? "Edit or remove profile photo" : "Upload profile photo"}
              style={{
                position: "absolute",
                bottom: "-2px",
                left: "-2px",
                width: "28px",
                height: "28px",
                borderRadius: "50%",
                background: "var(--accent)",
                border: "2.5px solid var(--bg-main)",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                cursor: c.isUploadingAvatar ? "not-allowed" : "pointer",
                color: "#e8d8c9",
                boxShadow: "0 2px 8px rgba(0, 0, 0, 0.5)",
                transition: "transform 0.15s ease, opacity 0.15s ease",
                padding: 0,
                zIndex: 10,
              }}
              onMouseEnter={(e) => {
                if (!c.isUploadingAvatar) e.currentTarget.style.background = "#f3701e";
              }}
              onMouseLeave={(e) => {
                if (!c.isUploadingAvatar) e.currentTarget.style.background = "var(--accent)";
              }}
            >
              {c.isUploadingAvatar ? (
                <Loader2 size={13} className="animate-spin" />
              ) : (
                <Pencil size={13} strokeWidth={2.5} />
              )}
            </button>
          )}

          {/* Dropdown menu when avatar exists */}
          {c.showAvatarMenu && c.currentUser?.avatar_data_url && (
            <div
              style={{
                position: "absolute",
                top: "100%",
                left: 0,
                marginTop: "8px",
                background: "var(--bg-card)",
                border: "1px solid rgba(232, 216, 201, 0.35)",
                borderRadius: "12px",
                padding: "6px",
                zIndex: 50,
                minWidth: "160px",
                boxShadow: "0 10px 25px -5px rgba(0, 0, 0, 0.6)",
                display: "flex",
                flexDirection: "column",
                gap: "4px",
              }}
            >
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  c.setShowAvatarMenu(false);
                  c.handlePickAvatar();
                }}
                disabled={c.isUploadingAvatar}
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: "8px",
                  background: "transparent",
                  border: "none",
                  color: "#e8d8c9",
                  padding: "8px 10px",
                  borderRadius: "8px",
                  fontSize: "12px",
                  fontWeight: 600,
                  cursor: "pointer",
                  textAlign: "left",
                  transition: "background 0.15s ease",
                }}
                onMouseEnter={(e) => (e.currentTarget.style.background = "rgba(232, 216, 201, 0.2)")}
                onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
              >
                <Camera size={14} color="var(--accent-light)" />
                <span>Change photo</span>
              </button>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  c.setShowAvatarMenu(false);
                  c.handleRemoveAvatar();
                }}
                disabled={c.isUploadingAvatar}
                style={{
                  display: "flex",
                  alignItems: "center",
                  gap: "8px",
                  background: "transparent",
                  border: "none",
                  color: "#f87171",
                  padding: "8px 10px",
                  borderRadius: "8px",
                  fontSize: "12px",
                  fontWeight: 600,
                  cursor: "pointer",
                  textAlign: "left",
                  transition: "background 0.15s ease",
                }}
                onMouseEnter={(e) => (e.currentTarget.style.background = "rgba(239, 68, 68, 0.15)")}
                onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
              >
                <Trash2 size={14} color="#f87171" />
                <span>Remove photo</span>
              </button>
            </div>
          )}
        </div>

        {/* User Details */}
        <div>
          <div style={{ display: "flex", alignItems: "center", gap: "12px", marginBottom: "6px" }}>
            <h2
              style={{
                fontSize: "26px",
                fontWeight: 800,
                margin: 0,
                color: "#e8d8c9",
                letterSpacing: "-0.5px",
              }}
            >
              {c.currentUser ? c.currentUser.username : "Guest Listener"}
            </h2>
          </div>

          <div
            style={{
              fontSize: "13px",
              color: "rgba(255, 255, 255, 0.65)",
              display: "flex",
              alignItems: "center",
              gap: "8px",
              flexWrap: "wrap",
            }}
          >
            {c.currentUser ? (
              <span>
                Member since{" "}
                <strong style={{ color: "#e8d8c9" }}>
                  {c.formatDate(c.currentUser.created_at)}
                </strong>
              </span>
            ) : (
              <span>
                Listening history saved locally •{" "}
                <button
                  onClick={c.onOpenAuthModal}
                  style={{
                    background: "none",
                    border: "none",
                    color: "var(--accent-light)",
                    cursor: "pointer",
                    textDecoration: "underline",
                    padding: 0,
                    fontSize: "13px",
                    fontWeight: 600,
                  }}
                >
                  Sign in to customize profile photo
                </button>
              </span>
            )}
          </div>

          {c.avatarError && (
            <div
              style={{
                marginTop: "8px",
                color: "#f87171",
                fontSize: "12px",
                display: "flex",
                alignItems: "center",
                gap: "6px",
              }}
            >
              <AlertCircle size={14} />
              <span>{c.avatarError}</span>
            </div>
          )}
        </div>
      </div>

      {!c.currentUser && (
        <button
          onClick={c.onOpenAuthModal}
          style={{
            background: "linear-gradient(135deg, #f3701e, #4b607f)",
            border: "none",
            color: "#e8d8c9",
            padding: "8px 16px",
            borderRadius: "10px",
            fontSize: "13px",
            fontWeight: 600,
            cursor: "pointer",
          }}
        >
          Sign In / Register
        </button>
      )}
    </div>
  );
}
