import { useEffect, useRef } from "react";
import { AlertTriangle } from "lucide-react";
import { answerConfirm, useConfirmRequest } from "../../services/confirmDialog";

/** The one Yes/No popup. Mounted once; opened with `confirmAction()`. */
export function ConfirmModal() {
  const request = useConfirmRequest();
  const confirmRef = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (!request) return;
    confirmRef.current?.focus();
    const onKey = (event: KeyboardEvent) => { if (event.key === "Escape") answerConfirm(false); };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [request]);
  if (!request) return null;
  return (
    <div className="modal-overlay" onClick={() => answerConfirm(false)}>
      <div
        className="modal-content"
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="confirm-modal-title"
        aria-describedby="confirm-modal-message"
        style={{
          maxWidth: "440px",
          width: "92%",
          padding: "24px",
          borderRadius: "14px",
          backgroundColor: "var(--bg-card, #1a1714)",
          border: "1px solid var(--border, rgba(232, 216, 201, 0.15))",
          boxShadow: "0 24px 50px rgba(0, 0, 0, 0.65)",
        }}
        onClick={event => event.stopPropagation()}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "10px", marginBottom: "12px" }}>
          <AlertTriangle size={20} color="var(--accent-secondary, #f3701e)" />
          <h3 id="confirm-modal-title" style={{ fontSize: "1.15rem", fontWeight: 700, margin: 0, color: "#e8d8c9" }}>
            {request.title}
          </h3>
        </div>
        <p id="confirm-modal-message" style={{ fontSize: "0.88rem", color: "var(--text-dim, #8c8273)", margin: "0 0 20px 0", lineHeight: 1.5 }}>
          {request.message}
        </p>
        <div style={{ display: "flex", gap: "10px", justifyContent: "flex-end" }}>
          <button type="button" className="btn btn-secondary" onClick={() => answerConfirm(false)}>
            {request.cancelLabel || "No"}
          </button>
          <button ref={confirmRef} type="button" className="btn btn-primary" onClick={() => answerConfirm(true)}>
            {request.confirmLabel || "Yes"}
          </button>
        </div>
      </div>
    </div>
  );
}
