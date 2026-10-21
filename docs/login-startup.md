# Start in tray on login

Settings → Startup → **Start in tray on login** controls whether Kaze launches
in the tray when the current user signs in. It defaults off. Registration is
read from the operating system each time the Settings control mounts; it is
not a cloud or SQLite preference. The switch confirms writes before updating,
blocks overlapping saves and displays read/write errors with retry support.

Login registration passes `--autostart`. The main window is created invisible
for that argument, so there is no initial window flash. Tray Open restores the
existing webview. On Linux, restoration must show the entire GTK widget tree
(`show_all`), including the container around the webview. Showing only the outer
GtkWindow leaves an initially hidden login window blank. Keep the existing
webview and Wayland activation-token handling; do not reload or rebuild the page
to make it paint. An ordinary manual launch always opens/restores the window,
including when login startup is enabled. A second login launch exits through
the existing single-instance plugin without foregrounding the primary process.

If tray creation fails or Linux has no registered StatusNotifier host (bounded
two-second check), login startup shows the window instead. Playback restoration
retains its existing paused behavior; login startup never starts music itself.
No new playback subscriptions, root state, timers or browser audio are added.

## Platform registration

- Linux writes `${XDG_CONFIG_HOME:-~/.config}/autostart/org.intelligentmusicplayer.app.desktop`
  using the existing `directories::BaseDirs` path resolver. It quotes/escapes the
  executable, registers the original AppImage path when applicable, atomically
  replaces the entry, and removes it when switched off. `Hidden=true` and
  `X-GNOME-Autostart-enabled=false` are treated as disabled. The desktop session
  must process XDG autostart entries; Niri's installed session service requests
  `xdg-desktop-autostart.target`.
- Windows/macOS use Tauri's autostart plugin with the same argument and the
  default macOS LaunchAgent strategy. Those OS runtimes were not tested here.

Use the installed executable when enabling startup; a moved/deleted portable
file or development executable must be registered again from its new location.
Testing uses disposable XDG directories and never enables the live profile.

Linux uses an explicit implementation because `auto-launch` 0.6.0, beneath
Tauri autostart 2.7.0, hardcodes `~/.config` and emits an unquoted Linux Exec.
See the [XDG autostart specification](https://specifications.freedesktop.org/autostart/latest/),
[Exec quoting rules](https://specifications.freedesktop.org/desktop-entry/latest/exec-variables.html)
and [Tauri plugin documentation](https://v2.tauri.app/plugin/autostart/).

## Verification

```sh
npm run build
node --test --test-isolation=none tests/*.test.mjs
cargo test --manifest-path src-tauri/Cargo.toml --bin kaze startup --offline
cargo build --manifest-path src-tauri/Cargo.toml --bin kaze --offline --locked --features tauri/custom-protocol
KAZE_PRIVATE_AUTOSTART_TEST=1 dbus-run-session -- python3 scripts/test-autostart-native.py target/debug/kaze
KAZE_PRIVATE_AUTOSTART_TEST=1 dbus-run-session -- python3 scripts/test-autostart-rendering-native.py target/debug/kaze
```

The native regression requires Niri, PyGObject/Gio/Soup, desktop-file-validate
and a usable AppIndicator library. It exercises actual UI saves in both themes,
filesystem failure/retry, a quoted executable path, persisted registration,
hidden cold login, existing-window tray restore, quiet duplicate login,
normal-launch restoration, no-host fallback and absence of playback history.
A real logout/login and Windows/macOS behavior remain manual acceptance checks.

The rendering regression also requires Pillow. It captures only the disposable
process's own window through Niri and checks actual painted pixels before any
inspector connection. Both themes cover first Open from the tray, manual
second-instance launch, repeated hide/restore and the no-host fallback. A normal
launch is the rendering control, and test profiles are removed afterward;
screenshots/logs remain under the printed `/tmp/kaze-autostart-rendering-*` path.

The v1.0.9 regression was reproduced with the installed release: normal startup
painted correctly, while both themes' first and subsequent tray Open produced
uniform blank windows. The original native test's DOM/IPC assertions could all
pass despite invisible GTK ancestors. They cannot replace a real pixel check.
