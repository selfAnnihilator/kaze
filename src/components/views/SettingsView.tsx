import {
  memo,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type FormEvent,
} from "react";
import {
  ArrowLeft,
  ArrowRight,
  Check,
  Database,
  Folder,
  FolderPlus,
  LogOut,
  RefreshCw,
  Trash2,
} from "lucide-react";
import type {
  AppSettings,
  RemoteAudioCacheStats,
  UserProfile,
} from "../../types";
import { dispatchCommand, executeQuery } from "../../services/api";
import type { InterfaceStyle } from "../../presentation/InterfaceContext";
import { WindowedList } from "../../presentation/WindowedList";
import { useViewMemory } from "../../presentation/ViewMemory";
import { MetadataSettings } from "../MetadataSettings";
import { StartupSettings } from "../StartupSettings";

type Section = "main" | "folders" | "system";
type MusicFolder = { id: string; path: string; track_count?: number };
interface SettingsViewProps {
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
function cacheSnapshot(response: unknown): RemoteAudioCacheStats {
  const raw = response as {
    data?: RemoteAudioCacheStats;
    total_size_bytes?: number;
  } | null;
  const value = (raw?.data || raw) as RemoteAudioCacheStats | null;
  if (
    !value ||
    typeof value.total_size_bytes !== "number" ||
    typeof value.file_count !== "number"
  )
    throw new Error("Cache information unavailable");
  return value;
}
const size = (bytes: number) =>
  bytes >= 1024 ** 3
    ? `${(bytes / 1024 ** 3).toFixed(1)} GB`
    : `${(bytes / 1024 ** 2).toFixed(1)} MB`;

export const SettingsView = memo(function SettingsView({
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
  const [newFolderPath, setNewFolderPath] = useState("");
  const [folderBusy, setFolderBusy] = useState<string | null>(null);
  const [folderError, setFolderError] = useState<string | null>(null);
  const [closeBehavior, setCloseBehavior] = useState(
    settings?.close_button_behavior ?? "minimize_to_tray",
  );
  const [savingClose, setSavingClose] = useState(false);
  const [closeError, setCloseError] = useState<string | null>(null);
  const closeRequest = useRef(false);
  const [cache, setCache] = useState<RemoteAudioCacheStats | null>(null);
  const [cacheLoading, setCacheLoading] = useState(false);
  const [cacheError, setCacheError] = useState<string | null>(null);
  const [cacheMessage, setCacheMessage] = useState<string | null>(null);
  const [clearingCache, setClearingCache] = useState(false);
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
  useEffect(() => {
    if (section !== "system") return;
    let cancelled = false;
    setCacheLoading(true);
    setCache(null);
    setCacheError(null);
    executeQuery({ query: "GetRemoteAudioCacheStats" })
      .then((response) => {
        const value = cacheSnapshot(response);
        if (!cancelled) setCache(value);
      })
      .catch(() => {
        if (!cancelled)
          setCacheError(
            "Couldn't load storage usage. Reopen this section to try again.",
          );
      })
      .finally(() => {
        if (!cancelled) setCacheLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [section]);

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
  const folderAction = async (
    key: string,
    action: () => Promise<void>,
    message: string,
  ) => {
    if (folderBusy) return;
    setFolderBusy(key);
    setFolderError(null);
    try {
      await action();
    } catch {
      setFolderError(message);
    } finally {
      setFolderBusy(null);
    }
  };
  const addFolder = (event: FormEvent) => {
    event.preventDefault();
    if (!newFolderPath.trim()) return;
    void folderAction(
      "add",
      async () => {
        await onAddFolder(newFolderPath.trim());
        setNewFolderPath("");
      },
      "Couldn't add this folder. Check the path and try again.",
    );
  };
  const clearCache = async () => {
    if (clearingCache) return;
    setClearingCache(true);
    setCacheError(null);
    setCacheMessage(null);
    try {
      await dispatchCommand({ command: "ClearRemoteAudioCache" });
      setCacheMessage("Temporary audio cleared. Tracks in use stay available.");
      try {
        setCache(
          cacheSnapshot(
            await executeQuery({ query: "GetRemoteAudioCacheStats" }),
          ),
        );
      } catch {
        setCacheError(
          "Cache cleared, but usage couldn't refresh. Reopen this section to check it.",
        );
      }
    } catch {
      setCacheError("Couldn't clear temporary audio. Please try again.");
    } finally {
      setClearingCache(false);
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
  const folderRow = (folder: MusicFolder) => (
    <div className="settings-folder">
      <Folder size={20} aria-hidden="true" />
      <div>
        <strong title={folder.path}>{folder.path}</strong>
        <span>{folder.track_count ?? 0} tracks</span>
      </div>
      <button
        className="settings-icon-action"
        type="button"
        aria-label={`Remove folder ${folder.path}`}
        disabled={!!folderBusy}
        onClick={() =>
          void folderAction(
            folder.id,
            () => onRemoveFolder(folder.id),
            "Couldn't remove this folder. Please try again.",
          )
        }
      >
        <Trash2 size={17} />
      </button>
    </div>
  );
  const title =
    section === "main"
      ? "Settings"
      : section === "folders"
        ? "Music Folders"
        : "System & Storage";
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
          <section
            className="settings-links"
            aria-labelledby="settings-library-heading"
          >
            <h2 id="settings-library-heading">Library &amp; storage</h2>
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
      ) : section === "folders" ? (
        <section
          className="settings-detail"
          aria-label="Music Folders settings"
        >
          <div className="settings-detail-intro">
            <p>Choose where Kaze finds your music.</p>
            <button
              type="button"
              className="settings-action"
              disabled={isScanning || !!folderBusy}
              onClick={() =>
                void folderAction(
                  "scan",
                  onRescanLibrary,
                  "Couldn't scan your library. Please try again.",
                )
              }
            >
              <RefreshCw
                size={16}
                className={isScanning ? "spin-animation" : undefined}
              />
              {isScanning ? "Scanning…" : "Rescan Library"}
            </button>
          </div>
          {configuredFolders.length > 50 ? (
            <WindowedList
              items={configuredFolders}
              rowHeight={88}
              label="Music folders"
              renderItem={folderRow}
            />
          ) : (
            <div className="settings-folder-list">
              {configuredFolders.length ? (
                configuredFolders.map((folder) => (
                  <div key={folder.id}>{folderRow(folder)}</div>
                ))
              ) : (
                <p className="settings-hint">
                  Add a folder to start your library.
                </p>
              )}
            </div>
          )}
          <form onSubmit={addFolder} className="settings-folder-form">
            <label htmlFor="settings-folder-path">Folder path</label>
            <input
              id="settings-folder-path"
              value={newFolderPath}
              onChange={(event) => setNewFolderPath(event.target.value)}
              placeholder="/path/to/your/music"
              autoComplete="off"
            />
            <button
              type="submit"
              className="settings-action settings-action--primary"
              disabled={!!folderBusy || !newFolderPath.trim()}
            >
              <FolderPlus size={17} />
              {folderBusy === "add" ? "Adding…" : "Add Folder"}
            </button>
          </form>
          {folderError && <p role="alert">{folderError}</p>}
        </section>
      ) : (
        <section
          className="settings-detail"
          aria-label="System & Storage settings"
        >
          <div className="settings-storage-summary">
            <span className="settings-eyebrow">TEMPORARY AUDIO</span>
            <strong>
              {cacheLoading
                ? "Loading…"
                : cache
                  ? size(cache.total_size_bytes)
                  : "Unavailable"}
            </strong>
            {cache && (
              <p>
                {cache.file_count} cached{" "}
                {cache.file_count === 1 ? "track" : "tracks"}
                {cache.max_size_bytes > 0
                  ? ` · ${size(cache.max_size_bytes)} limit`
                  : ""}
              </p>
            )}
          </div>
          <div className="settings-storage-action">
            <div>
              <h2>Clear temporary audio</h2>
              <p>
                Free up space. Your music files and playlists stay in place.
              </p>
            </div>
            <button
              type="button"
              className="settings-action"
              disabled={clearingCache || cacheLoading}
              onClick={() => void clearCache()}
            >
              <Trash2 size={16} />
              {clearingCache ? "Clearing…" : "Clear Cache"}
            </button>
          </div>
          {cacheMessage && <p role="status">{cacheMessage}</p>}
          {cacheError && <p role="alert">{cacheError}</p>}
        </section>
      )}
    </div>
  );
});
