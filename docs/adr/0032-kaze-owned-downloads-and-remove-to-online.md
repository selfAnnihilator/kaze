# Kaze-owned download folder and remove-to-online

Downloads used to land among the user's own music. Their ✓ state was guessed from fuzzy local matches, and nothing could be undone. Users couldn't tell their own library from Kaze's downloads, and a wrong match showed a ✓ on songs that were never downloaded.

Decision:

| Area | Rule |
| --- | --- |
| Ownership | Kaze writes downloads only to its own folder, `<Music>/Kaze` (`config.downloads.download_dir` overrides it). Files outside that folder are read-only to Kaze: they are never moved or deleted (`ensure_owned_files`). |
| Separation | Local versus Downloads comes from the file path prefix (`LibraryScope`). There is no new column or migration, so older versions keep working on the same profile. |
| State | ✓ comes only from exact ids: task source ids, online aliases and the saved file's library row. The folder watcher keeps the state live (`DownloadsChanged`). |
| Remove | Removing a download turns the song back into its online recording. Before the local row and the file are deleted, likes, playlist entries, history, recommendation feedback, Flow slots and statistics are re-keyed to the online id (`release_to_online`, `UPDATE OR IGNORE` against the playlist uniqueness trigger). Queued copies are replaced. |
| Confirmation | Removal always asks first, through a store-driven popup (`confirmDialog.ts`), never through App state. |

Alternatives:
- A `source` column on `tracks`. This was rejected: it needs a migration, and older versions would refuse the profile (see ADR 0031).
- Deleting the song outright. This was rejected: it would lose likes, playlists and stats that the user built while it was downloaded.
- Keeping downloads among the user's music. This was rejected: Kaze couldn't safely delete or move anything there.

Consequences:
- If the user moves a file out of the Kaze folder, it becomes a local song.
- Tags carry the source id, so adoption after a reinstall restores the ✓ state.
- Statistics merge into the online song. Per-device rows are summed.

Tests: `src-tauri/tests/download_folder_tests.rs`
- Relocation keeps library rows.
- Adoption.
- Exact-id rows.
- A user file is never deleted.
- Remove moves likes, playlists, history and stats to the online song.

Frontend: `tests/downloadsView.test.mjs` and `tests/songControls.test.mjs`. Behaviour and modules: [DOWNLOADS.md](../DOWNLOADS.md).
