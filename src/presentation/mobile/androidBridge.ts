import { isTauri } from "../../services/api";

/** The Android-only backend commands exist only in the phone build. */
const onAndroid = () => isTauri() && /Android/i.test(navigator.userAgent);

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

/**
 * Opens Android's folder picker and returns the folder's path (after making
 * sure Kaze may read music), or null when cancelled or not on Android.
 */
export async function pickMusicFolder(): Promise<string | null> {
  if (!onAndroid()) return null;
  return call<string | null>("android_pick_music_folder");
}

/** Asks for permission to read music before folders are added or scanned. Throws if refused. */
export async function ensureMusicAccess(): Promise<void> {
  if (!onAndroid()) return;
  if (!(await call<boolean>("android_request_audio_access"))) {
    throw new Error("Kaze needs permission to read your music. Allow “Music and audio” for Kaze in Android settings.");
  }
}

/** Shares text through the Android share sheet; elsewhere the Web Share API or the clipboard. */
export async function shareText(title: string, text: string): Promise<"shared" | "copied"> {
  if (onAndroid()) {
    await call<void>("android_share", { title, text });
    return "shared";
  }
  if (navigator.share) {
    await navigator.share({ title, text });
    return "shared";
  }
  await navigator.clipboard.writeText(text);
  return "copied";
}
