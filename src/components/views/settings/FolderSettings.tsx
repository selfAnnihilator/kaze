import { useState, type FormEvent } from "react";
import { Folder, FolderPlus, RefreshCw, Trash2 } from "lucide-react";
import { WindowedList } from "../../../presentation/WindowedList";

export type MusicFolder = { id: string; path: string; track_count?: number };

/** Settings → Music Folders: the configured folders, add/remove, and rescan. */
export function FolderSettings({ configuredFolders, isScanning, onAddFolder, onRemoveFolder, onRescanLibrary }: {
  configuredFolders: MusicFolder[];
  isScanning: boolean;
  onAddFolder: (path: string) => Promise<void>;
  onRemoveFolder: (folderId: string) => Promise<void>;
  onRescanLibrary: () => Promise<void>;
}) {
  const [newFolderPath, setNewFolderPath] = useState("");
  const [folderBusy, setFolderBusy] = useState<string | null>(null);
  const [folderError, setFolderError] = useState<string | null>(null);
  const folderAction = async (key: string, action: () => Promise<void>, message: string) => {
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
          void folderAction(folder.id, () => onRemoveFolder(folder.id), "Couldn't remove this folder. Please try again.")
        }
      >
        <Trash2 size={17} />
      </button>
    </div>
  );
  return (
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
  );
}
