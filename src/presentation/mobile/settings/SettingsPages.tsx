import { useState, type FormEvent, type ReactNode } from "react";
import { ArrowLeft, Folder, FolderOpen, FolderPlus, Lock, RefreshCw, Trash2 } from "lucide-react";
import { ensureMusicAccess, pickMusicFolder } from "../androidBridge";
import { MobileMusicTaste } from "./MobileMusicTaste";
import { formatBytes, useAudioCache } from "./useSettingsData";
import { SettingsHead } from "./SettingsHead";
import { UpdateCheck } from "./UpdateCheck";
import type { MobileScreenProps } from "../types";

type Ctx = MobileScreenProps["c"];

function SubPage({ title, onBack, children }: { title: string; onBack: () => void; children: ReactNode }) {
  return (
    <div className="m-set">
      <button type="button" className="m-set-back" onClick={onBack}><ArrowLeft size={20} strokeWidth={2} /><span>BACK TO SETTINGS</span></button>
      <div className="m-set-hero"><span className="m-kicker">03 // LIBRARY &amp; STORAGE</span><h1 className="m-display m-set-hero__sub">{title}</h1></div>
      <div className="m-set-rule" />
      {children}
    </div>
  );
}

/** Settings → Music taste: the A3 screen with Save instead of Start Listening. */
export function TastePage({ c, onBack }: { c: Ctx; onBack: () => void }) {
  return (
    <div className="m-set">
      <button type="button" className="m-set-back" onClick={onBack}><ArrowLeft size={20} strokeWidth={2} /><span>BACK TO SETTINGS</span></button>
      <MobileMusicTaste userId={c.currentUser?.id || "default"} kicker="03 // LIBRARY & STORAGE" doneLabel="SAVE"
        onSaved={() => { c.handleTasteSaved(); onBack(); }} />
    </div>
  );
}

/** Music Folders (Figma F7): folders with remove, add a path, rescan, and the download folder. */
export function FoldersPage({ c, onBack }: { c: Ctx; onBack: () => void }) {
  const [path, setPath] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const folders = c.onboardingStatus?.configured_folders || [];
  const act = async (work: () => Promise<void>, failed: string) => {
    setBusy(true);
    setError(null);
    try { await work(); } catch (e) { setError(`${failed} ${e instanceof Error ? e.message : String(e)}`); } finally { setBusy(false); }
  };
  const add = (e: FormEvent) => {
    e.preventDefault();
    const value = path.trim();
    if (!value) return setError("Type the folder’s path first, e.g. /storage/emulated/0/Music.");
    void act(async () => { await ensureMusicAccess(); await c.handleAddFolder(value); setPath(""); }, "Couldn’t add this folder.");
  };
  const pick = () => void act(async () => {
    const picked = await pickMusicFolder();
    if (picked) await c.handleAddFolder(picked);
  }, "Couldn’t add this folder.");
  const rescan = () => void act(async () => { await ensureMusicAccess(); await c.handleRescanLibrary(); }, "Couldn’t start the scan.");

  return (
    <SubPage title="MUSIC FOLDERS." onBack={onBack}>
      {folders.map((f) => (
        <div key={f.id} className="m-set-folder">
          <Folder size={20} strokeWidth={2} />
          <span className="m-set-row__text"><span className="m-set-folder__path">{f.path}</span><span className="m-set-row__mono">{(f.track_count ?? 0).toLocaleString("en-US")} TRACKS</span></span>
          <button type="button" className="m-set-folder__remove" disabled={busy} onClick={() => void act(() => c.handleRemoveFolder(f.id), "Couldn’t remove this folder.")} aria-label={`Remove ${f.path}`}>
            <Trash2 size={18} strokeWidth={2} />
          </button>
        </div>
      ))}
      {!folders.length && <p className="m-set-note m-set-pad">No music folders yet. Add the folder that holds your songs below.</p>}
      <div className="m-set-pad m-set-pad--top">
        <button type="button" className="m-btn m-btn--outline m-set-big" disabled={busy} onClick={pick}>
          <FolderOpen size={18} strokeWidth={2} /><span>CHOOSE A FOLDER</span>
        </button>
      </div>
      <form className="m-set-add" onSubmit={add}>
        <label className="m-field m-field--sm">
          <FolderPlus size={18} strokeWidth={2} className="m-field__icon" />
          <input className="m-field__input" value={path} onChange={(e) => setPath(e.target.value)} placeholder="Or type a folder path…" aria-label="Folder path" autoCapitalize="none" />
        </label>
        <button type="submit" className="m-btn m-btn--sm m-set-add__btn" disabled={busy}>ADD</button>
      </form>
      <div className="m-set-pad m-set-pad--top">
        <button type="button" className="m-btn m-set-big" disabled={busy || c.isScanning} onClick={rescan}>
          <RefreshCw size={18} strokeWidth={2} /><span>{c.isScanning ? "SCANNING…" : "RESCAN LIBRARY"}</span>
        </button>
      </div>
      {error && <p className="m-set-note is-error m-set-pad" role="alert">{error}</p>}
      <SettingsHead num="—" title="DOWNLOAD FOLDER" />
      <div className="m-set-pad"><div className="m-set-locked"><Lock size={16} strokeWidth={2} /><span>{(c.settings?.downloads?.resolved_dir || "~/MUSIC/KAZE").toUpperCase()}</span></div></div>
    </SubPage>
  );
}

/** System & Storage (Figma F8): temporary audio size and file count, the cache limit bar, Clear Cache. */
export function StoragePage({ onBack }: { onBack: () => void }) {
  const s = useAudioCache();
  const used = s.cache?.total_size_bytes ?? 0;
  const max = s.cache?.max_size_bytes ?? 0;
  const ratio = max > 0 ? Math.min(1, used / max) : 0;
  return (
    <SubPage title="SYSTEM & STORAGE." onBack={onBack}>
      <div className="m-set-stats">
        <div className="m-set-stat"><span className="m-set-row__mono">TEMPORARY AUDIO</span><strong className="is-red">{s.cache ? formatBytes(used) : "—"}</strong></div>
        <div className="m-set-stat"><span className="m-set-row__mono">FILES</span><strong>{s.cache ? s.cache.file_count.toLocaleString("en-US") : "—"}</strong></div>
      </div>
      <div className="m-set-limit">
        <div className="m-set-limit__head"><span>CACHE LIMIT</span><span className="m-set-row__mono">{s.cache ? `${formatBytes(used)} / ${max ? formatBytes(max) : "NO LIMIT"}` : "—"}</span></div>
        <div className="m-set-limit__bar"><div className="m-set-limit__fill" style={{ transform: `scaleX(${ratio})` }} /></div>
        <p className="m-set-note">Temporary audio makes replays instant. Clearing it never touches your library or downloads.</p>
      </div>
      <div className="m-set-pad m-set-pad--top">
        <button type="button" className="m-btn m-set-big" disabled={s.busy || !used} onClick={() => void s.clear()}>
          <Trash2 size={18} strokeWidth={2} /><span>{s.busy ? "CLEARING…" : "CLEAR CACHE"}</span>
        </button>
      </div>
      {s.error && <p className="m-set-note is-error m-set-pad" role="alert">Couldn’t read or clear temporary audio. Open this page again to retry.</p>}
      <SettingsHead num="02" title="APP UPDATES" />
      <UpdateCheck />
    </SubPage>
  );
}
