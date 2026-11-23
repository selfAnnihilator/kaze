/** Where an in-app update is: idle, downloading (with percent) or handed to Android to install. */
export type UpdatePhase = { state: "idle" } | { state: "downloading"; percent: number } | { state: "installing" };

/** The banner and Settings label for the update button. */
export function updateLabel(p: UpdatePhase): string {
  if (p.state === "downloading") return `UPDATING ${p.percent}%`;
  return p.state === "installing" ? "INSTALLING…" : "UPDATE";
}
