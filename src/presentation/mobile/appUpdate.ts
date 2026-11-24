import { useSyncExternalStore } from "react";
import { isTauri } from "../../services/api";
import { showPhoneToast } from "./phoneToasts";
import type { UpdatePhase } from "./updatePhase";

export { updateLabel, type UpdatePhase } from "./updatePhase";


let phase: UpdatePhase = { state: "idle" };
const listeners = new Set<() => void>();
function setPhase(next: UpdatePhase) {
  phase = next;
  listeners.forEach((l) => l());
}

/** The current update phase, for the banner and Settings (no App state: a small pub-sub). */
export function useUpdatePhase(): UpdatePhase {
  return useSyncExternalStore((l) => { listeners.add(l); return () => { listeners.delete(l); }; }, () => phase, () => phase);
}

// Android reports a failed install after `android_install_update` returns, so
// the listeners outlive the call; only one pair ever exists.
let stopListening: (() => void) | null = null;
function stopUpdateListeners() {
  stopListening?.();
  stopListening = null;
}

async function listenWhileUpdating() {
  stopUpdateListeners();
  const { listen } = await import("@tauri-apps/api/event");
  const progress = await listen<{ percent: number }>("update-progress", (e) => {
    if (phase.state === "downloading") setPhase({ state: "downloading", percent: Math.min(100, e.payload.percent) });
  });
  const failed = await listen<{ message?: string }>("update-failed", (e) => {
    stopUpdateListeners();
    setPhase({ state: "idle" });
    showPhoneToast(`The update didn’t install: ${e.payload.message ?? "unknown error"}`);
  });
  stopListening = () => { progress(); failed(); };
}

/**
 * The banner's UPDATE button (Figma G10): Kaze downloads the new APK, checks
 * it, and installs it itself. Android restarts the app on the new version.
 */
export async function startAppUpdate(_version: string) {
  if (!isTauri() || phase.state !== "idle") return;
  setPhase({ state: "downloading", percent: 0 });
  try {
    await listenWhileUpdating();
    const { invoke } = await import("@tauri-apps/api/core");
    await invoke("android_install_update");
    setPhase({ state: "installing" });
  } catch (error) {
    stopUpdateListeners();
    setPhase({ state: "idle" });
    showPhoneToast(String(error).replace(/^Error: /, ""));
  }
}

/** Settings › Check for updates: the newer version (the banner appears too), or null when up to date. */
export async function checkForUpdate(): Promise<string | null> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<string | null>("android_check_update");
}

/** The installed version, e.g. "2.1.1". */
export async function installedVersion(): Promise<string> {
  const { getVersion } = await import("@tauri-apps/api/app");
  return getVersion();
}
