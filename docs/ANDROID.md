# Android build and platform notes

Kaze builds for Android from the same Tauri 2 repository as desktop. The decisions behind
the split are in [ADR 0035](adr/0035-android-platform-split.md). This page covers how to
build and what is wired today.

## Prerequisites

| Tool | Version used | Notes |
| --- | --- | --- |
| JDK | 21 | Runs Gradle. The app module compiles Java with source/target 1.8. |
| Android SDK | Platform 37 (`compileSdk`/`targetSdk` 37) | Set `ANDROID_HOME`, for example `/opt/android-sdk`. |
| Android NDK | 28 (for example `28.2.13676358`) | Set `NDK_HOME` to the NDK directory. |
| Rust | Stable, with the target installed | `rustup target add aarch64-linux-android` |
| Node.js | Same as desktop (`npm ci` in the repo root) | Tauri CLI runs through `npm run tauri`. |

`minSdk` is 24. Only `aarch64` is built by the commands below.

## Cross-compile environment

The NDK clang wrappers must be visible to the C/C++ build scripts (`oboe-sys`, `ring`,
`libsqlite`) and to the Rust linker. With `NDK` pointing at the NDK prebuilt toolchain
(`$NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin`):

```bash
export NDK="$NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin"
export CC_aarch64_linux_android="$NDK/aarch64-linux-android24-clang"
export CXX_aarch64_linux_android="$NDK/aarch64-linux-android24-clang++"
export AR_aarch64_linux_android="$NDK/llvm-ar"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$NDK/aarch64-linux-android24-clang"
```

The API level (24) must match `minSdk`. These variables are not stored in the repository,
so set them in your shell or a local, uncommitted environment file.

## Build commands

Run from the repository root unless noted.

```bash
# Rust check for the mobile target (from src-tauri/, with the variables above set)
cd src-tauri && cargo check --target aarch64-linux-android

# Host tests for the Android-only bridge logic and the plugin crate
cd src-tauri && cargo test --lib platform::media_bridge
cargo test -p tauri-plugin-kaze-media

# Debug or release APK (Gradle project in src-tauri/gen/android)
ANDROID_HOME=/opt/android-sdk NDK_HOME=$NDK_HOME npm run tauri android build -- --apk --target aarch64
```

Unsigned release APK output:
`src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk`.

Check the packaged manifest after a build:

```bash
/opt/android-sdk/build-tools/36.0.0/aapt2 dump xmltree --file AndroidManifest.xml <apk>
```

Look for `READ_MEDIA_AUDIO`, `FOREGROUND_SERVICE_MEDIA_PLAYBACK`, `WAKE_LOCK`, and the
`PlaybackService` entry with `foregroundServiceType` `mediaPlayback`.

Environment workaround: in a worktree whose `node_modules` is a read-only shared symlink,
Vite cannot write `node_modules/.vite-temp`. The build succeeds with the runner config
loader, passed through the Tauri config override:

```bash
npm run tauri android build -- --apk --target aarch64 --ci \
  --config '{"build":{"beforeBuildCommand":"npm run build -- --configLoader runner"}}'
```

Do not commit this override. It is for read-only environments only.

Do not re-run `npm run tauri android init` on the committed project casually. It regenerates
`src-tauri/gen/android/` and can overwrite the manifest, Gradle, and resource changes.

## Architecture

In Library, Playlists lists created, imported and saved playlists. Generated Daily Mixes
are available from Smart Mixes and Discover. Liked Songs has its own tab.
Adding a like requires signing in. If signed out, tapping Like on an eligible song
shows a toast with the sign-in requirement.

Music Taste in onboarding and Settings supports multiple song languages and any
number of the offered genres and artists. For You and
Flow begin with the selected languages and include more as listening history
builds up. These choices are saved for the current profile on this device.
The artist picker is the final step. It starts with artists from the chosen
languages, then uses known similarities and collaborations to suggest new artists.
Returning account sign-in skips automatic Music Taste onboarding.

Discover's Top Charts has separate language feeds. Search uses song language
metadata for queries such as “Malayalam top hits” and shows relevant JioSaavn
playlists under All and Playlists when the catalog returns them. Opening a
playlist loads its songs by the provider's playlist ID.

```
src-tauri/src/lib.rs                run()  (#[mobile_entry_point] on Android)
src-tauri/src/app.rs               thin main() for the desktop binary `kaze`
src-tauri/src/platform/shared.rs   IPC commands, artwork protocol, async runtime (all targets)
src-tauri/src/platform/desktop.rs  tray, updater, single instance, MPRIS, lifecycle (desktop only)
src-tauri/src/platform/android.rs  backend start, event bridge, media commands (Android only)
src-tauri/src/platform/media_bridge.rs  event-to-session mapping (tested on the host)
plugins/kaze-media/                Rust plugin crate + Kotlin Gradle library
```

Playback path on Android:

1. Playback runs in Rust `PlaybackService`: rodio and symphonia decode, cpal writes to AAudio.
2. EventBus events reach `platform/android.rs`. `MediaMirror` folds them into the session state.
3. `kaze-media` sends the state to Kotlin `PlaybackService`. It updates the Media3 session
   (notification, lock screen, Bluetooth and headset controls), requests audio focus, and
   holds a partial wake lock while playing.
4. Transport commands from the session come back as `mediaCommand` events. Rust maps them
   with `command_for` and dispatches them into CoreProcessor.

Kotlin never decodes or outputs audio. There is no WebView `<audio>` element.

Core modules (CoreProcessor, SQLite, recommendations, scanner) are the same code as desktop.

## Desktop-only features on Android

These are compiled out on Android, so they do not run there:

- Tray icon, updater, single-instance lock, autostart, MPRIS, and the `rfd` file dialog.
- The Linux-specific WebKit tuning and window chrome.

## Status and gaps

Implemented and compiled, but not yet proven on a device:

- Android build (`aarch64`): compiles, and the APK packages the required manifest entries.
- Plugin registration from Rust and the Kotlin command surface (`updatePlayback`,
  `pickMusicFolder`, `secretSet`/`secretGet`/`secretDelete`).

Wired in Rust (runtime behaviour still unverified on a device):

- **Credentials**: `KeystoreStore` in `cloud/secret_store.rs` delegates to the `PluginSecrets`
  bridge that `platform/android.rs` installs at startup. It calls the Kotlin `SecretStore`
  (EncryptedSharedPreferences, AES-256 Keystore master key). If the bridge is not installed or
  a call fails, callers fall back to the app-private 0600 file.
- **Music folder**: the async command `android_pick_music_folder` in `platform/android.rs`
  calls the plugin `pick_music_folder` (SAF `ACTION_OPEN_DOCUMENT_TREE`) and returns the tree
  URI, or `None` when cancelled.
- **Audio focus**: `PlaybackService` requests focus when Rust plays. A refused request pauses
  Rust. A transient loss pauses and resumes on gain. A manual resume or a stop clears any pending
  resume and releases focus; paused snapshots such as seeks keep it. The decision logic is unit tested in `FocusPolicy`.

Not wired yet (verify before relying on them):

- **Music folder use**: no frontend caller invokes `android_pick_music_folder`, and the library
  scanner still takes filesystem paths, so a picked SAF URI is not scanned. The default music
  directory is the Android `Music` audio directory. Scanning under scoped storage is unverified.
- **Avatar picker**: `ProfileService::pick_avatar_file` returns `Ok(None)` on mobile.

Unverified on a device or emulator (manual check still required):

- Playback continues for at least 60 s with the screen locked or off.
- Lock-screen, notification, headset and Bluetooth controls pause and resume playback.
- Audio focus loss (calls, other apps) pauses and resumes correctly.
- Whether cpal and AAudio keep playing after lock without extra stream usage hints.

Not covered by CI: `.github/workflows/release.yml` builds and tests on desktop runners only.
An Android job is not added; the owner should decide whether to add one.

Not verified: macOS and Windows builds from this change.
