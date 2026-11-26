# Android in-app updates

When the user taps UPDATE, Kaze offers an optional **Restart after updating?**
dialog before downloading the APK. It only appears when Display over other
apps is unavailable and Kaze has no remembered answer.

- **OK** remembers the answer and opens Android's Display over other apps
  settings. Select Kaze, enable access, and return to Kaze to continue.
- **UPDATE WITHOUT RESTART**, Back, or tapping outside the dialog remembers
  the refusal and continues the update immediately.
- If access is already granted or a previous answer is remembered, no dialog
  appears. Revoking previously granted access does not reset a remembered
  answer.

Returning from Settings continues the update whether or not access was
granted. If the settings activity is unavailable, updating still proceeds.
The existing `kaze_update.overlay_asked` preference is retained, including
answers recorded by older builds. Android exposes the current overlay grant;
Kaze's preference supplies the history needed to avoid repeated requests.

Kaze then downloads the APK, verifies its SHA-256, and installs it through
PackageInstaller. Installation permission remains a separate Android setting.
The replacement app restarts only if Display over other apps is granted;
otherwise the app closes during installation and the user opens it manually.

Android 11 and later open the general overlay settings list rather than
honoring a package-specific destination, so the dialog tells the user to
select Kaze. See [Android's permission changes](https://developer.android.com/about/versions/11/privacy/permissions#system-alert-window)
and the [Settings API](https://developer.android.com/reference/android/provider/Settings#canDrawOverlays(android.content.Context)).

Implementation: `platform/android_update.rs` calls the kaze-media
`prepareUpdate` command before downloading. `UpdateRestartPrompt.kt` renders
the native dialog; `UpdateRestartChoice.kt` records the choice before routing
it. `overlayAnswered` resumes the pending Rust operation. `UpdateRelaunch.kt`
retains the permission check when the replacement app starts.

Regression coverage: `UpdateRestartChoiceTest.kt` exercises permission states,
remembered refusal, approval ordering, and duplicate answers without an app
profile. `tests/androidUpdatePermission.test.mjs` checks native bridge wiring,
dialog actions, and the pre-download ordering.
