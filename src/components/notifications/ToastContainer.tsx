import React from "react";
import { CheckCircle2, AlertTriangle, Info, X } from "lucide-react";
import { AppNotification } from "../../types";

interface ToastContainerProps {
  toasts: AppNotification[];
  onDismiss: (id: string) => void;
}

interface ToastItemProps {
  toast: AppNotification;
  onDismiss: (id: string) => void;
}

const ToastItem: React.FC<ToastItemProps> = ({ toast, onDismiss }) => {
  React.useEffect(() => {
    const timer = setTimeout(() => {
      onDismiss(toast.id);
    }, 3000);
    return () => clearTimeout(timer);
  }, [toast.id, onDismiss]);

  const isError = toast.type === "error";
  const isSuccess = toast.type === "success";

  const borderColor = isError
    ? "rgba(239, 68, 68, 0.4)"
    : isSuccess
    ? "rgba(139, 124, 246, 0.4)"
    : "rgba(243, 112, 30, 0.4)";

  const accentColor = isError ? "#ef4444" : isSuccess ? "var(--accent-secondary)" : "var(--accent)";

  return (
    <div
      style={{
        pointerEvents: "auto",
        backgroundColor: "rgba(22, 24, 34, 0.98)",
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
        position: "relative",
        overflow: "hidden",
      }}
    >
      <div style={{ flexShrink: 0, marginTop: "2px" }}>
        {isError ? (
          <AlertTriangle size={18} color="#ef4444" />
        ) : isSuccess ? (
          <CheckCircle2 size={18} color="var(--accent-secondary)" />
        ) : (
          <Info size={18} color="var(--accent)" />
        )}
      </div>

      <div style={{ flex: 1, minWidth: 0 }}>
        <div
          style={{
            fontSize: "0.88rem",
            fontWeight: 700,
            color: "#e8d8c9",
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
        onMouseEnter={(e) => (e.currentTarget.style.color = "#e8d8c9")}
        onMouseLeave={(e) => (e.currentTarget.style.color = "var(--text-dim)")}
        title="Dismiss notification"
      >
        <X size={15} />
      </button>

      {/* 3-Second visual expiration progress bar */}
      <div
        style={{
          position: "absolute",
          bottom: 0,
          left: 0,
          height: "2px",
          backgroundColor: accentColor,
          opacity: 0.6,
          width: "100%",
          animation: "toastProgressBar 3s linear forwards",
        }}
      />
    </div>
  );
};

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
      {toasts.map((toast) => (
        <ToastItem key={toast.id} toast={toast} onDismiss={onDismiss} />
      ))}
    </div>
  );
};
