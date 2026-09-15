import React, { useState } from "react";
import { Folder, Check } from "lucide-react";

interface OnboardingModalProps {
  defaultMusicDir: string;
  onComplete: (folders: string[], startScan: boolean) => void;
}

export const OnboardingModal: React.FC<OnboardingModalProps> = ({
  defaultMusicDir,
  onComplete,
}) => {
  const [useDefault, setUseDefault] = useState(true);
  const [customPath, setCustomPath] = useState("");
  const [startScan, setStartScan] = useState(true);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const folder = useDefault ? defaultMusicDir : customPath.trim();
    if (folder) {
      onComplete([folder], startScan);
    }
  };

  return (
    <div className="modal-overlay">
      <div className="modal-content">
        <div style={{ display: "flex", alignItems: "center", gap: "10px", marginBottom: "16px" }}>
          <img className="kage-logo" src="/kaze-icon.png" alt="" />
          <h2 style={{ fontSize: "1.4rem", fontWeight: 700 }}>Welcome to Kaze</h2>
        </div>

        <p style={{ color: "var(--text-muted)", fontSize: "0.95rem", lineHeight: 1.5, marginBottom: "20px" }}>
          Kaze is a local-first, privacy-respecting music player. To get started, confirm your
          music directory. Kaze stays strictly within this folder and never scans outside your
          approved roots.
        </p>

        <form onSubmit={handleSubmit} style={{ display: "flex", flexDirection: "column", gap: "16px" }}>
          {/* Default Folder Option */}
          <div
            onClick={() => {
              setUseDefault(true);
            }}
            style={{
              padding: "14px",
              borderRadius: "8px",
              border: useDefault ? "2px solid var(--accent)" : "1px solid var(--border)",
              backgroundColor: useDefault ? "rgba(243, 112, 30, 0.08)" : "var(--bg-card)",
              cursor: "pointer",
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
            }}
          >
            <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
              <Folder size={20} color={useDefault ? "var(--accent-light)" : "#9ca3af"} />
              <div>
                <div style={{ fontWeight: 600, fontSize: "0.9rem" }}>Use Default System Music Directory</div>
                <div style={{ fontSize: "0.8rem", color: "var(--text-dim)", wordBreak: "break-all" }}>
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
              padding: "14px",
              borderRadius: "8px",
              border: !useDefault ? "2px solid var(--accent)" : "1px solid var(--border)",
              backgroundColor: !useDefault ? "rgba(243, 112, 30, 0.08)" : "var(--bg-card)",
              cursor: "pointer",
            }}
          >
            <div style={{ display: "flex", alignItems: "center", justifyContent: "space-between", marginBottom: "8px" }}>
              <div style={{ display: "flex", alignItems: "center", gap: "12px" }}>
                <Folder size={20} color={!useDefault ? "var(--accent-light)" : "#9ca3af"} />
                <span style={{ fontWeight: 600, fontSize: "0.9rem" }}>Choose Custom Music Directory</span>
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
                  padding: "8px 12px",
                  borderRadius: "6px",
                  backgroundColor: "var(--bg-sidebar)",
                  border: "1px solid var(--border-light)",
                  color: "#e8d8c9",
                  fontSize: "0.88rem",
                  marginTop: "8px",
                }}
                autoFocus
              />
            )}
          </div>

          {/* Start Scan Checkbox */}
          <label style={{ display: "flex", alignItems: "center", gap: "10px", fontSize: "0.88rem", cursor: "pointer" }}>
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
            style={{ padding: "12px", justifyContent: "center", marginTop: "8px" }}
            disabled={!useDefault && !customPath.trim()}
          >
            Confirm & Start Kaze
          </button>
        </form>
      </div>
    </div>
  );
};
