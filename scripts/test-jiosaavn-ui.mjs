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
      f.query = async q => {
        const result = await query(q);
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
    assert.equal(await page.getByLabel("Online music source").textContent(), "JioSaavn");
    assert.equal(await page.getByRole("combobox", { name: "Online music source" }).count(), 0);
    const search = page.getByRole("textbox", { name: "Search music online" });
    await search.fill("Test Song"); await search.press("Enter");
    await page.waitForFunction(() => window.__kazeFixture.queries.some(q => q.query === "SearchOnlineMusic" && q.payload.provider === "jiosaavn"));
    await page.locator(".sidebar,.press-account-navigation").getByRole("button", { name: "Settings", exact: true }).click();
    const panel = page.locator(".settings-view");
    const availability = panel.getByRole("switch", { name: "Use JioSaavn for online music" });
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
    console.log(`PASS ${style}: exclusive JioSaavn source, availability, automatic fill, manual fill/retry/removal`);
    await context.close();
  }
} finally { await browser.close(); }
