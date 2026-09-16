import React, { useEffect, useState } from "react";
import { isTauri } from "../services/api";

interface UpdateBannerProps {}

/**
 * Listens for the "update-available" event emitted by the Rust backend.
 * Shows a subtle banner at the top of the app when a new version is downloading.
 * The app restarts automatically once the update is installed.
 */
export const UpdateBanner: React.FC<UpdateBannerProps> = () => {
  const [updateVersion, setUpdateVersion] = useState<string | null>(null);
  const [dismissed, setDismissed] = useState(false);

  useEffect(() => {
    if (!isTauri()) return;

    let unlisten: (() => void) | undefined;

    (async () => {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        unlisten = await listen<{ version: string }>("update-available", (e) => {
          setUpdateVersion(e.payload.version);
        });
      } catch {
        // Not in Tauri context, ignore
      }
    })();

    return () => {
      unlisten?.();
    };
  }, []);

  if (!updateVersion || dismissed) return null;

  return (
    <div
      style={{
        position: "fixed",
        top: 0,
        left: 0,
        right: 0,
        zIndex: 9999,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        gap: "10px",
        padding: "8px 16px",
        background: "rgba(30, 30, 36, 0.98)",
        borderBottom: "1px solid rgba(255,255,255,0.08)",
        fontSize: "13px",
        color: "rgba(255,255,255,0.85)",
        fontFamily: "inherit",
      }}
    >
      <span
        style={{
          width: 7,
          height: 7,
          borderRadius: "50%",
          background: "#4ade80",
          flexShrink: 0,
          animation: "kaze-pulse 2s infinite",
        }}
      />
      <span>
        Kaze <strong style={{ color: "#fff" }}>v{updateVersion}</strong> is
        downloading — the app will restart automatically when ready.
      </span>
      <button
        onClick={() => setDismissed(true)}
        style={{
          marginLeft: "auto",
          background: "none",
          border: "none",
          color: "rgba(255,255,255,0.4)",
          cursor: "pointer",
          fontSize: "16px",
          lineHeight: 1,
          padding: "0 4px",
        }}
        aria-label="Dismiss update notification"
      >
        ×
      </button>
    </div>
  );
};
