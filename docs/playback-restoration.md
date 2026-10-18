# Playback restoration

Kaze remembers the last song, position, full queue (including Previous history and shuffle order), volume/mute and repeat settings on this device. On reopening, both player presentations and Linux MPRIS show that song **paused**, even if it was playing when Kaze quit. This follows the user's preference recorded on 2026-10-18. Audio resumes only when Play is pressed.

Closing to the tray continues using the existing running player. This feature applies to a new process after Quit, quit-on-close or an application restart.

## Ownership and storage

- Rust `PlaybackService` remains the only playback authority. No browser audio or frontend persistence loop is added.
- `application_settings.playback_session_v1` stores a versioned local JSON bookmark. It contains the queue and controls, not credentials, expiring media URLs, audio bytes or a listening-session token. No schema migration or cloud sync is needed.
- The existing backend monitor checkpoints at most every ten seconds, skipping unchanged values and unresolved transitions. Clean shutdown takes a final snapshot before Stop resets the backend. Forced termination can lose up to the last checkpoint interval.
- Startup initializes the bookmark once before native controllers/frontend queries. Commands wait for that same initialization, preventing a late restore from overwriting a new selection. Invalid/unsupported bookmarks fall back to an empty player.
- A restored track is an unloaded paused selection. Startup does not open media or download online audio. Seeking before Play edits its saved starting position. Quit/reopen without playing preserves it.
- Play uses the normal Rust source-resolution/generation path, seeks to the saved offset, and creates a new listening session. The seek offset is never counted as listening time. Next/Previous or another selection drops that offset.
- A cached online song resumes directly. If the online cache was removed or incomplete, Play may buffer the full source first so seeking is available. Failure leaves the paused bookmark available for retry; a newer selection supersedes the old request. Quit signals the existing playback shutdown flag before draining accepted commands, so this wait cancels promptly and the monitor cannot advance the queue during shutdown. Normal non-restored streaming stays progressive.
- Ordinary MPRIS paused audio keeps its immediate control response. Cold-bookmark Play/PlayPause use owned async command tasks so a restored online load does not block the desktop controller's reply. Metadata and Paused state are available immediately after cold startup.
- Clear Queue, including the existing logout cleanup path, saves the cleared selection. A bookmark storage failure during shutdown is reported after audio/history cleanup; it must not prevent quitting.

## Verification

Backend integration coverage in `src-tauri/tests/playback_restore_tests.rs` exercises cold relaunch, exact saved position, queue/settings, no startup audio/history, explicit Play and Next, repeated paused relaunch/seek, online/missing media without startup network access, Clear Queue, invalid data, failed writes and the periodic checkpoint. The existing playback, Flow, history, lifecycle, progressive streaming and MPRIS tests remain relevant.

Native verification uses a private D-Bus session, temporary XDG data and silent WAV audio. It checks Spatial and Brutalist cold launches, actual player/MPRIS metadata, a stable paused position, no listening session on startup and status-bar Play resuming cached online audio at the saved offset. Run native GUI suites sequentially:

```sh
cargo test --offline --manifest-path src-tauri/Cargo.toml --test playback_restore_tests
cargo build --offline --manifest-path src-tauri/Cargo.toml --features tauri/custom-protocol --bin kaze
KAZE_PRIVATE_RESTORE_TEST=1 dbus-run-session -- python3 scripts/test-playback-restore-native.py /absolute/path/to/embedded/kaze
```

The native script does not open the user's library, change an account or interrupt the live player. Use isolated writable XDG directories for Rust tests too. No new startup auto-play setting is exposed: the requested behavior is always restore paused.

Verified on 2026-10-18: 89 backend tests pass across the library/MPRIS, playback restoration, playback, Flow, history, lifecycle and progressive-stream targets; one pre-existing opt-in private-bus test remains ignored. All 42 frontend regressions pass. The final embedded build passes all three native groups: Spatial and Brutalist cold startup with advancing focused frames, correct player/MPRIS metadata and a stable paused position without history, then MPRIS Play resuming actual cached online audio at the saved offset. The new backend test also streams delayed audio from loopback, verifies no source starts before the saved offset can be sought, and verifies Quit cancels that load within one second. Other desktop OS runtimes and a real remote-provider failure were not exercised. No installed application, live account or playlist was modified by the verification.
