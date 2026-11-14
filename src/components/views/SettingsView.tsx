import { MusicTaste } from "../MusicTaste";
import {
  memo,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import {
  ArrowLeft,
  ArrowRight,
  Check,
  Database,
  Folder,
  LogOut,
  Download,
} from "lucide-react";
import type {
  AppSettings,
  UserProfile,
} from "../../types";
import { dispatchCommand } from "../../services/api";
import type { InterfaceStyle } from "../../presentation/InterfaceContext";
import { useViewMemory } from "../../presentation/ViewMemory";
import { MetadataSettings } from "../MetadataSettings";
import { EntityFeedbackSettings } from "../EntityFeedbackSettings";
import { StartupSettings } from "../StartupSettings";
import { FolderSettings, type MusicFolder } from "./settings/FolderSettings";
import { StorageSettings } from "./settings/StorageSettings";

type Section = "main" | "folders" | "system" | "taste";
interface SettingsViewProps {
  onRecommendationSetupSaved?: () => void;
  interfaceStyle: InterfaceStyle;
  onSelectInterfaceStyle: (style: InterfaceStyle) => Promise<void>;
  isSavingInterfaceStyle?: boolean;
  interfaceStyleError?: string | null;
  settings: AppSettings | null;
  configuredFolders: MusicFolder[];
  currentUser: UserProfile | null;
  onAddFolder: (path: string) => Promise<void>;
  onRemoveFolder: (folderId: string) => Promise<void>;
  onRescanLibrary: () => Promise<void>;
  onReloadSettings?: () => Promise<void>;
  onLogout: (scope: "device" | "all") => Promise<void>;
  onOpenAuthModal: () => void;
  isScanning?: boolean;
}

export const SettingsView = memo(function SettingsView({
  onRecommendationSetupSaved,
  interfaceStyle,
  onSelectInterfaceStyle,
  isSavingInterfaceStyle,
  interfaceStyleError,
  settings,
  configuredFolders,
  currentUser,
  onAddFolder,
  onRemoveFolder,
  onRescanLibrary,
  onReloadSettings,
  onLogout,
  onOpenAuthModal,
  isScanning = false,
}: SettingsViewProps) {
  const memory = useViewMemory("settings", () => ({
    section: "main" as Section,
  }));
  const [section, setSection] = useState<Section>(memory.section);
  const [closeBehavior, setCloseBehavior] = useState(
    settings?.close_button_behavior ?? "minimize_to_tray",
  );
  const [savingClose, setSavingClose] = useState(false);
  const [closeError, setCloseError] = useState<string | null>(null);
  const closeRequest = useRef(false);
  const [loggingOut, setLoggingOut] = useState(false);
  const [logoutChoiceOpen, setLogoutChoiceOpen] = useState(false);
  const [logoutError, setLogoutError] = useState<string | null>(null);
  const logoutDialog = useRef<HTMLDialogElement>(null);
  const logoutTrigger = useRef<HTMLButtonElement>(null);
  const logoutRequest = useRef(false);
  const root = useRef<HTMLDivElement>(null);
  const focusSection = useRef(false);
  useEffect(() => {
    const dialog = logoutDialog.current;
    if (!currentUser) {
      setLogoutChoiceOpen(false);
      return;
    }
    if (!logoutChoiceOpen || !dialog) return;
    dialog.showModal();
    dialog.querySelector<HTMLButtonElement>(".settings-logout-cancel")?.focus();
    return () => {
      dialog.close();
      logoutTrigger.current?.focus({ preventScroll: true });
    };
  }, [logoutChoiceOpen, currentUser?.id]);

  useEffect(() => {
    if (settings?.close_button_behavior)
      setCloseBehavior(settings.close_button_behavior);
  }, [settings?.close_button_behavior]);
  useLayoutEffect(() => {
    memory.section = section;
    if (!focusSection.current) return;
    const page = root.current?.closest<HTMLElement>(".main-content");
    if (page) page.scrollTop = 0;
    root.current
      ?.querySelector<HTMLElement>("h1")
      ?.focus({ preventScroll: true });
    focusSection.current = false;
  }, [section, memory]);
  const openSection = (next: Section) => {
    focusSection.current = true;
    setSection(next);
  };

  const saveCloseBehavior = async (
    behavior: "minimize_to_tray" | "quit_app",
  ) => {
    if (closeRequest.current || behavior === closeBehavior) return;
    closeRequest.current = true;
    setSavingClose(true);
    setCloseError(null);
    try {
      await dispatchCommand({
        command: "SetCloseButtonBehavior",
        payload: { behavior },
      });
      setCloseBehavior(behavior);
      try {
        await onReloadSettings?.();
      } catch {
        setCloseError("Your choice was saved. Reopen Settings to refresh it.");
      }
    } catch {
      setCloseError(
        "Couldn't save this choice. Your previous close behavior is still active.",
      );
    } finally {
      closeRequest.current = false;
      setSavingClose(false);
    }
  };
  const logOut = async (scope: "device" | "all") => {
    if (logoutRequest.current) return;
    logoutRequest.current = true;
    setLoggingOut(true);
    setLogoutError(null);
    try {
      await onLogout(scope);
      setLogoutChoiceOpen(false);
    } catch {
      setLogoutError(
        scope === "all"
          ? "Couldn't log out on all devices. Check your connection and try again."
          : "Couldn't log out. Please try again.",
      );
    } finally {
      logoutRequest.current = false;
      setLoggingOut(false);
    }
  };
  const title =
    section === "main"
      ? "Settings"
      : section === "folders"
        ? "Music Folders"
        : section === "taste" ? "Music taste" : "System & Storage";
  return (
    <div className={`settings-view settings-view--${section}`} ref={root}>
      <header className="settings-header">
        {section !== "main" && (
          <button
            type="button"
            className="settings-back"
            onClick={() => openSection("main")}
          >
            <ArrowLeft size={17} /> Back to Settings
          </button>
        )}
        <h1 tabIndex={-1}>
          {title}
          <span aria-hidden="true">.</span>
        </h1>
      </header>
      {section === "main" ? (
        <div className="settings-stack">
          <section
            className="settings-section interface-picker"
            aria-labelledby="interface-style-heading"
          >
            <h2 id="interface-style-heading">Interface Style</h2>
            <div
              className="settings-style-options"
              role="group"
              aria-label="Interface Style"
              aria-busy={isSavingInterfaceStyle}
            >
              {(["spatial", "brutalist"] as const).map((style) => (
                <button
                  key={style}
                  type="button"
                  aria-pressed={interfaceStyle === style}
                  disabled={isSavingInterfaceStyle}
                  onClick={() => void onSelectInterfaceStyle(style)}
                >
                  <span className="settings-choice-dot" aria-hidden="true" />
                  <span>
                    <strong>
                      {style === "spatial" ? "Spatial" : "Brutalist"}
                    </strong>
                  </span>
                  {interfaceStyle === style && (
                    <span className="settings-choice-status" aria-hidden="true">
                      <Check size={15} />
                      Selected
                    </span>
                  )}
                </button>
              ))}
            </div>
            {isSavingInterfaceStyle && (
              <p role="status">Saving Interface Style…</p>
            )}
            {interfaceStyleError && <p role="alert">{interfaceStyleError}</p>}
          </section>
          <section
            className="settings-section settings-close"
            aria-labelledby="settings-close-heading"
          >
            <h2 id="settings-close-heading">Window Close Behavior</h2>
            <select
              id="settings-close-behavior"
              aria-labelledby="settings-close-heading"
              value={closeBehavior}
              disabled={savingClose || !settings}
              onChange={(event) =>
                void saveCloseBehavior(
                  event.target.value as "minimize_to_tray" | "quit_app",
                )
              }
            >
              <option value="minimize_to_tray">
                Keep playing in the background
              </option>
              <option value="quit_app">Quit Kaze</option>
            </select>
            <p className="settings-hint">
              {closeBehavior === "minimize_to_tray"
                ? "Kaze stays in the system tray. If the tray is unavailable, Kaze quits."
                : "Stops playback and closes the app."}
            </p>
            {savingClose && <p role="status">Saving close behavior…</p>}
            {closeError && <p role="alert">{closeError}</p>}
          </section>
          <StartupSettings />
          <MetadataSettings />
          <EntityFeedbackSettings />
          <section
            className="settings-links"
            aria-labelledby="settings-library-heading"
          >
            <h2 id="settings-library-heading">Library &amp; storage</h2>
            <button type="button" className="settings-section-link" onClick={() => openSection("taste")}>
              <span><strong>Music taste</strong><span>Choose your favorite artists and genres</span></span><span className="settings-open-label">Manage <ArrowRight size={16} /></span>
            </button>
            <button
              type="button"
              className="settings-section-link"
              onClick={() => openSection("folders")}
            >
              <span className="settings-section-icon" aria-hidden="true">
                <Folder size={22} />
              </span>
              <span>
                <strong>Music Folders</strong>
                <span>
                  {configuredFolders.length}{" "}
                  {configuredFolders.length === 1 ? "folder" : "folders"} in
                  your library
                </span>
              </span>
              <span className="settings-open-label" aria-hidden="true">
                Manage
                <ArrowRight size={16} />
              </span>
            </button>
            {/* Kaze's own folder: downloads live here, separate from your music folders. */}
            <div className="settings-section-link" role="group" aria-label="Download folder">
              <span className="settings-section-icon" aria-hidden="true">
                <Download size={22} />
              </span>
              <span>
                <strong>Download folder</strong>
                <span title={settings?.downloads?.resolved_dir}>
                  {settings?.downloads?.resolved_dir || "~/Music/Kaze"}
                </span>
              </span>
            </div>
            <button
              type="button"
              className="settings-section-link"
              onClick={() => openSection("system")}
            >
              <span className="settings-section-icon" aria-hidden="true">
                <Database size={22} />
              </span>
              <span>
                <strong>System &amp; Storage</strong>
                <span>Manage temporary audio</span>
              </span>
              <span className="settings-open-label" aria-hidden="true">
                Manage
                <ArrowRight size={16} />
              </span>
            </button>
          </section>
          <section className="settings-account" aria-label="Account">
            <div>
              <h2>Account</h2>
              <p>
                {currentUser
                  ? currentUser.display_name || currentUser.username
                  : "You're listening as a guest."}
              </p>
            </div>
            {currentUser ? (
              <button
                type="button"
                className="settings-logout"
                ref={logoutTrigger}
                onClick={() => {
                  setLogoutError(null);
                  setLogoutChoiceOpen(true);
                }}
                disabled={loggingOut}
              >
                <LogOut size={17} />
                {loggingOut ? "Logging out…" : "Log Out"}
              </button>
            ) : (
              <button
                type="button"
                className="settings-action"
                onClick={onOpenAuthModal}
              >
                Sign In
              </button>
            )}
            {currentUser && logoutChoiceOpen && (
              <dialog
                ref={logoutDialog}
                className="settings-logout-dialog"
                tabIndex={-1}
                aria-modal="true"
                aria-labelledby="logout-choice-heading"
                aria-busy={loggingOut}
                onKeyDown={(event) => {
                  if (event.key !== "Tab") return;
                  const buttons = Array.from(
                    event.currentTarget.querySelectorAll<HTMLButtonElement>(
                      "button:not(:disabled)",
                    ),
                  );
                  const first = buttons[0],
                    last = buttons[buttons.length - 1];
                  if (!first) {
                    event.preventDefault();
                    event.currentTarget.focus();
                  } else if (
                    event.shiftKey &&
                    document.activeElement === first
                  ) {
                    event.preventDefault();
                    last?.focus();
                  } else if (
                    !event.shiftKey &&
                    document.activeElement === last
                  ) {
                    event.preventDefault();
                    first.focus();
                  }
                }}
                onCancel={(event) => {
                  event.preventDefault();
                  if (!logoutRequest.current) setLogoutChoiceOpen(false);
                }}
              >
                <h2 id="logout-choice-heading">
                  Where do you want to log out?
                </h2>
                <div className="settings-logout-options">
                  <button
                    type="button"
                    disabled={loggingOut}
                    onClick={() => void logOut("device")}
                  >
                    <strong>This device</strong>
                    <span>Keep other devices signed in.</span>
                  </button>
                  <button
                    type="button"
                    disabled={loggingOut}
                    onClick={() => void logOut("all")}
                  >
                    <strong>All devices</strong>
                    <span>
                      Log out everywhere, including this device. Requires a
                      connection.
                    </span>
                  </button>
                </div>
                {loggingOut && <p role="status">Logging out…</p>}
                {logoutError && <p role="alert">{logoutError}</p>}
                <button
                  type="button"
                  className="settings-action settings-logout-cancel"
                  disabled={loggingOut}
                  onClick={() => setLogoutChoiceOpen(false)}
                >
                  Cancel
                </button>
              </dialog>
            )}
          </section>
        </div>
      ) : section === "taste" ? (<MusicTaste userId={currentUser?.id || "default"} key={currentUser?.id || "default"} onSaved={() => { onRecommendationSetupSaved?.(); openSection("main"); }} />) : section === "folders" ? (
        <FolderSettings
          configuredFolders={configuredFolders}
          isScanning={isScanning}
          onAddFolder={onAddFolder}
          onRemoveFolder={onRemoveFolder}
          onRescanLibrary={onRescanLibrary}
        />
      ) : (
        <StorageSettings />
      )}

    </div>
  );
});
