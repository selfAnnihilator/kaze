// The OS login entry is the source of truth, including changes made outside Kaze.
export async function getAutostart(): Promise<boolean> {
  if (typeof window === "undefined" || !("__TAURI_INTERNALS__" in window))
    throw new Error("Login startup is available in the desktop app.");
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<boolean>("get_autostart");
}

export async function setAutostart(enabled: boolean): Promise<boolean> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<boolean>("set_autostart", { enabled });
}
