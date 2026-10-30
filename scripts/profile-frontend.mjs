/** Real development App, controlled catalogue, and opt-in profiling probe. */
import assert from "node:assert/strict";
import { readFile, mkdir, writeFile } from "node:fs/promises";
import { installDualInterfaceFixture } from "../tests/fixtures/dualInterfaceBackend.mjs";
const { chromium } = await import(
  process.env.KAZE_PLAYWRIGHT_MODULE ||
    "/home/abhi/.cache/ms-playwright-go/1.57.0/package/index.mjs"
);
const output = process.argv[2] || "/tmp/kaze-performance/development.json";
await mkdir(new URL(".", `file://${output}`).pathname, { recursive: true });
const browser = await chromium.launch({
  executablePath: "/usr/bin/chromium",
  headless: true,
  args: ["--no-sandbox"],
});
const context = await browser.newContext({
  viewport: { width: 1440, height: 900 },
});
await context.addInitScript({
  content: await readFile(
    new URL("./performance-probe.js", import.meta.url),
    "utf8",
  ),
});
await context.addInitScript(installDualInterfaceFixture);
await context.addInitScript(() => {
  const fixture = window.__kazeFixture, query = fixture.query.bind(fixture);
  fixture.query = request => request.query === "GetTrackLyrics"
    ? Promise.resolve({type:"Lyrics",data:{syncedLyrics:Array.from({length:80},(_,i)=>`[00:${String(i).padStart(2,"0")}.00]Test line ${i}`).join("\n")}})
    : query(request);
});
await context.addInitScript(() => (window.__perfNativeListeners = new Set()));
await context.route(
  "**/node_modules/.vite/deps/@tauri-apps_api_event.js*",
  (route) =>
    route.fulfill({
      contentType: "text/javascript",
      body: "export const listen=async(name,handler)=>{await new Promise(resolve=>setTimeout(resolve,80));window.__perfNativeListeners.add(handler);return ()=>window.__perfNativeListeners.delete(handler);};",
    }),
);
await context.route("**/src/services/api.ts*", (route) =>
  route.fulfill({
    contentType: "text/javascript",
    body: "export const isTauri=()=>true;export const executeQuery=q=>window.__kazeFixture.query(q);export const dispatchCommand=c=>window.__kazeFixture.command(c);export const subscribeBackendEvents=async cb=>{window.__kazeFixture.listeners.add(cb);return ()=>window.__kazeFixture.listeners.delete(cb);};",
  }),
);
const page = await context.newPage();
const report = { target: "development Chromium", measurements: {} };
const wait = (ms) => page.waitForTimeout(ms);
const save = () => writeFile(output, JSON.stringify(report, null, 2));
const snap = async (name) => {
  report.measurements[name] = await page.evaluate(() => ({
    ...window.__kazePerf.snapshot(true),
    backendListeners: window.__kazeFixture.listeners.size,
    updateListeners: window.__perfNativeListeners.size,
    queries: window.__kazeFixture.queries.reduce(
      (all, q) => ((all[q.query] = (all[q.query] || 0) + 1), all),
      {},
    ),
  }));
  await save();
  const m = report.measurements[name];
  console.log(
    "MEASURE",
    name,
    JSON.stringify({ commits: m.commits, dom: m.dom, images: m.images }),
  );
};
const reset = () => page.evaluate(() => window.__kazePerf.reset());
const nav = async (name) => {
  const direct = page.locator(".sidebar,.press-header,.press-account-navigation").getByRole("button", { name, exact: true });
  if (await direct.count()) await direct.click();
  else {
    await page.getByRole('button', {name:'Library and more',exact:true}).click();
    await page.getByRole('menuitem', {name,exact:true}).click();
  }
  await wait(100);
};
const tick = async (event, payload, count = 100) => {
  await page.evaluate(
    ({ event, payload, count }) => {
      let n = 0;
      window.__perfInterval = setInterval(() => {
        window.__kazeFixture.emit({
          event,
          payload: {
            ...payload,
            position_secs: n * 0.1,
            bytes_downloaded: n * 1000,
          },
        });
        if (++n === count) clearInterval(window.__perfInterval);
      }, 10);
    },
    { event, payload, count },
  );
  await wait(count * 10 + 150);
};
try {
  await page.goto(process.env.KAZE_TEST_URL || "http://127.0.0.1:5194");
  await page.locator(".home-flow").waitFor();
  await wait(1000);
  await page.evaluate(() => window.__kazePerf.attach());
  await snap("initial");
  await reset();
  await tick("PlaybackPositionChanged", { duration_secs: 240 });
  await snap("playback_100_ticks");
  await reset();
  await tick("DownloadProgress", { task_id: "d1" });
  await snap("download_100_ticks");
  await nav("Discover Music");
  await wait(500);
  await snap("discovery_mounted");
  await reset();
  const strip = page
    .locator(".discovery-chapter:not([hidden]) .horizontal-scroll-row")
    .first();
  await strip.evaluate((el) => {
    let n = 0;
    window.__perfScroll = setInterval(() => {
      el.scrollLeft = ++n * 3;
      if (n === 80) clearInterval(window.__perfScroll);
    }, 16);
  });
  await wait(1500);
  await snap("scroll_80_steps");
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await page.locator(".settings-style-options").waitFor();
  await wait(300);
  await snap("before_cycles");
  for (let i = 0; i < 20; i++) {
    await page
      .getByRole("button", {
        name: new RegExp(`^${i % 2 ? "Spatial" : "Brutalist"}`),
      })
      .click();
    await wait(50);
  }
  await snap("after_20_cycles");
  // Direct dragging must follow the pointer in the same event, without a smooth-scroll tail.
  await nav("Discover Music");
  await page.locator(".chapter-nav").waitFor();
  const box = await page
    .locator(".discovery-chapter:not([hidden]) .horizontal-scroll-row")
    .boundingBox();
  const dragStart = await strip.evaluate(el => el.scrollLeft);
  await page.mouse.move(box.x + box.width / 2, box.y + 40);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width / 2 - 100, box.y + 40);
  report.drag = await page.evaluate(() => ({
    left: document.querySelector(
      ".discovery-chapter:not([hidden]) .horizontal-scroll-row",
    ).scrollLeft,
    behavior: getComputedStyle(
      document.querySelector(
        ".discovery-chapter:not([hidden]) .horizontal-scroll-row",
      ),
    ).scrollBehavior,
  }));
  report.drag.start = dragStart;
  await page.mouse.up();
  await wait(250);
  report.releasedShelf = await strip.evaluate(el => el.scrollLeft);
  await page.locator(".chapter-nav button").nth(1).click();
  await page.locator(".chapter-nav button").nth(0).click();
  await wait(250);
  report.restoredShelf = await page
    .locator(".discovery-chapter:not([hidden]) .horizontal-scroll-row")
    .evaluate((el) => el.scrollLeft);
  // Both themes consume the same isolated transfer stream.
  await nav("Settings");
  await page.getByRole("button", { name: /^Brutalist/ }).click();
  await nav("Home");
  await wait(350);
  await reset();
  await tick("DownloadProgress", { task_id: "d1" });
  await snap("brutalist_download_100_ticks");
  await reset();
  await tick("PlaybackPositionChanged", { duration_secs: 240 });
  await snap("brutalist_playback_100_ticks");
  await nav("Settings");
  await page.getByRole("button", { name: /^Spatial/ }).click();
  await nav("Downloads");
  await page
    .getByRole("heading", {name:"Saved downloads",exact:true})
    .waitFor();
  await reset();
  await tick("DownloadProgress", { task_id: "d1" });
  await snap("visible_download_100_ticks");
  await page.getByTitle("Show Lyrics", {exact:true}).click();
  const lyrics = page.locator(".main-content .custom-scrollbar");
  await lyrics.waitFor();
  await lyrics.evaluate(el => {el.style.scrollBehavior="auto";el.scrollTop=600;});
  await wait(250);
  const lyricTimers = await page.evaluate(()=>window.__kazePerf.snapshot().tasks.timers);
  await nav("Home");await wait(350);
  report.lyricsTimers = {mounted:lyricTimers,afterLeaving:await page.evaluate(()=>window.__kazePerf.snapshot().tasks.timers)};
  await page.evaluate(() =>
    window.__kazeFixture.emit({ event: "ScanCompleted", payload: {} }),
  );
  await page
    .getByTitle("Dismiss notification", { exact: true })
    .first()
    .waitFor();
  report.toastKeyframes = await page.evaluate(() =>
    document
      .getAnimations()
      .find((a) => a.animationName === "toastProgressBar")
      ?.effect.getKeyframes(),
  );
  await save();
  if (process.env.KAZE_ASSERT_PERFORMANCE === "1") {
    assert.equal(
      report.measurements.playback_100_ticks.renders.App || 0,
      0,
      "playback ticks must stay in progress leaves",
    );
    assert.equal(
      report.measurements.download_100_ticks.renders.App || 0,
      0,
      "download ticks must not rerender App",
    );
    assert.equal(
      report.measurements.discovery_mounted.hiddenChapterImages,
      0,
      "inactive Spatial chapters must be unmounted",
    );
    assert.equal(
      report.measurements.after_20_cycles.backendListeners,
      1,
      "one backend listener after twenty switches",
    );
    assert.equal(
      report.measurements.after_20_cycles.updateListeners,
      1,
      "a delayed update subscription must dispose the abandoned StrictMode mount",
    );
    assert.equal(
      report.measurements.brutalist_download_100_ticks.renders.App || 0,
      0,
    );
    assert.equal(
      report.measurements.brutalist_playback_100_ticks.renders.App || 0,
      0,
    );
    assert.equal(
      report.measurements.visible_download_100_ticks.renders.App || 0,
      0,
    );
    assert.ok(
      (report.measurements.visible_download_100_ticks.renders
        .LiveDownloadTask2 ||
        report.measurements.visible_download_100_ticks.renders
          .LiveDownloadTask ||
        0) > 0,
      "visible transfer rows remain live",
    );
    assert.ok(report.lyricsTimers.mounted>0,"manual lyric scroll schedules resync");
    assert.equal(report.lyricsTimers.afterLeaving,0,"leaving lyrics cancels its pending resync");
    assert.ok(
      report.toastKeyframes?.every(
        (frame) => frame.width === undefined && frame.transform !== undefined,
      ),
      "toast expiration must animate transform only",
    );
    assert.equal(
      report.drag.behavior,
      "auto",
      "drag must bypass CSS smoothing",
    );
    assert.ok(
      Math.abs(report.drag.left - report.drag.start - 100) < 2,
      "drag must follow pointer immediately",
    );
    assert.ok(
      Math.abs(report.restoredShelf - report.releasedShelf) < 2,
      "unmounted chapter restores shelf position",
    );
  }
} finally {
  await browser.close();
}
