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
        backgroundColor: "var(--shade-70)",
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
          padding: "var(--space-24)",
          borderRadius: "14px",
          backgroundColor: "var(--bg-card, var(--bg-main))",
          border: "1px solid var(--border, var(--text-main-a15))",
          boxShadow: "0 20px 48px var(--shade-65)",
        }}
      >
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            marginBottom: "var(--space-16)",
          }}
        >
          <div
            style={{ display: "flex", alignItems: "center", gap: "var(--space-8)" }}
          >
            <Sparkles
              size={18}
              color="var(--accent-secondary, var(--accent))"
            />
            <h3
              style={{
                fontSize: "var(--fs-18)",
                fontWeight: "var(--weight-bold)",
                margin: 0,
                color: "var(--text-main, var(--text-main))",
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
              padding: "var(--space-4)",
            }}
          >
            <X size={18} />
          </button>
        </div>

        <div style={{ marginBottom: "var(--space-12)" }}>
          <div
            style={{
              fontSize: "var(--fs-15)",
              fontWeight: "var(--weight-semibold)",
              color: "var(--text-main, var(--text-main))",
            }}
          >
            {explained.title}
          </div>
          <div
            style={{
              fontSize: "var(--fs-13)",
              color: "var(--text-dim, var(--text-dim))",
            }}
          >
            {explained.artist}
          </div>
        </div>

        <div
          style={{
            padding: "var(--space-12) var(--space-14)",
            borderRadius: "8px",
            backgroundColor: "var(--accent-a8)",
            border: "1px solid var(--accent-a20)",
            fontSize: "var(--fs-14)",
            color: "var(--accent-light, var(--accent-light))",
            lineHeight: 1.45,
          }}
        >
          <TrackExplanation evidence={explained.explanation} />
        </div>

        <div
          style={{
            marginTop: "var(--space-18)",
            display: "flex",
            justifyContent: "flex-end",
          }}
        >
          <button
            className="btn btn-secondary"
            onClick={() => onClose()}
            style={{ fontSize: "var(--fs-14)", padding: "var(--space-6) var(--space-16)" }}
          >
            Got it
          </button>
        </div>
      </div>
    </div>
  );
}
