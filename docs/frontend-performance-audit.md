# Kaze frontend performance audit — 2026-10-16

The Spatial and Brutalist layouts, features and visual identity are preserved. This pass changes frontend update frequency, mounting, artwork request lifetime and visual activity. No production Rust, playback engine, Flow, MPRIS, tray, single-instance, dependency, commit or push change was made in this pass. Existing uncommitted integration changes remain in place.

## 1. Confirmed causes and hot paths

The audit inspected the existing diff, functional App, both presentation paths, shared controls, event subscription, timers/listeners/observers, artwork requests, virtualized renderers and active computed CSS. Controlled tests exercise the actual App rather than the sample design reducers.

- **Transfer ticks reached the entire application.** The backend download-progress handler mapped `downloads` into root state on every event. A stream of 100 events caused 100 App/Home renders, including disclosure lists, controls and dialogs, even on Home. This is the largest reproduced broad rendering problem.
- **Horizontal windowing tracked scroll pixels.** `WindowedStrip` put `scrollLeft` into React state on every scroll event and rebuilt its visible card children. The native shelf test recorded 151 commits in its sample, including normal playback-leaf commits.
- **Inactive Spatial chapters remained mounted.** Their hidden strips still owned observers and mounted artwork. This was inactive chapter work inside the selected theme; it was not two full theme trees mounted together.
- **Hidden playback visuals kept rendering.** Native WebKitGTK reported `document.visibilityState === "hidden"` after the real tray-hide path, yet the progress leaf committed 240 times in one hidden minute.
- **Late update subscription cleanup leaked a development listener.** A delayed `listen()` resolving after effect cleanup left its abandoned registration active. Reproduction against the original UpdateBanner source gave two listeners under StrictMode; the corrected source gave one, with no page errors.
- **Lyrics retained a short-lived timer after navigation.** A manual-scroll/resync reproduction left one ten-second timer queued after leaving the view. Unmount cleanup now cancels it.
- **Artwork requests were fragmented.** App, Home, stage/queue artwork and Albums could independently resolve the same local cover. Home sometimes requested a cover despite already receiving its URL; the old album cache retained entries without a bound.

A full application hang was **not reproduced**. These findings establish specific avoidable work, not a claim that every possible hang has been diagnosed.

## 2. Theme and rerender architecture

Only the active presentation branch mounts. The normal entry does not mount either reference lab, its reducers, animation loops or sample catalogue. Shared application state and one backend subscription stay mounted across theme changes.

Playback position already used the leaf emitter correctly. Before and after, 100 position events caused zero App, Home, sidebar or top-search renders; only the progress component updated. That architecture remains intact. Download progress now follows the same isolation principle: a keyed external store updates only subscribed, mounted `LiveDownloadTask` regions. Backend status/structural changes still refresh the authoritative task list.

Development StrictMode repeats initial effects and therefore initial queries; the native production frontend has one initial query path. StrictMode was retained. The asynchronous update-banner cleanup now disposes registrations even when they finish after unmount.

## 3. Scroll, animation and visual activity fixes

- Windowed strips update their React window only when crossing a sleeve boundary. Memoized sleeve wrappers retain unchanged cards. Table windows similarly track row boundaries instead of pixels.
- Mouse dragging writes native `scrollLeft` through refs. CSS smooth scrolling is temporarily suspended during a drag and restored afterward; arrow browsing keeps its existing behavior. The regression verifies an immediate 100px response to a 100px drag.
- Spatial mounts only its active chapter. Per-chapter horizontal positions live in refs and restore when returning to a chapter. The test verifies the 100px shelf position survives unmount/remount.
- The fixed-stage wheel gesture gate remains because chapters replace content in place rather than scrolling a document. It does not set state for every wheel pixel. Native overflow remains responsible for horizontal scrolling; no custom momentum/RAF loop was introduced.
- Progress leaves keep the latest emitter snapshot while hidden without committing visual updates, then synchronize on visibility restoration. Download ticks are similarly suppressed for hidden visual subscribers.
- One shared IntersectionObserver pauses offscreen spinner/equalizer targets. A class/child mutation observer registers and releases targets as bounded UI regions change. There are no per-frame layout reads or scroll measurements for this policy. Hidden-window CSS pauses animations and disables transitions; existing reduced-motion rules remain.
- Transfer bars and the toast expiration effect use `scaleX` instead of changing animated width. Their visible geometry and meaning are retained.
- Now Playing still derives stable sleeve identities/positions from the actual queue/current track. It has no continuous idle animation loop or per-frame card-position React state.

## 4. Images and lists

`coverArt.ts` shares concurrent requests, limits IPC reads to four, cancels queued work whose consumers leave, and retains at most 64 results / 8 MiB of cover strings. Scan completion invalidates retained covers. Components skip local resolution when they already have a usable URL. Critical artwork consumers use asynchronous decoding; offscreen shelf images use lazy loading where appropriate. No remote URL resizing scheme or new backend image API was invented.

Downloaded tracks, album/playlist galleries, search/source tables, collection tracks and queues were already windowed in the integration. Those mechanisms remain. The controlled catalogue contains 1,036 tracks and 130 albums; the shared feature suite checks bounded mounted track/card rows and keeps their actions usable. No tiny fixed list was newly virtualized.

Large embedded artwork can still require a large decode. The cache bound limits retained frontend strings; it is not a bound on WebKit's decoded-image/GPU caches. No artwork was downsampled or visually simplified without evidence.

## 5. CSS and WebKitGTK findings

Active computed-style inventory found no repeated backdrop blur/filter or mix-blend layer in the tested Home/Discovery/library presentations. The old global noise overlay is already disabled for the production interface. Static cover shadows and tilted sleeves were retained. Modal-only blur remains an existing bounded effect, not a repeated-card effect.

The primary runtime measurement uses WebKitGTK 2.54.1, the embedded production Vite/React frontend and actual Rust silent-WAV playback. The performance fixture controls catalogue queries and style-save responses for repeatable frontend pressure; the separate native continuity suite exercises actual SQLite persistence and Flow. The Rust executable is a debug build with `tauri/custom-protocol`; whole-process CPU includes that backend. Chromium development measurements establish React call-path behavior, not a substitute WebKit frame-rate result.

## 6. Before/after evidence

Baseline observations below were collected before the fixes on the uncommitted integration. The baseline binary and original raw JSON were temporary and were cleared between resumed sessions. These aggregates were read from their completed output before that loss. Final regenerated data is retained in [development-final.json](performance/development-final.json) and [native-final.json](performance/native-final.json).

| Same controlled scenario | Before | After |
| --- | ---: | ---: |
| App/Home renders for 100 download ticks on Home | 100 / 100 | 0 / 0 |
| App renders for 100 playback ticks | 0 | 0 |
| Discovery card renders during 80 small scroll steps (development) | 67 | 1 |
| Native horizontal-scroll sample, total React commits | 151 | 16 |
| Native hidden minute, visual commits | 240 | 0 |
| Delayed update registration, active StrictMode listeners | 2 | 1 |
| Pending lyrics resync timer after navigation | 1 | 0 |
| Discovery strips' ResizeObservers while one Spatial chapter is active | 2 | 1 |
| Inactive Spatial chapter artwork | Mounted | Unmounted |

The initial native comparison had approximately 15–16ms p95 RAF gaps for horizontal browsing on both versions, with no gaps over 32ms in either sample. Rapid chapter changes still occasionally produced one roughly 35–41ms gap. Reduced React work is the demonstrated improvement; a universal frame-rate or complete-stall elimination claim is not supported.

The completed native run performed 20 style switches, six route/album navigation rounds, 16 rapid actual track changes, virtual-library scrolling, one minute hidden to tray and 1,260 seconds of actual playback. Track/queue identity remained intact and the backend position advanced 1,262.725 seconds across the soak and its final UI checks. Median switch dispatch/completion time was 16.1ms (11.3–24.3ms); this is not a paint-latency measurement.

| Final native observation | Result |
| --- | ---: |
| Spatial Home while playing, RAF gap p95 / maximum | 14 / 16ms |
| Discovery horizontal scroll, RAF gap p95 / maximum | 17 / 18ms |
| Rapid actual track changes, RAF gap p95 / maximum | 16 / 18ms |
| Brutalist virtual-library scroll, RAF gap p95 / maximum | 18 / 18ms |
| End-of-soak RAF gap p95 / maximum | 15 / 15ms |
| Rapid chapter changes | One 35ms gap; p95 18ms |
| Whole-app / renderer CPU, idle | 1.8% / 0.2% |
| Whole-app / renderer CPU, either style playing | 5.4% / 2.6% |
| Whole-app / renderer CPU, hidden | 2.48% / 0.33% |
| Whole-app / renderer CPU, finally paused | 1.6% / 0% |

The earlier paired baseline observed idle CPU 2.2% / 0.4%, Spatial playing 5.0% / 2.2%, Brutalist playing 5.2% / 2.4%, and hidden 2.97% / 0.8%. Foreground CPU differences are within short-sample/system noise; a foreground CPU reduction is not established. All frame samples except the chapter stress sample had zero gaps above 32ms.

Memory did **not** remain flat. Private memory was 357.2 MiB before the 20 switches, 405.6 MiB immediately afterward and 376.6 MiB after repeated navigation. Across the subsequent 21-minute soak it rose from 373.3 to 462.4 MiB (+89.1 MiB), almost entirely in WebKitWebProcess. Mounted DOM (538), images (15), listeners and observer counts were identical at soak endpoints; no pending application RAF, timers or intervals accumulated. End-of-soak frame stability remained good. These counts exclude some classes of leak, but do not explain the native memory growth.

## 7. Leaks and remaining limits

The update-banner race is a reproduced listener leak and is fixed. The original backend subscription already correctly disposes late registrations and remains singular. Theme/navigation exercises inspect listener, observer, pending timer and RAF counts; one intentional visual-activity observer remains mounted with App and releases its offscreen targets on route changes.

A memory rise during warm-up does not by itself prove a leak: image decoding, allocator caches, WebKit caches and the inspector all affect private memory. The benchmark records private process memory (`Private_Clean + Private_Dirty`), not summed RSS. CPU is percent of one core. Short samples and compositor/system load introduce noise; use the JSON as observations rather than a guaranteed percentage saving.

A separate [memory diagnostic](performance/memory-diagnostic.json) used the actual production frontend in a fresh disposable native window, stopped backend playback, and delivered three 5,000-position-event bursts through its registered frontend event callback. It sampled the live JavaScript heap after explicit GC. Heap size was 5.005 MiB initially, 4.367 MiB after the normal profiled burst, 4.486 MiB with React profiling/mutation counting disabled, and 4.572 MiB after additionally replacing the seek-track gradient with a flat test-only background. Heap node counts stayed around 32–33 thousand. Renderer private memory nevertheless rose from 208.6 to 242.3 to 292.6 to 312.0 MiB. The sequential experiment does not isolate an allocator/cache/inspector cause; the remaining growth also prevents identifying the gradient as the sole cause. The profiler's other wrappers and inspector polling remained enabled. The test-only CSS was never applied to product code.

**Remaining bottleneck:** native renderer private-memory growth is unresolved. No growing live JavaScript graph or accumulating application handlers/DOM was demonstrated; native allocations, caching and inspector overhead remain hypotheses. A native allocation trace or a fresh inspector-free process experiment is needed before blaming a particular effect or changing engine/backend behavior. The benchmark's `passed` field means its interaction/lifecycle assertions completed; it does not certify bounded memory. The exact intermittent hang was not reproduced in that audit; the focused Queue follow-up below subsequently captured a rendering stall.

No visual identity was simplified. The same sleeves, shadows, type, geometry, interactions and circular familiarity dial remain. Pausing invisible visual work, unmounting inactive chapters and replacing width animation with an equivalent transform do not remove a feature.

Unverified in the original audit: every real remote provider/network failure, the user's exact intermittent-hang sequence (investigated separately below), other operating systems, a release-optimized Rust executable, and an exhaustive native heap/GPU allocation trace. Existing media/lifecycle behavior is exercised separately through the real native continuity suite. The installed application is not overwritten by this work.

## 8. Verification and reproduction

Completed: production build; offline `cargo check --bin kaze`; 42 Node tests including request sharing/cancellation/cache-bound/invalidation cases; all 31 shared feature groups with no page errors; six preview/dial parity groups; nine fixed-stage layout groups; extended development performance regressions; the full 21-minute native performance run; and [nine real native continuity groups](performance/native-continuity.json). Native continuity includes ten hide/restore cycles with no hidden visual mutations, restored current progress, actual Flow steering, queue/MPRIS/history/session continuity, paused-position equality and cold-launch SQLite style persistence. The last lyrics timeout cleanup was built and checked through the final browser/continuity runs after the full soak; it did not change the progress/scroll paths measured in that soak.

```sh
npm run build
node --test --test-isolation=none --test-reporter=tap tests/*.test.mjs
cargo check --bin kaze --manifest-path src-tauri/Cargo.toml --offline
KAZE_ASSERT_PERFORMANCE=1 node scripts/profile-frontend.mjs /absolute/output/development.json
node scripts/test-dual-interface.mjs
node scripts/test-spatial-layout.mjs
node scripts/test-preview-parity.mjs
cargo build --manifest-path src-tauri/Cargo.toml --offline --bin kaze --features tauri/custom-protocol
KAZE_ASSERT_PERFORMANCE=1 KAZE_PRIVATE_PERF_TEST=1 dbus-run-session -- python3 scripts/profile-frontend-native.py /absolute/copied/kaze /absolute/output/native.json --soak-seconds 1260
KAZE_PRIVATE_DUAL_TEST=1 dbus-run-session -- python3 scripts/test-dual-interface-native.py /absolute/copied/kaze
KAZE_PRIVATE_PERF_TEST=1 dbus-run-session -- python3 scripts/profile-frontend-native.py /absolute/copied/kaze /absolute/output/memory.json --memory-diagnostic
```

The browser scripts expect a local Vite server on port 5194 (override with `KAZE_TEST_URL`). The native harness uses disposable data, a private D-Bus session, its own real window/tray, a controlled catalogue and actual silent audio. Profiling is injected through the inspector; `scripts/performance-probe.js` is not imported or shipped by the app. Native UI suites must run sequentially because they share compositor focus. Copy the embedded executable before other Cargo commands can replace it. Keep benchmark output in the project for long/resumed tasks rather than relying on `/tmp` survival.


## Focused Queue follow-up (2026-10-18)

The user rejected the nested-scroll mitigation after reproducing the same freeze; that layout change is reverted. The normal-profile window continued executing JavaScript and navigating internally while its displayed Queue frame stayed stale, with zero animation frames even when focused. A same-binary native A/B experiment reproduces the stall with default DMA-BUF rendering and passes six Queue/Home rounds when only `WEBKIT_DISABLE_DMABUF_RENDERER=1` changes. Linux startup now supplies that workaround for NVIDIA/Wayland while preserving explicit overrides and other platforms/sessions. This is a verified workaround on this host, not a traced upstream driver fix.

The rebuilt original-layout native regression passes twelve focused frame probes over six Queue/Home round trips with real Rust artwork, simulated progress and both outer/native-key and inner/programmatic scrolling. No external rendering variable is needed; the startup policy applies it. The normal-profile diagnostic window was reopened, and the user confirmed their original music/Flow → What’s next → scroll → Home sequence now stays responsive. See [the interface audit](dual-interface-audit.md#queue-frame-stall-workaround-2026-10-18), [A/B evidence](performance/queue-frames-comparison.json) and [final regression](performance/queue-frames-native.json). The earlier native private-memory growth is separately unresolved; this short frame test does not establish bounded memory or certify a particular frame rate.
