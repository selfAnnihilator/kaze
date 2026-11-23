import { useState } from "react";
import { ArrowRight, Check, ChevronRight, FolderCheck, FolderOpen, FolderPlus } from "lucide-react";
import { ensureMusicAccess, pickMusicFolder } from "../androidBridge";

type Complete = (folders: string[], startScan: boolean) => Promise<void>;

/** The folder choice: the default folder, or one picked with Android's picker (or typed). */
function useFolderChoice(defaultMusicDir: string, onComplete: Complete) {
  const [useDefault, setUseDefault] = useState(true);
  const [path, setPath] = useState("");
  const [scan, setScan] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  /** "Choose a folder": Android's picker fills the path; the typed field stays as a fallback. */
  const choose = async () => {
    setUseDefault(false);
    setError(null);
    try {
      const picked = await pickMusicFolder();
      if (picked) setPath(picked);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const finish = async () => {
    const folder = useDefault ? defaultMusicDir : path.trim();
    if (!folder) return setError("Type the folder’s path, e.g. /storage/emulated/0/Music.");
    setBusy(true);
    setError(null);
    try {
      await ensureMusicAccess();
      await onComplete([folder], scan);
    } catch (e) {
      setError(`Couldn’t use this folder. ${e instanceof Error ? e.message : String(e)}`);
      setBusy(false);
    }
  };

  return { useDefault, setUseDefault, path, setPath, scan, setScan, busy, error, choose, finish };
}

/**
 * First run, step 1 (Figma A2 "WELCOME TO KAZE."): the default music folder or
 * one you choose, "Scan automatically now", then Continue.
 */
export function MobileOnboarding({ defaultMusicDir, onComplete }: { defaultMusicDir: string; onComplete: Complete }) {
  const { useDefault, setUseDefault, path, setPath, scan, setScan, busy, error, choose, finish } = useFolderChoice(defaultMusicDir, onComplete);

  return (
    <section className="m-sheet m-onboard" role="dialog" aria-modal="true" aria-label="Welcome to Kaze">
      <div className="m-onboard__head">
        <span className="m-kicker">SETUP 01 / 02 // LIBRARY</span>
        <h1 className="m-display m-onboard__title">WELCOME TO KAZE.</h1>
        <p className="m-onboard__intro">Kaze plays the music on this device. Tell it where your music lives and it will build your library.</p>
      </div>
      <div className="m-onboard__choices">
        <button type="button" className={`m-onboard__choice ${useDefault ? "is-on" : ""}`} aria-pressed={useDefault} onClick={() => setUseDefault(true)}>
          <FolderCheck size={24} strokeWidth={2} />
          <span className="m-set-row__text"><strong>Use default folder</strong><span className="m-set-row__mono">{defaultMusicDir.toUpperCase()}</span></span>
          <span className={`m-check ${useDefault ? "is-on" : ""}`}>{useDefault && <Check size={14} strokeWidth={2} />}</span>
        </button>
        <button type="button" className={`m-onboard__choice ${useDefault ? "" : "is-on"}`} aria-pressed={!useDefault} onClick={() => void choose()}>
          <FolderOpen size={24} strokeWidth={2} />
          <span className="m-set-row__text"><strong>Choose a folder</strong><span className="m-set-row__mono">{!useDefault && path ? path.toUpperCase() : "PICK ANY FOLDER ON THIS DEVICE"}</span></span>
          <ChevronRight size={20} strokeWidth={2} />
        </button>
        {!useDefault && (
          <label className="m-field m-field--sm">
            <FolderPlus size={18} strokeWidth={2} className="m-field__icon" />
            <input className="m-field__input" value={path} onChange={(e) => setPath(e.target.value)} placeholder="Folder path, or pick one above…" aria-label="Folder path" autoCapitalize="none" />
          </label>
        )}
        <button type="button" className="m-onboard__scan" role="checkbox" aria-checked={scan} onClick={() => setScan(!scan)}>
          <span className={`m-check ${scan ? "is-on" : ""}`}>{scan && <Check size={14} strokeWidth={2} />}</span>
          <span>Scan automatically now</span>
        </button>
        {error && <p className="m-set-note is-error" role="alert">{error}</p>}
      </div>
      <div className="m-onboard__actions">
        <button type="button" className="m-btn m-btn--grow m-taste__go" disabled={busy} onClick={() => void finish()}>
          <ArrowRight size={18} strokeWidth={2} /><span>{busy ? "SETTING UP…" : "CONTINUE"}</span>
        </button>
      </div>
    </section>
  );
}
