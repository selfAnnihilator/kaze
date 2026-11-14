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
    ? "var(--danger-a40)"
    : isSuccess
    ? "var(--accent-secondary-a40)"
    : "var(--accent-a40)";

  const accentColor = isError ? "var(--danger)" : isSuccess ? "var(--accent-secondary)" : "var(--accent)";

  return (
    <div
      style={{
        pointerEvents: "auto",
        backgroundColor: "var(--palette-slate-900-a95)",
        border: `1px solid ${borderColor}`,
        borderLeft: `4px solid ${accentColor}`,
        borderRadius: "var(--radius-10)",
        padding: "var(--space-14) var(--space-16)",
        display: "flex",
        alignItems: "flex-start",
        gap: "var(--space-12)",
        boxShadow: "0 10px 30px var(--shade-60)",
        animation: "toastSlideIn var(--duration-moderate) var(--ease-emphasized)",
        transition: "transform var(--duration-base) ease, opacity var(--duration-base) ease",
        position: "relative",
        overflow: "hidden",
      }}
    >
      <div style={{ flexShrink: 0, marginTop: "var(--space-2)" }}>
        {isError ? (
          <AlertTriangle size={18} color="var(--danger)" />
        ) : isSuccess ? (
          <CheckCircle2 size={18} color="var(--accent-secondary)" />
        ) : (
          <Info size={18} color="var(--accent)" />
        )}
      </div>

      <div style={{ flex: 1, minWidth: 0 }}>
        <div
          style={{
            fontSize: "var(--fs-14)",
            fontWeight: "var(--weight-bold)",
            color: "var(--text-main)",
            marginBottom: "var(--space-2)",
          }}
        >
          {toast.title}
        </div>
        <div
          style={{
            fontSize: "var(--fs-13)",
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
          padding: "var(--space-2)",
          display: "flex",
          alignItems: "center",
          flexShrink: 0,
          marginTop: "var(--space-2)",
          transition: "color var(--duration-fast) ease",
        }}
        onMouseEnter={(e) => (e.currentTarget.style.color = "var(--text-main)")}
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
          animation: "toastProgressBar var(--duration-loop-slow) linear forwards",
          transformOrigin: "left",
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
        zIndex: "var(--z-toast)",
        display: "flex",
        flexDirection: "column",
        gap: "var(--space-10)",
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
