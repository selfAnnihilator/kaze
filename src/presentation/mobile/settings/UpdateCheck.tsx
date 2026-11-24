import { useEffect, useState } from "react";
import { ArrowUpCircle, RefreshCw } from "lucide-react";
import { checkForUpdate, installedVersion, startAppUpdate, updateLabel, useUpdatePhase } from "../appUpdate";

type Found = { kind: "idle" } | { kind: "checking" } | { kind: "latest" } | { kind: "update"; version: string } | { kind: "error"; text: string };

/** Settings › System & storage: the installed version and Check for updates. */
export function UpdateCheck() {
  const [version, setVersion] = useState("");
  const [found, setFound] = useState<Found>({ kind: "idle" });
  const phase = useUpdatePhase();
  useEffect(() => { void installedVersion().then(setVersion).catch(() => undefined); }, []);
  const check = async () => {
    setFound({ kind: "checking" });
    try {
      const next = await checkForUpdate();
      setFound(next ? { kind: "update", version: next } : { kind: "latest" });
    } catch (error) {
      setFound({ kind: "error", text: String(error).replace(/^Error: /, "") });
    }
  };
  return (
    <div className="m-set-pad m-set-pad--top">
      {found.kind === "update" ? (
        <button type="button" className="m-btn m-set-big" disabled={phase.state !== "idle"} onClick={() => void startAppUpdate(found.version)}>
          <ArrowUpCircle size={18} strokeWidth={2} /><span>{phase.state === "idle" ? `UPDATE TO V${found.version}` : updateLabel(phase)}</span>
        </button>
      ) : (
        <button type="button" className="m-btn m-btn--outline m-set-big" disabled={found.kind === "checking"} onClick={() => void check()}>
          <RefreshCw size={18} strokeWidth={2} /><span>{found.kind === "checking" ? "CHECKING…" : "CHECK FOR UPDATES"}</span>
        </button>
      )}
      <p className={`m-set-note ${found.kind === "error" ? "is-error" : ""}`} role="status">
        {found.kind === "latest" && `Kaze ${version ? `v${version} ` : ""}is up to date.`}
        {found.kind === "update" && `Kaze v${found.version} is available. Kaze downloads and installs it, then closes; open it again to use the new version.`}
        {found.kind === "error" && found.text}
        {(found.kind === "idle" || found.kind === "checking") && (version ? `Kaze v${version}` : "")}
      </p>
    </div>
  );
}
