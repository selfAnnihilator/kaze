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
    let disposed = false;

    (async () => {
      try {
        const { listen } = await import("@tauri-apps/api/event");
        const release = await listen<{ version: string }>(
          "update-available",
          (e) => {
            if (disposed) return;
            setUpdateVersion(e.payload.version);
          },
        );
        if (disposed) release();
        else unlisten = release;
      } catch {
        // Not in Tauri context, ignore
      }
    })();

    return () => {
      disposed = true;
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
        zIndex: "var(--z-dialog)",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        gap: "var(--space-10)",
        padding: "var(--space-8) var(--space-16)",
        background: "var(--palette-gray-850-a95)",
        borderBottom: "1px solid var(--overlay-8)",
        fontSize: "var(--fs-13)",
        color: "var(--overlay-85)",
        fontFamily: "inherit",
      }}
    >
      <span
        style={{
          width: 7,
          height: 7,
          borderRadius: "50%",
          background: "var(--palette-green-400)",
          flexShrink: 0,
          animation: "kaze-pulse var(--duration-loop) infinite",
        }}
      />
      <span>
        Kaze <strong style={{ color: "var(--white)" }}>v{updateVersion}</strong> is
        downloading — the app will restart automatically when ready.
      </span>
      <button
        onClick={() => setDismissed(true)}
        style={{
          marginLeft: "auto",
          background: "none",
          border: "none",
          color: "var(--overlay-40)",
          cursor: "pointer",
          fontSize: "var(--fs-16)",
          lineHeight: 1,
          padding: "0 var(--space-4)",
        }}
        aria-label="Dismiss update notification"
      >
        ×
      </button>
    </div>
  );
};
