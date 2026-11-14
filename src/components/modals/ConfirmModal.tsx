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
          padding: "var(--space-24)",
          borderRadius: "var(--radius-14)",
          backgroundColor: "var(--bg-card, var(--bg-main))",
          border: "1px solid var(--border, var(--text-main-a15))",
          boxShadow: "0 24px 50px var(--shade-65)",
        }}
        onClick={event => event.stopPropagation()}
      >
        <div style={{ display: "flex", alignItems: "center", gap: "var(--space-10)", marginBottom: "var(--space-12)" }}>
          <AlertTriangle size={20} color="var(--accent-secondary, var(--accent))" />
          <h3 id="confirm-modal-title" style={{ fontSize: "var(--fs-18)", fontWeight: "var(--weight-bold)", margin: 0, color: "var(--text-main)" }}>
            {request.title}
          </h3>
        </div>
        <p id="confirm-modal-message" style={{ fontSize: "var(--fs-14)", color: "var(--text-dim, var(--palette-sand-500))", margin: "0 0 var(--space-20) 0", lineHeight: 1.5 }}>
          {request.message}
        </p>
        <div style={{ display: "flex", gap: "var(--space-10)", justifyContent: "flex-end" }}>
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
