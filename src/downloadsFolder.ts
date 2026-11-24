/** Which section a song belongs to, by Kaze's download folder (no imports: shared and testable). */

type WithPath = { file_path?: string | null };

/** True when a track lives in Kaze's download folder (not the user's library). */
export const inDownloadsFolder = (track: WithPath, downloadsDir?: string) =>
  !!downloadsDir && String(track.file_path ?? "").startsWith(downloadsDir.replace(/\/+$/, "") + "/");

/** Whether a track belongs in a section: Downloads = Kaze's folder, Local = everything else. */
export const inSection = (track: WithPath, downloadsDir: string | undefined, section: "local" | "downloads") =>
  inDownloadsFolder(track, downloadsDir) === (section === "downloads");

/**
 * Songs from the user's own folders take no like or dislike (PC and phone).
 * Online songs (no file, or an `online://` placeholder) and Kaze downloads do.
 */
export const canRateFile = (track: WithPath | null | undefined, downloadsDir?: string) =>
  !!track && (!track.file_path || track.file_path.startsWith("online://") || inDownloadsFolder(track, downloadsDir));
