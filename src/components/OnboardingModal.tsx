import React, { useState } from "react";
import { Folder, Check } from "lucide-react";

interface OnboardingModalProps {
  defaultMusicDir: string;
  onComplete: (folders: string[], startScan: boolean) => Promise<void>;
}

export const OnboardingModal: React.FC<OnboardingModalProps> = ({
  defaultMusicDir,
  onComplete,
}) => {
  const [pending, setPending] = useState(false);
  const busy = React.useRef(false);
  const [error, setError] = useState<string | null>(null);
  const [useDefault, setUseDefault] = useState(true);
  const [customPath, setCustomPath] = useState("");
  const [startScan, setStartScan] = useState(true);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (busy.current) return;
    const folder = useDefault ? defaultMusicDir : customPath.trim();
    if (folder) {
      busy.current = true; setPending(true); setError(null);
      try { await onComplete([folder], startScan); }
      catch (failure) { setError(failure instanceof Error ? failure.message : String(failure)); }
      finally { busy.current = false; setPending(false); }
    }
  };

  return (
    <div className="modal-overlay">
      <div className="modal-content">
        <div style={{ display: "flex", alignItems: "center", gap: "var(--space-10)", marginBottom: "var(--space-16)" }}>
          <img className="kage-logo" src="/kaze-icon.png" alt="" />
          <h2 style={{ fontSize: "var(--fs-22)", fontWeight: "var(--weight-bold)" }}>Welcome to Kaze</h2>
        </div>

        <p style={{ color: "var(--text-muted)", fontSize: "var(--fs-15)", lineHeight: 1.5, marginBottom: "var(--space-20)" }}>
          Kaze is a local-first, privacy-respecting music player. To get started, confirm your
          music directory. Kaze stays strictly within this folder and never scans outside your
          approved roots.
        </p>

        {error && <p role="alert">Couldn’t save your music folder. {error}</p>}
        <form aria-busy={pending} onSubmit={handleSubmit} style={{ display: "flex", flexDirection: "column", gap: "var(--space-16)" }}>
          {/* Default Folder Option */}
          <div
            onClick={() => {
              setUseDefault(true);
            }}
            style={{
              padding: "var(--space-14)",
              borderRadius: "var(--radius-8)",
              border: useDefault ? "2px solid var(--accent)" : "1px solid var(--border)",
              backgroundColor: useDefault ? "var(--accent-a8)" : "var(--bg-card)",
              cursor: "pointer",
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: "var(--space-12)" }}>
              <Folder size={20} color={useDefault ? "var(--accent-light)" : "var(--palette-gray-350)"} />
              <div>
                <div style={{ fontWeight: "var(--weight-semibold)", fontSize: "var(--fs-14)" }}>Use Default System Music Directory</div>
                <div style={{ fontSize: "var(--fs-13)", color: "var(--text-dim)", wordBreak: "break-all" }}>
                  {defaultMusicDir || "~/Music"}
                </div>
              </div>
            </div>
            {useDefault && <Check size={18} color="var(--accent)" />}
          </div>

          {/* Custom Folder Option */}
          <div
            onClick={() => setUseDefault(false)}
            style={{
              padding: "var(--space-14)",
              borderRadius: "var(--radius-8)",
              border: !useDefault ? "2px solid var(--accent)" : "1px solid var(--border)",
              backgroundColor: !useDefault ? "var(--accent-a8)" : "var(--bg-card)",
              cursor: "pointer",
            }}
          >
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "var(--space-8)" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "var(--space-12)" }}>
                <Folder size={20} color={!useDefault ? "var(--accent-light)" : "var(--palette-gray-350)"} />
                <span style={{ fontWeight: "var(--weight-semibold)", fontSize: "var(--fs-14)" }}>Choose Custom Music Directory</span>
              </div>
              {!useDefault && <Check size={18} color="var(--accent)" />}
            </div>

            {!useDefault && (
              <input
                type="text"
                placeholder="/path/to/your/music/folder"
                value={customPath}
                onChange={(e) => setCustomPath(e.target.value)}
                style={{
                  width: "100%",
                  padding: "var(--space-8) var(--space-12)",
                  borderRadius: "var(--radius-6)",
                  backgroundColor: "var(--bg-sidebar)",
                  border: "1px solid var(--border-light)",
                  color: "var(--text-main)",
                  fontSize: "var(--fs-14)",
                  marginTop: "var(--space-8)",
                }}
                autoFocus
              />
            )}
          </div>

          {/* Start Scan Checkbox */}
          <label style={{ display: "flex", alignItems: "center", gap: "var(--space-10)", fontSize: "var(--fs-14)", cursor: "pointer" }}>
            <input
              type="checkbox"
              checked={startScan}
              onChange={(e) => setStartScan(e.target.checked)}
              style={{ accentColor: "var(--accent)" }}
            />
            <span>Automatically scan and index tracks now</span>
          </label>

          <button
            type="submit"
            className="btn btn-primary"
            style={{ padding: "var(--space-12)", justifyContent: "center", marginTop: "var(--space-8)" }}
            disabled={pending || (!useDefault && !customPath.trim())}
          >
            Confirm & Start Kaze
          </button>
        </form>
      </div>
    </div>
  );
};
