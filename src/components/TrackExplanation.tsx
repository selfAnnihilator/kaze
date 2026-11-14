import React from "react";
import type { FlowExplanation } from "../types";
import { presentFlowExplanation } from "../flowExplanation";

export const TrackExplanation = React.memo(({ evidence }: { evidence?: FlowExplanation | null }) => {
  const copy = presentFlowExplanation(evidence);
  return <div>
    <strong>{copy.title}</strong>
    <p style={{ margin: "var(--space-6) 0 0" }}>{copy.body}</p>
    {copy.secondary.length > 0 && <>
      <div style={{ marginTop: "var(--space-12)", color: "var(--text-muted)", fontSize: "var(--fs-13)" }}>Also considered</div>
      <ul style={{ margin: "var(--space-4) 0 0", paddingLeft: "var(--space-18)" }}>{copy.secondary.map(text => <li key={text}>{text}</li>)}</ul>
    </>}
  </div>;
});
