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
