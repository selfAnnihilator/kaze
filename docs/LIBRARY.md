# Local Music Library Architecture & Scanner Specification

## 1. Core Principles

The Library subsystem is responsible for indexing and maintaining the user's local music collection while upholding strict security, performance, and privacy standards:

1. **Default Music Directory with Onboarding Choice**:
   * On first launch, the player inspects the operating system's standard music path (e.g. `~/Music` via `directories::UserDirs::audio_dir()`).
   * During onboarding, the user is presented with this default directory and given the explicit choice to accept it or select any custom directory (or multiple directories).
   * No scanning occurs until the user confirms their music folders during onboarding.
2. **Strict Boundary Containment**:
   * The scanner traverses only the registered library folders and their child subdirectories.
   * Traversal strictly enforces root boundary containment:
     - Symlinks that resolve outside the registered library root are discarded and never followed.
     - Canonicalized file paths must start with the canonicalized library root folder.
     - The scanner will never scan the entire filesystem or access paths outside the user's explicitly approved directories.
3. **Incremental Scanning & Performance**:
   * The scanner records each file's size (`file_size`) and modified timestamp (`modified_timestamp`).
   * When rescanning, unchanged files (matching size and timestamp) are skipped immediately without re-parsing audio tags.
   * File hash (SHA-256) is computed optionally or when timestamp discrepancies occur.
4. **Resilience & Missing Files**:
   * Files deleted from disk are detected during scans and pruned or marked as missing in the database.
   * Corrupted or unreadable audio files emit non-fatal warnings and do not abort the overall scanning job.
5. **Real-time Filesystem Monitoring**:
   * Registered library roots are monitored using `notify` for real-time file creation, modification, and deletion events, triggering debounced incremental updates.

---

## 2. Supported Audio Formats

The player leverages `lofty` to extract comprehensive audio tags across standard audio formats:

| Format | Extensions | Metadata Container |
|---|---|---|
| **MP3** | `.mp3` | ID3v2.3, ID3v2.4, ID3v1 |
| **FLAC** | `.flac` | Vorbis Comments |
| **OGG / Vorbis** | `.ogg`, `.oga` | Vorbis Comments |
| **Opus** | `.opus` | Vorbis Comments |
| **M4A / AAC** | `.m4a`, `.aac`, `.mp4` | MP4 iTunes tags |
| **WAV** | `.wav` | RIFF INFO / ID3 chunk |

---

## 3. Extracted Tag Schema

For each discovered audio track, the following attributes are parsed:
* **Track Title** (falls back to filename if absent)
* **Artist** (falls back to "Unknown Artist")
* **Album** (falls back to "Unknown Album")
* **Album Artist**
* **Genre** (normalized)
* **Track Number** & **Disc Number**
* **Release Year / Date**
* **Duration** (in seconds with floating point precision)
* **Audio Properties**: Bitrate (kbps), Sample Rate (Hz), Format/Codec
* **Artwork Presence**: Boolean indicator if front cover art is embedded
* **MusicBrainz IDs**: Track ID, Release ID, Artist ID if present in tags

---

## 4. Full-Text Search (FTS5)

A virtual SQLite FTS5 table (`tracks_fts`) is updated in tandem with track insertions, updates, and deletions:
* Tokens indexed: `title`, `artist`, `album`, `genre`.
* Supports prefix queries (e.g. `Radioh*`), case-insensitive matching, and fast ranked text retrieval across large collections in <2ms.
