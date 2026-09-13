import React from "react";
import { CheckCircle2, AlertTriangle, Info, X } from "lucide-react";
import { AppNotification } from "../../types";

interface ToastContainerProps {
  toasts: AppNotification[];
  onDismiss: (id: string) => void;
}

export const ToastContainer: React.FC<ToastContainerProps> = ({ toasts, onDismiss }) => {
  if (toasts.length === 0) return null;

  return (
    <div
      style={{
        position: "fixed",
        top: "24px",
        right: "24px",
        zIndex: 99999,
        display: "flex",
        flexDirection: "column",
        gap: "10px",
        maxWidth: "380px",
        width: "100%",
        pointerEvents: "none",
      }}
    >
      {toasts.map((toast) => {
        const isError = toast.type === "error";
        const isSuccess = toast.type === "success";

        const borderColor = isError
          ? "rgba(239, 68, 68, 0.4)"
          : isSuccess
          ? "rgba(16, 185, 129, 0.4)"
          : "rgba(139, 92, 246, 0.4)";

        const accentColor = isError ? "#ef4444" : isSuccess ? "#10b981" : "#8b5cf6";

        return (
          <div
            key={toast.id}
            style={{
              pointerEvents: "auto",
              backgroundColor: "rgba(22, 24, 34, 0.95)",
              backdropFilter: "blur(12px)",
              border: `1px solid ${borderColor}`,
              borderLeft: `4px solid ${accentColor}`,
              borderRadius: "10px",
              padding: "14px 16px",
              display: "flex",
              alignItems: "flex-start",
              gap: "12px",
              boxShadow: "0 10px 30px rgba(0, 0, 0, 0.6)",
              animation: "toastSlideIn 0.25s cubic-bezier(0.16, 1, 0.3, 1)",
              transition: "all 0.2s ease",
            }}
          >
            <div style={{ flexShrink: 0, marginTop: "2px" }}>
              {isError ? (
                <AlertTriangle size={18} color="#ef4444" />
              ) : isSuccess ? (
                <CheckCircle2 size={18} color="#10b981" />
              ) : (
                <Info size={18} color="#8b5cf6" />
              )}
            </div>

            <div style={{ flex: 1, minWidth: 0 }}>
              <div
                style={{
                  fontSize: "0.88rem",
                  fontWeight: 700,
                  color: "#fff",
                  marginBottom: "2px",
                }}
              >
                {toast.title}
              </div>
              <div
                style={{
                  fontSize: "0.8rem",
                  color: "var(--text-muted)",
                  lineHeight: 1.35,
                  wordBreak: "break-word",
                }}
              >
                {toast.message}
              </div>
            </div>

            <button
              type="button"
              onClick={() => onDismiss(toast.id)}
              style={{
                background: "none",
                border: "none",
                color: "var(--text-dim)",
                cursor: "pointer",
                padding: "2px",
                display: "flex",
                alignItems: "center",
                flexShrink: 0,
                marginTop: "2px",
                transition: "color 0.15s ease",
              }}
              onMouseEnter={(e) => (e.currentTarget.style.color = "#fff")}
              onMouseLeave={(e) => (e.currentTarget.style.color = "var(--text-dim)")}
              title="Dismiss notification"
            >
              <X size={15} />
            </button>
          </div>
        );
      })}
    </div>
  );
};
