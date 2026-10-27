/** Real App with disposable IPC fixtures: source selection and reversible enrichment controls. */
import assert from "node:assert/strict";
import { installDualInterfaceFixture } from "../tests/fixtures/dualInterfaceBackend.mjs";
const { chromium } = await import(process.env.KAZE_PLAYWRIGHT_MODULE || "/home/abhi/.cache/ms-playwright-go/1.57.0/package/index.mjs");
const browser = await chromium.launch({ executablePath: process.env.KAZE_CHROMIUM || "/usr/bin/chromium", headless: true, args: ["--no-sandbox"] });
try {
  for (const style of ["spatial", "brutalist"]) {
    const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
    await context.addInitScript({ content: `sessionStorage.setItem('test.interface-style','${style}');(${installDualInterfaceFixture.toString()})();` });
    await context.addInitScript(() => {
      const f = window.__kazeFixture, query = f.query.bind(f), command = f.command.bind(f);
      const preferences = { enable_jiosaavn: true, auto_fill_metadata: true };
      Object.assign(f.playback, {
        current_track: { ...f.tracks[0], id: "itunes:legacy", title: "Dance With Me", artist_name: "Bruno Mars", file_path: "online://itunes:legacy", format: "online", duration_secs: 219, playback_source: { provider: "jiosaavn", track_id: "fixtureAlbum" } },
        current_track_id: "itunes:legacy", is_playing: false, is_paused: false, duration_secs: 219,
      });
      f.query = async q => {
        if (q.query === "SearchOnlineMusic" && q.payload.query === "Unavailable Song") {
          return new Promise(resolve => { window.__completeUnavailableSearch = () => resolve({ type: "DiscoveryRecommendations", data: [] }); });
        }
        const result = await query(q);
        if (q.query === "SearchOnlineMusic") result.data = result.data.slice(0, 2).map((track, index) => ({ ...track, title: "Dance With Me", artist: "Bruno Mars", album: index ? "The Romantic" : "Single", duration_secs: 219, provider: "jiosaavn", provider_id: index ? "fixtureAlbum" : "fixtureSingle", external_track_id: index ? "jiosaavn:fixtureAlbum" : "jiosaavn:fixtureSingle", playback_source: { provider: "jiosaavn", track_id: index ? "fixtureAlbum" : "fixtureSingle" } }));
        if (q.query === "GetSettings") result.data.metadata = { ...result.data.metadata, ...preferences };
        return result;
      };
      f.command = async c => {
        const result = await command(c);
        if (c.command === "SetMetadataPreference") preferences[c.payload.key] = c.payload.enabled;
        if (c.command === "RemoveProviderEnrichment") preferences.auto_fill_metadata = false;
        return result;
      };
    });
    await context.route("**/src/services/api.ts*", route => route.fulfill({ contentType: "text/javascript", body: `export const isTauri=()=>false;export const executeQuery=q=>window.__kazeFixture.query(q);export const dispatchCommand=c=>window.__kazeFixture.command(c);export const subscribeBackendEvents=async cb=>{window.__kazeFixture.listeners.add(cb);return ()=>window.__kazeFixture.listeners.delete(cb);};` }));
    await context.route("https://**", route => route.fulfill({ status: 404, body: "fixture" }));
    const page = await context.newPage(), errors = [];
    page.on("pageerror", error => errors.push(error.message));
    await page.goto(process.env.KAZE_TEST_URL || "http://127.0.0.1:5194", { waitUntil: "domcontentloaded" });
    assert.equal(await page.getByLabel("Online music source").count(), 0);
    assert.equal(await page.getByRole("combobox", { name: "Online music source" }).count(), 0);
    await page.locator(".play-pause-btn").first().click();
    await page.waitForFunction(() => window.__kazeFixture.commands.some(c => c.command === "Resume"));
    assert.equal(await page.evaluate(() => window.__kazeFixture.queries.some(q => q.query === "SearchOnlineMusic")), false, "playing a source-linked saved ID must not redirect to search");
    const search = page.getByRole("textbox", { name: "Search music online" });
    await search.fill("Test Song"); await search.press("Enter");
    await page.waitForFunction(() => window.__kazeFixture.queries.some(q => q.query === "SearchOnlineMusic" && q.payload.provider === "jiosaavn" && q.payload.query === "Test Song"));
    const selected = page.locator('.discovery-search-active button[title="Play"]').first();
    await selected.waitFor(); await selected.click();
    await page.waitForFunction(() => window.__kazeFixture.commands.some(c => c.command === "PlayOnlineTrack" && c.payload.track_id === "jiosaavn:fixtureSingle"));
    await search.fill("Unavailable Song"); await search.press("Enter");
    await page.waitForFunction(() => typeof window.__completeUnavailableSearch === "function");
    await page.getByText("Searching music…", { exact: true }).waitFor({ timeout: 5000 });
    assert.equal(await page.locator(".discovery-hero").count(), 0, "catalog shelves must not appear as search results while loading");
    await page.evaluate(() => window.__completeUnavailableSearch());
    await page.getByText('No songs found for “Unavailable Song”', { exact: true }).waitFor();
    assert.equal(await page.locator(".discovery-hero").count(), 0);
    await page.locator(".sidebar,.press-account-navigation").getByRole("button", { name: "Settings", exact: true }).click();
    const panel = page.locator(".settings-view");
    const availability = panel.getByRole("switch", { name: "Enable online music" });
    const auto = panel.getByRole("switch", { name: "Automatically fill missing artwork and details" });
    await availability.waitFor(); assert.equal(await availability.isChecked(), true); assert.equal(await auto.isChecked(), true);
    await availability.uncheck();
    await page.waitForFunction(() => window.__kazeFixture.commands.some(c => c.command === "SetMetadataPreference" && c.payload.key === "enable_jiosaavn" && !c.payload.enabled));
    await panel.getByRole("button", { name: "Fill missing artwork and details", exact: true }).click();
    await panel.getByRole("button", { name: "Retry unmatched songs", exact: true }).click();
    await panel.getByRole("button", { name: "Remove provider-filled data", exact: true }).click();
    await page.waitForFunction(() => window.__kazeFixture.commands.some(c => c.command === "RemoveProviderEnrichment"));
    assert.equal(await auto.isChecked(), false);
    const fills = await page.evaluate(() => window.__kazeFixture.commands.filter(c => c.command === "FillLibraryGaps").map(c => c.payload.retry));
    assert.deepEqual(fills, [false, true]);
    await page.screenshot({ path: `/tmp/kaze-jiosaavn-${style}-settings.png` });
    assert.deepEqual(errors, []);
    console.log(`PASS ${style}: exclusive JioSaavn source, source-linked saved entry plays directly, selected release keeps its exact ID, empty search without catalog fallback, availability, automatic fill, manual fill/retry/removal`);
    await context.close();
  }
} finally { await browser.close(); }
