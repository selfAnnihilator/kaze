# ADR 0006: Strict Boundary Containment and Incremental Scanning

## Context
A desktop music player with access to the local filesystem must respect user privacy, prevent unauthorized directory traversal, and avoid degrading system performance. Scanning the entire filesystem or following arbitrary symlinks poses security and privacy risks. Additionally, re-parsing audio metadata on every scan wastes CPU, disk I/O, and battery.

## Options Considered
1. **Unrestricted / System-Wide Scanning**: Search common paths across the whole drive. This risks indexing sensitive files, reading irrelevant audio clips (game assets, system sounds), and high resource consumption.
2. **Follow Symlinks Unconditionally**: Follow any symlink found in configured folders. This risks directory loops and escaping the music folder into user home/root directories.
3. **Strict Boundary Containment + Incremental mtime/size Caching**:
   - Default to the operating system's audio folder (`~/Music` via OS standard paths).
   - Require explicit onboarding confirmation if the user chooses custom directories.
   - Strictly verify all traversed file and directory paths with canonical root boundary checks (`path.canonicalize().starts_with(&canonical_root)`).
   - Disable following symlinks (`follow_links(false)`).
   - Compare `file_size` and `modified_timestamp` against SQLite records; if unchanged, skip re-parsing metadata tags.
   - Automatically prune tracks deleted from disk during folder rescans.

## Decision
Adopt **Strict Boundary Containment** with **Incremental mtime/size Caching** and **Lofty Tag Extraction**.

## Reasoning
* Security and Privacy: Guarantees the application never reads or scans outside the user's explicitly approved music directory.
* Performance: Incremental rescanning takes milliseconds on collections with tens of thousands of tracks by skipping unchanged files.
* Clean Library State: Tracks deleted from disk are automatically pruned from the database and FTS5 search index.

## Consequences
* Files placed as symlinks pointing to drives or directories outside the library root will be safely ignored.
* If file modification timestamps are preserved during external edits, a full (non-incremental) rescan can be explicitly triggered by the user.
