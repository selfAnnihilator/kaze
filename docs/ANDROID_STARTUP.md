# Phone startup recovery

The phone entrypoint reads persisted settings before mounting App. A transient
failure of that first read used to show “Couldn’t open Kaze” immediately, even
when pressing Try again successfully opened the app.

`src/services/startupSettings.ts` now gives phone startup up to four attempts,
waiting 250, 500, then 1,000 ms between failures. It keeps the opening screen
visible and only mounts App with a validated settings response. Persistent
failure still reaches the existing manual retry surface. Desktop startup keeps
its existing single-attempt behavior and persisted-theme introduction.

The upstream cause of the intermittent failure on the user's phone is not
confirmed: the observed phone launch succeeded, and the release did not expose
the failing JavaScript error in the available logs. The confirmed defect is
that a single transient startup-settings failure required manual recovery.

Verification uses the real entrypoint with a synthetic unavailable bridge in
`tests/mobileStartupRecovery.test.mjs`, and real browser rendering with fresh
profiles in `scripts/test-mobile-startup-recovery.mjs`. Cases cover automatic
recovery, bounded exhaustion, working manual retry, and unchanged desktop
behavior. No app account or real profile is used.
