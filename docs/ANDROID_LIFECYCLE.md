# Android playback lifetime

Pressing Home or locking the screen keeps playback running in the media
foreground service. Fully dismissing Kaze from Recent apps, or finishing its
Activity, stops playback through the shared Rust `Stop` command and removes
the native player notification.

`KazeMediaPlugin` reconnects the media bridge on load/resume and closes it when
the Activity is finishing, excluding configuration changes. `PlaybackService`
also handles task removal. Closing is idempotent and blocks late playback
updates until Kaze opens again, so a queued event cannot recreate a dismissed
notification.

An empty/stopped playback snapshot releases the session, player, audio focus,
wake lock, and foreground notification before stopping the service. A paused
track can remain controllable while Kaze is open. A system request that starts
the service without a current track shuts it down again; `START_NOT_STICKY`
prevents Android from restoring an empty service after process death. External
controllers can send Stop through `ForwardingPlayer` to the same Rust handler.
Native cleanup still runs if an already-closed command bridge rejects Stop;
that failure is logged rather than leaving an active media session behind.

## Regression fixture

`tests/fixtures/androidMediaQa` builds a separate application,
`org.intelligentmusicplayer.lifecycleqa`, using the real Kaze media plugin and
service. It mirrors a silent synthetic track and replaces only the Rust command
sink. It never loads the main app's library, credentials, history, or audio.
The fixture invokes the production plugin's Activity lifecycle callbacks.

With the Android SDK, initialized Tauri Android project, and a connected test
phone available, run from the repository root:

```sh
rtk proxy env ANDROID_HOME=/opt/android-sdk src-tauri/gen/android/gradlew -p tests/fixtures/androidMediaQa :app:assembleDebug :tauri-plugin-kaze-media:testDebugUnitTest --no-daemon
rtk proxy env ANDROID_HOME=/opt/android-sdk python3 scripts/test-android-media-lifecycle.py
```

Set `ANDROID_SERIAL` if multiple devices are attached. The driver validates
the APK's package name before installing, only operates on the QA package, and
uninstalls it in cleanup. Results go to the fixture's ignored
`build/lifecycle-results.json`, or an explicit `--output` path.

Checks cover terminal playback, Home, pause/dismissal, playing task dismissal,
Activity finish, late events, an unavailable bridge, immediate stop/play,
external Stop, empty service startup, and reopening. The assertions inspect
active service/session/notification state. Notification history in a full
`dumpsys notification` dump is not evidence of an active notification;
`cmd notification list` provides the active keys used by this test.

The real phone investigation found an active Kaze session in playback state
`NONE`. The original service reproduced that empty-session leak and retained
a foreground session after playing task dismissal in the isolated fixture.
See the [lifecycle audit](audits/2026-11-26-android-lifecycle.md).
