import { Sparkles, X } from "lucide-react";
import type { FlowExplanation } from "../../types";
import { TrackExplanation } from "../TrackExplanation";

export interface ExplainedTrack {
  title: string;
  artist: string;
  explanation: FlowExplanation;
}

/** "Why this track?" for the current Flow song. */
export function WhyTrackModal({ explained, onClose }: { explained: ExplainedTrack; onClose: () => void }) {
  return (
    <div
      className="modal-overlay"
      onClick={() => onClose()}
      style={{
        position: "fixed",
        inset: 0,
        backgroundColor: "rgba(0, 0, 0, 0.72)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        zIndex: 1000,
      }}
    >
      <div
        className="modal-content"
        onClick={(e) => e.stopPropagation()}
        style={{
          maxWidth: "420px",
          width: "90%",
          padding: "24px",
          borderRadius: "14px",
          backgroundColor: "var(--bg-card, #1a1714)",
          border: "1px solid var(--border, rgba(232, 216, 201, 0.15))",
          boxShadow: "0 20px 48px rgba(0, 0, 0, 0.65)",
        }}
      >
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            marginBottom: "16px",
          }}
        >
          <div
            style={{ display: "flex", alignItems: "center", gap: "8px" }}
          >
            <Sparkles
              size={18}
              color="var(--accent-secondary, #f3701e)"
            />
            <h3
              style={{
                fontSize: "1.1rem",
                fontWeight: 700,
                margin: 0,
                color: "var(--text-main, #e8d8c9)",
              }}
            >
              Why This Track?
            </h3>
          </div>
          <button
            onClick={() => onClose()}
            style={{
              background: "none",
              border: "none",
              color: "var(--text-muted)",
              cursor: "pointer",
              padding: "4px",
            }}
          >
            <X size={18} />
          </button>
        </div>

        <div style={{ marginBottom: "12px" }}>
          <div
            style={{
              fontSize: "0.95rem",
              fontWeight: 600,
              color: "var(--text-main, #e8d8c9)",
            }}
          >
            {explained.title}
          </div>
          <div
            style={{
              fontSize: "0.82rem",
              color: "var(--text-dim, #948779)",
            }}
          >
            {explained.artist}
          </div>
        </div>

        <div
          style={{
            padding: "12px 14px",
            borderRadius: "8px",
            backgroundColor: "rgba(243, 112, 30, 0.08)",
            border: "1px solid rgba(243, 112, 30, 0.2)",
            fontSize: "0.88rem",
            color: "var(--accent-light, #ff9b60)",
            lineHeight: 1.45,
          }}
        >
          <TrackExplanation evidence={explained.explanation} />
        </div>

        <div
          style={{
            marginTop: "18px",
            display: "flex",
            justifyContent: "flex-end",
          }}
        >
          <button
            className="btn btn-secondary"
            onClick={() => onClose()}
            style={{ fontSize: "0.85rem", padding: "6px 16px" }}
          >
            Got it
          </button>
        </div>
      </div>
    </div>
  );
}
