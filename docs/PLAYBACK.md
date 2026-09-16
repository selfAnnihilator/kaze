# Playback Subsystem Architecture & Audio Engine Specification

## 1. Overview

The Playback Service provides low-latency, glitch-free audio playback decoupled from the frontend interface. The application can run headlessly via the backend alone.

---

## 2. Audio Backend Abstraction

To avoid locking the architecture into a single audio crate and to facilitate headless unit testing, audio hardware interaction is isolated behind the `AudioBackend` trait:

```rust
pub trait AudioBackend: Send + Sync {
    fn load_and_play(&mut self, file_path: &Path) -> AppResult<()>;
    fn pause(&mut self) -> AppResult<()>;
    fn resume(&mut self) -> AppResult<()>;
    fn stop(&mut self) -> AppResult<()>;
    fn seek(&mut self, position: Duration) -> AppResult<()>;
    fn set_volume(&mut self, volume: f32) -> AppResult<()>; // 0.0 to 1.0
    fn position(&self) -> Duration;
    fn is_paused(&self) -> bool;
    fn is_finished(&self) -> bool;
}
```

### 2.1 Concrete Implementations
* **`RodioAudioBackend`**: High-level cross-platform audio rendering using `rodio::OutputStream` and `rodio::Sink`. Runs the real-time audio thread safely.
* **`MockAudioBackend`**: Deterministic in-memory backend used in headless tests and CI environments where physical soundcards or audio daemons (ALSA/PulseAudio/PipeWire) may be unavailable.

---

## 3. Playback Queue Management

The playback queue is maintained by `PlaybackQueue`:
* **Items**: Vector of tracks with unique queue instance IDs.
* **Current Index**: Optional pointer to the currently playing track.
* **Original vs Shuffled State**: When shuffle is toggled, an indexed permutation map preserves the original order so disabling shuffle restores the natural sequence.
* **Repeat Modes**:
  - `RepeatMode::Off`: Sequence stops after the final track in the queue finishes.
  - `RepeatMode::One`: The current track repeats indefinitely until manually changed.
  - `RepeatMode::All`: Once the last queue track finishes, playback wraps around to the beginning.
* **Queue Mutations**:
  - `enqueue(track, play_next = true)`: Inserts track immediately following the current song.
  - `enqueue(track, play_next = false)`: Appends track to the end of the queue.
  - `remove(index)`: Removes track by position.
  - `clear()`: Clears the upcoming queue while leaving current song intact or stopping.

---

## 4. State & Event Lifecycle

```text
[ CoreProcessor ] ── Command::PlayTrack ──> [ PlaybackService ]
                                                    │
                                   ┌────────────────┴────────────────┐
                                   ▼                                 ▼
                         [ AudioBackend ]                    [ PlaybackQueue ]
                           (load file)                       (update index)
                                   │                                 │
                                   └────────────────┬────────────────┘
                                                    ▼
                                            Emits onto EventBus:
                                          - PlaybackStarted
                                          - QueueUpdated
                                          - PlaybackPositionChanged (periodic)
                                          - TrackFinished (on completion)
```

### Periodic Position Ticks
When audio is actively playing, a lightweight Tokio timer emits `PlaybackPositionChanged { position_secs, duration_secs }` at a throttled 250ms cadence to keep frontend progress bars smooth without flooding the IPC channel.

---

## 5. Unified Remote Audio Playback & Hardened Bounded Cache

Kaze utilizes a unified playback pipeline where local library tracks, downloaded files, and remote streams/previews all converge through the backend `PlaybackService`:

```text
Local file ────────┐
                   │
Remote stream ─────┼──> Rust PlaybackService ──> RodioAudioBackend
                   │
Cached audio ──────┘
                        │
                        ▼
            One queue, one state machine,
            one history path, one stats path,
            one backend event stream
```

### 5.1 Remote Audio Cache Invariants & Bounds
- **Dedicated Location**: Strictly bounded to `<app-cache>/remote-audio/` (legacy `stream_cache` automatically migrated on startup).
- **Hard Size Limit**: Default 1 GiB (`DEFAULT_MAX_CACHE_BYTES = 1,073,741,824`).
- **Eviction Target**: Auto-evicts down to ~900 MiB (`DEFAULT_EVICTION_TARGET_BYTES = 943,718,400`) when the limit is exceeded.
- **Max Single File**: 500 MiB limit (`DEFAULT_MAX_SINGLE_FILE_BYTES = 524,288,000`), rejecting oversized files mid-stream to prevent disk exhaustion.
- **Eviction Policy**: True LRU eviction sorted by last-modified time (`mtime`), updated on every cache hit via `std::fs::File::set_times`.
- **Active Playback Protection**: The currently playing audio file is strictly protected from LRU eviction even if it is the oldest file on disk.
- **Active Download Protection**: Actively downloading files and their `.part` counterparts are registered in an in-memory set and protected from eviction.
- **Cancellation & Failure Safety**: Downloads stream into `<sha256>.audio.part` guarded by an RAII drop guard (`PartFileCleanupGuard`). If a task is aborted, skips, or fails, the `.part` file is immediately deleted.
- **Startup Orphan Cleanup**: On boot, `startup_maintenance()` removes orphaned `.part` files older than 24 hours.
- **In-Flight Deduplication**: Concurrent requests for the exact same track cache key share a single download task via `tokio::sync::Notify` instead of redundant parallel downloads.
- **Strict Boundary Safety**: All cache file reads, writes, and evictions strictly verify `path.parent() == Some(&self.cache_dir)`. The engine never modifies or evicts files outside the designated cache directory.
- **IPC Observability & Management**:
  - `Command::ClearRemoteAudioCache`: Safely removes all un-protected cached audio, returning `bytes_freed` and `files_removed`.
  - `Query::GetRemoteAudioCacheStats`: Returns `total_size_bytes`, `file_count`, `max_size_bytes`, and `partial_file_count`.

