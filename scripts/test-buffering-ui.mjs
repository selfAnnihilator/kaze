/** Real App, deterministic reader-event replay; no network or user profile. */
import assert from "node:assert/strict";
import { installDualInterfaceFixture } from "../tests/fixtures/dualInterfaceBackend.mjs";
const { chromium } = await import(process.env.KAZE_PLAYWRIGHT_MODULE || "/home/abhi/.cache/ms-playwright-go/1.57.0/package/index.mjs");
const browser = await chromium.launch({ executablePath: process.env.KAZE_CHROMIUM || "/usr/bin/chromium", headless: true, args: ["--no-sandbox"] });
try {
  for (const style of ["spatial", "brutalist"]) {
    const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
    await context.addInitScript({ content: `sessionStorage.setItem('test.interface-style','${style}');(${installDualInterfaceFixture.toString()})();` });
    await context.route("**/src/services/api.ts*", route => route.fulfill({ contentType: "text/javascript", body: `export const isTauri=()=>false;export const executeQuery=q=>window.__kazeFixture.query(q);export const dispatchCommand=c=>window.__kazeFixture.command(c);export const subscribeBackendEvents=async cb=>{window.__kazeFixture.listeners.add(cb);return ()=>window.__kazeFixture.listeners.delete(cb);};` }));
    await context.route("https://**", route => route.fulfill({ status: 404, body: "fixture" }));
    const page = await context.newPage(), errors = [];
    page.setDefaultTimeout(5000);
    page.on("pageerror", error => errors.push(error.message));
    await page.goto(process.env.KAZE_TEST_URL || "http://127.0.0.1:5194", { waitUntil: "domcontentloaded" });
    await page.waitForFunction(() => window.__kazeFixture.listeners.size > 0);
    const emit = (event, payload) => page.evaluate(({ event, payload }) => {
      for (const listener of window.__kazeFixture.listeners) listener({ event, payload });
    }, { event, payload });
    const control = page.locator(".play-pause-btn").first();
    const started = { session_id: "session-1", track_id: "t0", title: "Current song", artist: "Artist", duration_secs: 240, source: "library" };
    await emit("PlaybackStarted", started);
    await control.getAttribute("aria-label");
    await page.waitForFunction(() => document.querySelector(".play-pause-btn")?.getAttribute("aria-label") === "Pause");
    const change = (is_buffering, overrides = {}) => emit("PlaybackBufferingChanged", { session_id: "session-1", track_id: "t0", is_buffering, ...overrides });
    await change(true);
    await page.waitForFunction(() => document.querySelector(".play-pause-btn")?.getAttribute("aria-label") === "Loading song");
    await emit("PlaybackPositionChanged", { session_id: "session-1", position_secs: 42, duration_secs: 240 });
    await change(false);
    await page.waitForFunction(() => document.querySelector(".play-pause-btn")?.getAttribute("aria-label") === "Pause");
    assert.equal(await page.getByRole("slider", { name: "Seek", exact: true }).first().inputValue(), "42", "buffer recovery must preserve playback progress");
    await page.evaluate(() => {
      const f = window.__kazeFixture, query = f.query.bind(f);
      f.query = async q => {
        if (q.query === "GetPlaybackState" && window.__holdPlaybackSnapshot) {
          window.__holdPlaybackSnapshot = false;
          const result = structuredClone(await query(q));
          result.data.is_buffering = true;
          return new Promise(resolve => { window.__releaseStalePlayback = () => resolve(result); });
        }
        return query(q);
      };
      window.__holdPlaybackSnapshot = true;
    });
    await emit("LibraryMetadataUpdated", {});
    await page.waitForFunction(() => typeof window.__releaseStalePlayback === "function");
    await change(true); await change(false);
    await page.evaluate(() => window.__releaseStalePlayback());
    await page.waitForTimeout(100);
    assert.equal(await control.getAttribute("aria-label"), "Pause", "a slow state response must not restore buffering after recovery");
    await change(true, { track_id: "t1", session_id: "next-song" });
    await change(true, { session_id: "obsolete-session" });
    await change(true, { track_id: "t1" });
    await page.waitForTimeout(100);
    assert.equal(await control.getAttribute("aria-label"), "Pause", "background/obsolete readers must not buffer the current song");
    await emit("PlaybackPaused", { session_id: "session-1", track_id: "t0", position_secs: 42 });
    await change(true); await change(false);
    await page.waitForFunction(() => document.querySelector(".play-pause-btn")?.getAttribute("aria-label") === "Play");
    assert.equal(await control.isEnabled(), true, "download recovery must not resume a paused song");
    await emit("PlaybackStarted", { ...started, session_id: "session-2" });
    await change(true); // Previous playback of the very same song.
    await page.waitForTimeout(100);
    assert.equal(await control.getAttribute("aria-label"), "Pause");
    await page.evaluate(() => {
      const f = window.__kazeFixture, command = f.command.bind(f), query = f.query.bind(f);
      f.query = async q => {
        const result = await query(q);
        if (q.query === "SearchOnlineMusic") result.data = result.data.map((track, i) => ({
          ...track, provider: "jiosaavn", playback_source: { provider: "jiosaavn", track_id: `fixture-${i}` },
        }));
        return result;
      };
      f.command = async c => {
        const result = await command(c);
        if (["PlayTrack", "PlayOnlineTrack"].includes(c.command)) {
          await new Promise(resolve => { window.__finishPlayCommand = resolve; });
        }
        return result;
      };
    });
    const search = page.getByRole("textbox", { name: "Search music online" });
    await search.fill("Test Song"); await search.press("Enter");
    const selection = page.locator('.discovery-search-active button[title="Play"]').first();
    await selection.waitFor();
    await selection.click();
    await page.waitForFunction(() => typeof window.__finishPlayCommand === "function", null, { timeout: 5000 }).catch(async error => {
      console.error(await page.evaluate(() => window.__kazeFixture.commands.slice(-4)));
      throw error;
    });
    assert.equal(await control.getAttribute("aria-label"), "Pause", "PlaybackStarted must stop loading while the command finishes background work");
    await page.evaluate(() => window.__finishPlayCommand());
    assert.deepEqual(errors, []);
    console.log(`PASS ${style}: current-session buffering clears; stale responses and next/obsolete sessions ignored; progress/pause preserved; started audio clears pending command loading`);
    await context.close();
  }
} finally { await browser.close(); }
