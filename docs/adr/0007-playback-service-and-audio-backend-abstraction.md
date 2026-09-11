# ADR 0007: Playback Service and Audio Backend Abstraction

## Context
Audio playback in a desktop player requires hardware access (ALSA, PulseAudio, PipeWire, WASAPI, CoreAudio), real-time buffer streaming, seek capabilities, volume scaling, queue manipulation (shuffle, repeat), and continuous position updates. Directly exposing `rodio` or `cpal` types throughout the application creates tight coupling, makes headless testing difficult in environments without sound hardware, and leaks audio driver platform quirks into domain layers.

## Options Considered
1. **Direct Rodio Usage in UI/Core**: Use `rodio::Sink` and `rodio::OutputStream` directly in command handlers. This tightly couples the whole codebase to `rodio`, causes thread safety issues with platform stream handles (`!Send` on ALSA), and prevents automated headless testing.
2. **External Audio Subprocess / Daemon**: Run an external media player daemon (e.g. mpv or a separate process) over IPC. Introduces significant latency, packaging overhead, and process crash recovery complexity.
3. **AudioBackend Trait + Thread-Safe Playback Service**:
   - Define a pure abstraction trait: `AudioBackend` with `load_and_play`, `pause`, `resume`, `stop`, `seek`, `set_volume`, `position`, `is_paused`, and `is_finished`.
   - Implement `RodioAudioBackend` for native cross-platform audio using `rodio` and `cpal`.
   - Implement `MockAudioBackend` for deterministic, soundcard-independent unit and integration testing.
   - Wrap queue state in `PlaybackQueue` with full support for sequential playback, `RepeatMode` (`Off`, `One`, `All`), and reversible `shuffle`.
   - Run a dedicated Tokio timer loop in `PlaybackService` emitting `PlaybackPositionChanged` every 250ms and detecting track completion to trigger automatic track progression.

## Decision
Adopt the **`AudioBackend` Trait** with a dedicated **`PlaybackService`**, **`PlaybackQueue`**, and **`MockAudioBackend`**.

## Reasoning
* Isolation: Audio hardware primitives never leak outside `src/playback/`.
* Testability: The entire playback state machine, queue transitions, seek operations, volume changes, repeat modes, and auto-advance logic can be tested deterministically in headless CI without sound hardware.
* Responsiveness: Periodic position updates are throttled to 250ms, delivering smooth frontend progress bars while avoiding IPC channel saturation.

## Consequences
* New audio engines (e.g. `kira` or direct `cpal`/PipeWire native integrations) can replace `RodioAudioBackend` by implementing `AudioBackend` with zero changes to `PlaybackService` or `CoreProcessor`.
