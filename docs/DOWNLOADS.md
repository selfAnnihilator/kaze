# Downloads

Kaze can save an online (JioSaavn) song for offline playback. Downloads are **Kaze-owned files** in their own folder, kept apart from the user's local library. Decisions: [ADR 0032](adr/0032-kaze-owned-downloads-and-remove-to-online.md). The old Soulseek design is archived in [archive/DOWNLOADS-soulseek.md](archive/DOWNLOADS-soulseek.md).

## Behaviour

| Area | What happens |
|---|---|
| Starting | The download button on an online song calls `startDownload` (`services/downloadActions.ts`). The backend resolves the stream, writes `.<task>.part` in the Kaze folder, tags it (title, artist, album, cover, source id) and renames it into place. The task becomes `COMPLETED` with its `destination_path`. |
| Folder | `<system Music folder>/Kaze`, for example `~/Music/Kaze`. `config.downloads.download_dir` overrides it. Settings shows the resolved path (`downloads.resolved_dir`). At startup, Kaze moves its own older downloads into the folder (`relocate_into_folder`). Library rows keep their ids, so history, likes and playlists stay attached. |
| Live state | A `notify` watcher on the folder (500 ms debounce; dot-files such as `.part` and `.moving` are ignored) runs `sync_folder`: it adopts tagged files with no record, marks records whose file is gone, and emits `DownloadsChanged`. The frontend re-fetches downloads, the library pages and playlist memberships. |
| ✓ state | Exact ids only. `GetDownloads` rows (`downloads/rows.rs`) carry every id that shows the song: online aliases and the saved file's library row. The UI never guesses by title or artist. |
| Local vs Downloads | Derived from paths, with no schema change. `LibraryScope::{Local, Downloads}` (`database/repositories/library_scope.rs`) filters by the Kaze folder prefix. The Library shows only the user's own folders; the Downloads view lists Kaze's songs. |
| Removing | Clicking a downloaded ✓ opens a Yes/No popup (`services/confirmDialog.ts`, `ConfirmModal`). On Yes, `RemoveDownload` runs. `ensure_owned_files` refuses any file outside the Kaze folder. `release_to_online` then moves likes, playlist entries, history and statistics from the local row to the online song, deletes the local row and the file, and replaces queued copies with the online song. The song keeps working as a stream. |
| Never | Kaze never moves or deletes files it didn't create, and never downloads without a user action. |

## Modules

| Concern | Location |
|---|---|
| Provider (search, stream to file, tagging) | `src-tauri/src/downloads/jiosaavn.rs`, `tags.rs`, `traits.rs` |
| Task lifecycle | `src-tauri/src/downloads/service.rs` |
| Folder: watch, relocate, adopt, reconcile | `src-tauri/src/downloads/service/folder.rs` |
| Removal | `src-tauri/src/downloads/service/removal.rs` |
| Rows for the UI | `src-tauri/src/downloads/rows.rs` |
| SQL (`relocate`, `release_to_online`) | `src-tauri/src/database/repositories/download_repo.rs` |
| Processor commands (`StartDownload`, `CancelDownload`, `RemoveDownload`, `GetDownloads`) | `src-tauri/src/core/processor/downloads.rs` |
| Frontend actions and state | `src/services/downloadActions.ts`, `src/services/downloadProgress.ts`, `src/hooks/useDownloadAction.ts` (`describeDownload`) |
| UI | `src/components/DownloadButton.tsx`, `src/components/views/DownloadsView.tsx`, `src/hooks/app/useDownloadsSection.ts` |

## Data

The `download_tasks` table: id, provider, title, artist, album, filename, `destination_path`, sizes, `status` (`QUEUED`, `DOWNLOADING`, `COMPLETED`, `FAILED`, `CANCELLED`, `MISSING`) and timestamps. The online source id is stored on the task and in the file's tags, which is how adoption works after a reinstall. No migration was added for any of the behaviour above (see [ADR 0031](adr/0031-additive-migrations-and-downgrade.md)).

## Tests

- `src-tauri/tests/download_folder_tests.rs`: folder ownership, relocation, adoption and removal back to online. It includes `a_file_outside_the_kaze_folder_is_never_deleted`.
- `src-tauri/tests/download_tests.rs`: the task lifecycle.
- `tests/downloadsView.test.mjs` and `tests/songControls.test.mjs`: exact-id ✓ state, scoped lists and the remove flow.
