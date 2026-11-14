import React, { useState } from "react";
import { X, Sparkles } from "lucide-react";

export interface PromptModalProps {
  title: string;
  message: string;
  defaultValue?: string;
  onConfirm: (value: string) => void;
  onCancel: () => void;
}

export const PromptModal: React.FC<PromptModalProps> = ({
  title,
  message,
  defaultValue = "",
  onConfirm,
  onCancel,
}) => {
  const [val, setVal] = useState(defaultValue);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!val.trim()) return;
    onConfirm(val.trim());
  };

  return (
    <div className="modal-overlay" onClick={onCancel}>
      <div
        className="modal-content"
        style={{
          maxWidth: "460px",
          width: "92%",
          padding: "24px",
          borderRadius: "14px",
          backgroundColor: "var(--bg-card, #1a1714)",
          border: "1px solid var(--border, rgba(232, 216, 201, 0.15))",
          boxShadow: "0 24px 50px rgba(0, 0, 0, 0.65)",
        }}
        onClick={(e) => e.stopPropagation()}
      >
        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "16px",
          }}>
          <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
            <Sparkles size={20} color="var(--accent-secondary, #f3701e)" />
            <h3 style={{ fontSize: "1.15rem", fontWeight: 700, margin: 0, color: "#e8d8c9",
              }}>
              {title}
            </h3>
          </div>
          <button
            onClick={onCancel}
            style={{ background: "none", border: "none", color: "var(--text-muted, #a89f91)", cursor: "pointer", padding: "4px",
            }}
          >
            <X size={18} />
          </button>
        </div>

        <p style={{ fontSize: "0.88rem", color: "var(--text-dim, #8c8273)", margin: "0 0 16px 0", lineHeight: 1.5,
          }}>
          {message}
        </p>

        <form onSubmit={handleSubmit}>
          <input
            type="text"
            autoFocus
            className="input-field"
            value={val}
            onChange={(e) => setVal(e.target.value)}
            style={{ width: "100%", fontSize: "0.95rem", marginBottom: "20px" }}
          />
          <div style={{ display: "flex", gap: "10px", justifyContent: "flex-end" }}>
            <button type="button" className="btn btn-secondary" onClick={onCancel}>
              Cancel
            </button>
            <button type="submit" className="btn btn-primary" disabled={!val.trim()}>
              Save Name
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
