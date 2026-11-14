import { AlertCircle, Camera, Loader2, Pencil, Trash2, User } from "lucide-react";
import type { StatsViewContext } from "./context";

export function ProfileCard({ c }: { c: StatsViewContext }) {
  return (
    <div
      style={{
        background: "linear-gradient(135deg, var(--palette-indigo-800-a45) 0%, var(--palette-blue-900-a65) 100%)",
        border: "1px solid var(--overlay-8)",
        borderRadius: "20px",
        padding: "var(--space-24) var(--space-28)",
        marginBottom: "var(--space-36)",
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        flexWrap: "wrap",
        gap: "var(--space-20)",
        boxShadow: "0 8px 32px -8px var(--shade-50)",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: "var(--space-20)", flexWrap: "wrap" }}>
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
                : "linear-gradient(135deg, var(--accent) 0%, var(--accent-slate) 100%)",
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              fontSize: "var(--fs-28)",
              fontWeight: "var(--weight-extrabold)",
              color: "var(--text-main)",
              border: "3px solid var(--text-main-a50)",
              boxShadow: "0 0 20px var(--text-main-a25)",
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
              <User size={36} color="var(--overlay-50)" />
            )}

            {/* Uploading overlay */}
            {c.isUploadingAvatar && (
              <div
                style={{
                  position: "absolute",
                  inset: 0,
                  background: "var(--shade-75)",
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
                color: "var(--text-main)",
                boxShadow: "0 2px 8px var(--shade-50)",
                transition: "transform 0.15s ease, opacity 0.15s ease",
                padding: 0,
                zIndex: 10,
              }}
              onMouseEnter={(e) => {
                if (!c.isUploadingAvatar) e.currentTarget.style.background = "var(--accent)";
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
                marginTop: "var(--space-8)",
                background: "var(--bg-card)",
                border: "1px solid var(--text-main-a35)",
                borderRadius: "12px",
                padding: "var(--space-6)",
                zIndex: 50,
                minWidth: "160px",
                boxShadow: "0 10px 25px -5px var(--shade-60)",
                display: "flex",
                flexDirection: "column",
                gap: "var(--space-4)",
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
                  gap: "var(--space-8)",
                  background: "transparent",
                  border: "none",
                  color: "var(--text-main)",
                  padding: "var(--space-8) var(--space-10)",
                  borderRadius: "8px",
                  fontSize: "var(--fs-12)",
                  fontWeight: "var(--weight-semibold)",
                  cursor: "pointer",
                  textAlign: "left",
                  transition: "background 0.15s ease",
                }}
                onMouseEnter={(e) => (e.currentTarget.style.background = "var(--text-main-a20)")}
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
                  gap: "var(--space-8)",
                  background: "transparent",
                  border: "none",
                  color: "var(--palette-red-300)",
                  padding: "var(--space-8) var(--space-10)",
                  borderRadius: "8px",
                  fontSize: "var(--fs-12)",
                  fontWeight: "var(--weight-semibold)",
                  cursor: "pointer",
                  textAlign: "left",
                  transition: "background 0.15s ease",
                }}
                onMouseEnter={(e) => (e.currentTarget.style.background = "var(--danger-a15)")}
                onMouseLeave={(e) => (e.currentTarget.style.background = "transparent")}
              >
                <Trash2 size={14} color="var(--palette-red-300)" />
                <span>Remove photo</span>
              </button>
            </div>
          )}
        </div>

        {/* User Details */}
        <div>
          <div style={{ display: "flex", alignItems: "center", gap: "var(--space-12)", marginBottom: "var(--space-6)" }}>
            <h2
              style={{
                fontSize: "var(--fs-28)",
                fontWeight: "var(--weight-extrabold)",
                margin: 0,
                color: "var(--text-main)",
                letterSpacing: "-0.5px",
              }}
            >
              {c.currentUser ? c.currentUser.username : "Guest Listener"}
            </h2>
          </div>

          <div
            style={{
              fontSize: "var(--fs-13)",
              color: "var(--overlay-65)",
              display: "flex",
              alignItems: "center",
              gap: "var(--space-8)",
              flexWrap: "wrap",
            }}
          >
            {c.currentUser ? (
              <span>
                Member since{" "}
                <strong style={{ color: "var(--text-main)" }}>
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
                    fontSize: "var(--fs-13)",
                    fontWeight: "var(--weight-semibold)",
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
                marginTop: "var(--space-8)",
                color: "var(--palette-red-300)",
                fontSize: "var(--fs-12)",
                display: "flex",
                alignItems: "center",
                gap: "var(--space-6)",
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
            background: "linear-gradient(135deg, var(--accent), var(--accent-slate))",
            border: "none",
            color: "var(--text-main)",
            padding: "var(--space-8) var(--space-16)",
            borderRadius: "10px",
            fontSize: "var(--fs-13)",
            fontWeight: "var(--weight-semibold)",
            cursor: "pointer",
          }}
        >
          Sign In / Register
        </button>
      )}
    </div>
  );
}
