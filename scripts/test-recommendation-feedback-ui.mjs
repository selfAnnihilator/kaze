import assert from "node:assert/strict";
import { installDualInterfaceFixture } from "../tests/fixtures/dualInterfaceBackend.mjs";
const { chromium } = await import(process.env.KAZE_PLAYWRIGHT_MODULE || "/home/abhi/.cache/ms-playwright-go/1.57.0/package/index.mjs");
const browser = await chromium.launch({ executablePath: process.env.KAZE_CHROMIUM || "/usr/bin/chromium", headless: true, args: ["--no-sandbox"] });
try {
  for (const style of ["spatial", "brutalist"]) {
    const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
    await context.addInitScript({ content: `sessionStorage.setItem('test.interface-style','${style}');(${installDualInterfaceFixture.toString()})();` });
    await context.addInitScript(() => {
      const f = window.__kazeFixture, command = f.command.bind(f), query = f.query.bind(f);
      const hidden = new Set();
      f.command = async c => {
        if (["DislikeTrack", "RemoveTrackFeedback"].includes(c.command)) {
          f.commands.push(c);
          const id = c.payload.track_id, disliked = c.command === "DislikeTrack";
          if (disliked) hidden.add(id); else hidden.delete(id);
          f.emit({ event: "RecommendationFeedbackChanged", payload: { user_id: "u1", track_id: id, track_ids: [id], title: "Kerala", disliked, manual_like: disliked ? -1 : 0 } });
          f.emit({ event: "TasteProfileChanged", payload: { user_id: "u1" } });
          return { status: "Ok" };
        }
        return command(c);
      };
      f.query = async q => {
        const result = await query(q);
        if (["GetDiscoveryRecommendations", "GetWorldTrending"].includes(q.query)) result.data = result.data.filter(song => !hidden.has(song.external_track_id));
        if (q.query === "GetHomeOverview") for (const key of ["recent_rotation", "for_you", "rediscover"]) result.data[key] = result.data[key].filter(song => !hidden.has(song.track_id));
        return result;
      };
    });
    await context.route("**/src/services/api.ts*", route => route.fulfill({ contentType: "text/javascript", body: `export const isTauri=()=>false;export const executeQuery=q=>window.__kazeFixture.query(q);export const dispatchCommand=c=>window.__kazeFixture.command(c);export const subscribeBackendEvents=async cb=>{window.__kazeFixture.listeners.add(cb);return ()=>window.__kazeFixture.listeners.delete(cb);};` }));
    await context.route("https://**", route => route.fulfill({ status: 404, body: "fixture" }));
    const page = await context.newPage(), errors = []; page.setDefaultTimeout(10000);
    page.on("pageerror", error => errors.push(error.message));
    await page.goto(process.env.KAZE_TEST_URL || "http://127.0.0.1:5194", { waitUntil: "domcontentloaded" });
    const home = page.locator(style === "spatial" ? ".spatial-home" : ".press-home");
    // Mark a Home recommendation while the current track continues playing.
    await home.waitFor();
    await home.getByRole("button", { name: "Not interested in Kerala", exact: true }).first().waitFor();
    const countBefore = await home.getByRole("button", { name: "Not interested in Kerala", exact: true }).count();
    const control = home.getByRole("button", { name: "Not interested in Kerala", exact: true }).first();
    await control.click();
    await page.waitForFunction(() => window.__kazeFixture.commands.some(c => c.command === "DislikeTrack"));
    await page.getByText('Won’t recommend “Kerala” again.', { exact: true }).waitFor();
    const command = await page.evaluate(() => window.__kazeFixture.commands.find(c => c.command === "DislikeTrack"));
    assert.ok(command.payload.track_id);
    assert.ok(await home.getByRole("button", { name: "Not interested in Kerala", exact: true }).count() < countBefore, "the dismissed Home song must disappear");
    await page.getByRole("button", { name: "Allow again", exact: true }).click();
    await page.waitForFunction(() => window.__kazeFixture.commands.some(c => c.command === "RemoveTrackFeedback"));
    await page.waitForFunction(() => !document.querySelector(".recommendation-feedback-toast"));
    assert.equal(await page.evaluate(() => window.__kazeFixture.commands.some(c => ["PlayTrack", "PlayOnlineTrack", "NextTrack", "Pause"].includes(c.command))), false, "feedback must not start/skip/pause playback");
    assert.ok(await home.count());
    assert.deepEqual(errors, []);
    console.log(`PASS ${style}: Not interested persists via command, hides recommendations, Allow again, playback untouched`);
    await context.close();
  }
} finally { await browser.close(); }
