import React from "react";
import type { FlowExplanation } from "../types";
import { presentFlowExplanation } from "../flowExplanation";

export const TrackExplanation = React.memo(({ evidence }: { evidence?: FlowExplanation | null }) => {
  const copy = presentFlowExplanation(evidence);
  return <div>
    <strong>{copy.title}</strong>
    <p style={{ margin: "6px 0 0" }}>{copy.body}</p>
    {copy.secondary.length > 0 && <>
      <div style={{ marginTop: "12px", color: "var(--text-muted)", fontSize: "0.8rem" }}>Also considered</div>
      <ul style={{ margin: "4px 0 0", paddingLeft: "18px" }}>{copy.secondary.map(text => <li key={text}>{text}</li>)}</ul>
    </>}
  </div>;
});
