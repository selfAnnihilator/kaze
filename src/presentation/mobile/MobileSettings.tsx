import { memo, useState, type ReactNode } from "react";
import { ChevronRight, Download, Folder, HardDrive, LogIn, LogOut, X } from "lucide-react";
import { FoldersPage, StoragePage, TastePage } from "./settings/SettingsPages";
import { LogOutDialog } from "./settings/LogOutDialog";
import { SettingsHead } from "./settings/SettingsHead";
import { useHiddenEntities, useMetadataSettings, type MetadataKey } from "./settings/useSettingsData";
import type { EntityFeedbackRow } from "../../types";
import type { MobileScreenProps } from "./types";

type Section = "main" | "taste" | "folders" | "system";
type Ctx = MobileScreenProps["c"];


function SwitchRow({ title, sub, on, disabled, onToggle }: { title: string; sub: string; on: boolean; disabled: boolean; onToggle: () => void }) {
  return (
    <button type="button" className="m-set-row" role="switch" aria-checked={on} disabled={disabled} onClick={onToggle}>
      <span className="m-set-row__text"><strong>{title}</strong><span>{sub}</span></span>
      <span className={`m-switch ${on ? "is-on" : ""}`} aria-hidden="true"><span className="m-switch__knob" /></span>
    </button>
  );
}

function OnlineMusic() {
  const m = useMetadataSettings();
  const row = (key: MetadataKey, title: string, sub: string) => (
    <SwitchRow title={title} sub={sub} on={m.prefs?.[key] ?? true} disabled={m.busy || !m.prefs} onToggle={() => m.toggle(key)} />
  );
  return (
    <>
      <SettingsHead num="01" title="ONLINE MUSIC & DETAILS" />
      {row("enable_jiosaavn", "Enable online music", "Stream and download from JioSaavn")}
      {row("auto_fill_metadata", "Fill missing artwork & details", "Automatically, while you listen")}
      {row("enable_lastfm_similarity", "Find similar artists with Last.fm", "Improves Flow and For You")}
      <div className="m-set-actions">
        <button type="button" className="m-btn m-btn--outline m-btn--sm m-set-actions__wide" disabled={m.busy || !m.prefs} onClick={() => m.fill(false)}>FILL MISSING ARTWORK NOW</button>
        <button type="button" className="m-btn m-btn--outline m-btn--sm m-btn--grow" disabled={m.busy || !m.prefs} onClick={() => m.fill(true)}>RETRY UNMATCHED</button>
        <button type="button" className="m-btn m-btn--outline m-btn--sm m-btn--grow m-set-actions__danger" disabled={m.busy || !m.prefs} onClick={m.removeFilled}>REMOVE FILLED DATA</button>
      </div>
      {m.note && <p className={`m-set-note ${m.note.error ? "is-error" : ""}`} role={m.note.error ? "alert" : "status"}>{m.note.text}</p>}
    </>
  );
}

const chipLabel = (row: EntityFeedbackRow) =>
  `${row.level === "less" ? "LESS" : row.kind.toUpperCase()} · ${row.display.toUpperCase()}`;

function Hidden() {
  const h = useHiddenEntities();
  return (
    <>
      <SettingsHead num="02" title="HIDDEN FROM RECOMMENDATIONS" />
      <div className="m-set-chips">
        {h.rows?.map((row) => (
          <button key={`${row.kind}:${row.key}`} type="button" className="m-set-chip" onClick={() => void h.remove(row)}
            aria-label={`Show ${row.display} in recommendations again`}>
            <span>{chipLabel(row)}</span><X size={12} strokeWidth={2} />
          </button>
        ))}
        {h.rows && !h.rows.length && <p className="m-set-note">Nothing hidden yet. Tap the thumbs-down on a song to hide an artist or hear less of a style.</p>}
        {h.error && <p className="m-set-note is-error" role="alert">Couldn’t load or update this list. Open Settings again to retry.</p>}
      </div>
    </>
  );
}

function LinkRow({ icon, title, meta, onOpen }: { icon: ReactNode; title: string; meta: string; onOpen: () => void }) {
  return (
    <button type="button" className="m-set-link" onClick={onOpen}>
      <span className="m-set-link__icon">{icon}</span>
      <span className="m-set-row__text"><strong>{title}</strong><span className="m-set-row__mono">{meta}</span></span>
      <ChevronRight size={18} strokeWidth={2} />
    </button>
  );
}

function Account({ c }: { c: Ctx }) {
  const [choosing, setChoosing] = useState(false);
  const user = c.currentUser;
  return (
    <>
      <SettingsHead num="04" title="ACCOUNT" />
      <div className="m-set-account">
        <span className="m-set-row__text">
          <strong>{user ? user.display_name || user.username : "Listening as a guest"}</strong>
          <span className="m-set-row__mono">{user ? `@${user.username.toUpperCase()} · SYNCS ACROSS DEVICES` : "SIGN IN TO SYNC ACROSS DEVICES"}</span>
        </span>
        {user ? (
          <button type="button" className="m-btn m-btn--outline m-btn--sm" onClick={() => setChoosing(true)}><LogOut size={18} strokeWidth={2} /><span>LOG OUT</span></button>
        ) : (
          <button type="button" className="m-btn m-btn--sm" onClick={() => c.setIsAuthModalOpen(true)}><LogIn size={18} strokeWidth={2} /><span>SIGN IN</span></button>
        )}
      </div>
      {choosing && <LogOutDialog onCancel={() => setChoosing(false)} onLogOut={c.handleLogout} />}
    </>
  );
}

/**
 * Settings (Figma F5/F6 on the phone): Online Music & Details, Hidden from
 * Recommendations, Library & Storage and Account. The PC-only sections
 * (interface style, background, startup) are hidden, as agreed.
 */
export const MobileSettings = memo(function MobileSettings({ c }: MobileScreenProps) {
  const [section, setSection] = useState<Section>("main");
  const back = () => setSection("main");
  const folders = c.onboardingStatus?.configured_folders || [];

  if (section === "taste") return <TastePage c={c} onBack={back} />;
  if (section === "folders") return <FoldersPage c={c} onBack={back} />;
  if (section === "system") return <StoragePage onBack={back} />;

  return (
    <div className="m-set">
      <div className="m-set-hero"><span className="m-kicker">THE DECK // CONFIG</span><h1 className="m-display m-set-hero__title">SETTINGS.</h1></div>
      <OnlineMusic />
      <Hidden />
      <SettingsHead num="03" title="LIBRARY & STORAGE" />
      <div className="m-set-links">
        <LinkRow icon={null} title="Music taste" meta="ARTISTS & GENRES YOU LIKE" onOpen={() => setSection("taste")} />
        <LinkRow icon={<Folder size={20} strokeWidth={2} />} title="Music folders" meta={`${folders.length} ${folders.length === 1 ? "FOLDER" : "FOLDERS"} · RESCAN`} onOpen={() => setSection("folders")} />
        <LinkRow icon={<Download size={20} strokeWidth={2} />} title="Download folder" meta={(c.settings?.downloads?.resolved_dir || "~/MUSIC/KAZE").toUpperCase()} onOpen={() => setSection("folders")} />
        <LinkRow icon={<HardDrive size={20} strokeWidth={2} />} title="System & storage" meta="TEMPORARY AUDIO" onOpen={() => setSection("system")} />
      </div>
      <Account c={c} />
    </div>
  );
});
