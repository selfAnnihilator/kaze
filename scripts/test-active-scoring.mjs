/** Verify the real App refreshes personal picks only after committed taste changes. */
import assert from "node:assert/strict";
import { installDualInterfaceFixture } from "../tests/fixtures/dualInterfaceBackend.mjs";
const { chromium } = await import(process.env.KAZE_PLAYWRIGHT_MODULE || "/home/abhi/.cache/ms-playwright-go/1.57.0/package/index.mjs");
const browser = await chromium.launch({ executablePath: process.env.KAZE_CHROMIUM || "/usr/bin/chromium", headless: true, args: ["--no-sandbox"] });
const base = process.env.KAZE_TEST_URL || "http://127.0.0.1:5194";
try {
  for (const style of ["spatial", "brutalist"]) {
    const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
    await context.addInitScript(installDualInterfaceFixture);
    await context.addInitScript((style) => {
      sessionStorage.setItem("test.interface-style", style);
      const f = window.__kazeFixture;
      f.settings.interface_style = style;
      f.recs.splice(0, f.recs.length, {
        external_track_id: "online:active-pick", provider: "fixture", provider_id: "active-pick",
        title: "Initial Pick", artist: "Learned Artist", duration_secs: 240,
        recommendation_reason: "Based on your listening habit", match_status: "NOT_FOUND", in_wishlist: false,
      });
      const originalQuery = f.query.bind(f);
      let initialDiscovery = true;
      f.query = async q => {
        if (q.query === "GetDiscoveryRecommendations" && initialDiscovery) {
          initialDiscovery = false;
          f.queries.push(q);
          return new Promise(resolve => { f.resolveInitial = () => resolve({ type: "DiscoveryRecommendations", data: [{ ...f.recs[0], title: "Guest Stale Pick" }] }); });
        }
        return originalQuery(q);
      };
    }, style);
    await context.route("**/src/services/api.ts*", route => route.fulfill({ contentType: "text/javascript", body: `export const isTauri=()=>false;export const executeQuery=q=>window.__kazeFixture.query(q);export const dispatchCommand=c=>window.__kazeFixture.command(c);export const subscribeBackendEvents=async cb=>{window.__kazeFixture.listeners.add(cb);return ()=>window.__kazeFixture.listeners.delete(cb);};` }));
    const page = await context.newPage();
    const errors = [];
    page.on("pageerror", error => errors.push(error.message));
    await page.goto(base);
    await page.locator(".home-flow").waitFor();
    await page.locator(".sidebar, .press-header, .press-account-navigation").getByRole("button", { name: "Discover Music", exact: true }).click();
    await page.locator(".discovery-chapters").waitFor();
    if (style === "spatial") await page.locator(".chapter-nav button").nth(1).click();
    else await page.getByRole("combobox", { name: "Discovery selection" }).selectOption("1");
    await page.getByText("Initial Pick", { exact: true }).first().waitFor();
    await page.evaluate(() => window.__kazeFixture.resolveInitial());
    await page.waitForTimeout(400);
    assert.equal(await page.getByText("Guest Stale Pick", { exact: true }).count(), 0,
      "initial account hydration must replace an outstanding guest-scoped request");
    const counts = () => page.evaluate(() => {
      const f = window.__kazeFixture;
      return ["GetDiscoveryRecommendations", "GetWorldTrending"].map(name => f.queries.filter(q => q.query === name).length);
    });
    const before = await counts();
    await page.evaluate(() => {
      const f = window.__kazeFixture;
      f.recs[0].title = "Learned Pick";
      for (let i = 0; i < 4; i++) f.emit({ event: "TasteProfileChanged", payload: { user_id: "u1" } });
    });
    await page.getByText("Learned Pick", { exact: true }).first().waitFor();
    assert.deepEqual(await counts(), [before[0] + 1, before[1]], "coalesced refresh must leave public charts alone");
    await page.evaluate(() => {
      const f = window.__kazeFixture;
      f.emit({ event: "TasteProfileChanged", payload: { user_id: "someone-else" } });
      for (let i = 0; i < 50; i++) f.emit({ event: "PlaybackPositionChanged", payload: { session_id: "fixture", position_secs: i, duration_secs: 240 } });
    });
    await page.waitForTimeout(400);
    assert.deepEqual(await counts(), [before[0] + 1, before[1]], "ticks and another account must not refresh taste");
    console.log(`PASS ${style}: post-commit refresh, rapid-skip debounce, chart and account isolation, no tick queries`);

    await page.evaluate(() => {
      const f = window.__kazeFixture, query = f.query.bind(f);
      f.deferNext = true;
      f.query = async q => {
        if (q.query === "GetDiscoveryRecommendations" && f.deferNext) {
          f.deferNext = false;
          f.queries.push(q);
          return new Promise(resolve => { f.resolveStale = () => resolve({ type: "DiscoveryRecommendations", data: [{ ...f.recs[0], title: "Stale Pick" }] }); });
        }
        return query(q);
      };
      f.emit({ event: "TasteProfileChanged", payload: { user_id: "u1" } });
    });
    await page.waitForFunction(() => !!window.__kazeFixture.resolveStale);
    await page.evaluate(() => {
      const f = window.__kazeFixture;
      f.recs[0].title = "Fresh Pick";
      f.emit({ event: "TasteProfileChanged", payload: { user_id: "u1" } });
    });
    await page.getByText("Fresh Pick", { exact: true }).first().waitFor();
    await page.evaluate(() => window.__kazeFixture.resolveStale());
    await page.waitForTimeout(200);
    assert.equal(await page.getByText("Stale Pick", { exact: true }).count(), 0);
    await page.getByText("Fresh Pick", { exact: true }).first().waitFor();
    assert.equal(await page.evaluate(() => window.__kazeFixture.listeners.size), 1);
    assert.deepEqual(errors, []);
    console.log(`PASS ${style}: delayed older recommendations cannot replace newer listening results; one backend subscription`);
    await context.close();
  }
} finally { await browser.close(); }
