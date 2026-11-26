/** Real mobile entrypoint with an unavailable settings bridge; disposable browser profiles only. */
import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import { installDualInterfaceFixture } from "../tests/fixtures/dualInterfaceBackend.mjs";

const { chromium } = await import(process.env.KAZE_PLAYWRIGHT_MODULE || "/home/abhi/.cache/ms-playwright-go/1.57.0/package/index.mjs");
const output = process.env.KAZE_TEST_OUTPUT || "/tmp/kaze-mobile-startup-recovery";
const base = process.env.KAZE_TEST_URL || "http://127.0.0.1:5198";
await mkdir(output, { recursive: true });
const browser = await chromium.launch({ executablePath: process.env.KAZE_CHROMIUM || "/usr/bin/chromium", headless: true, args: ["--no-sandbox", "--disable-gpu"] });

async function scenario(failures) {
  const context = await browser.newContext({
    viewport: { width: 390, height: 844 },
    userAgent: "Mozilla/5.0 (Linux; Android 14) AppleWebKit/537.36 Chrome/131.0.0.0 Mobile Safari/537.36",
  });
  await context.addInitScript({ content: `
    (${installDualInterfaceFixture.toString()})();
    window.startupAttempts = 0;
    window.failStartup = ${failures};
    const query = window.__kazeFixture.query.bind(window.__kazeFixture);
    window.__kazeFixture.query = async q => {
      if (q.query === 'GetSettings' && ++window.startupAttempts <= window.failStartup)
        throw Error('test: startup settings unavailable');
      return query(q);
    };
    window.sawStartupRetry = false;
    new MutationObserver(() => {
      if (document.querySelector('.app-startup-retry')) window.sawStartupRetry = true;
    }).observe(document, {childList: true, subtree: true});
  ` });
  await context.route("**/src/services/api.ts*", route => route.fulfill({
    contentType: "text/javascript",
    body: "export const isTauri=()=>false;export const executeQuery=q=>window.__kazeFixture.query(q);export const dispatchCommand=c=>window.__kazeFixture.command(c);export const subscribeBackendEvents=async cb=>{window.__kazeFixture.listeners.add(cb);return()=>window.__kazeFixture.listeners.delete(cb);};",
  }));
  const page = await context.newPage();
  const errors = [];
  page.on("pageerror", error => errors.push(error.message));
  try {
    await page.goto(base, { waitUntil: "domcontentloaded" });
    if (failures > 3) {
      await page.getByRole("button", { name: "Try again", exact: true }).waitFor();
      assert.equal(await page.evaluate(() => window.startupAttempts), 4);
      await page.waitForTimeout(1200);
      assert.equal(await page.evaluate(() => window.startupAttempts), 4, "exhausted startup cannot poll forever");
      await page.evaluate(() => { window.failStartup = 0; });
      await page.getByRole("button", { name: "Try again", exact: true }).click();
    }
    await page.locator(".m-shell").waitFor();
    await page.locator(".m-startup").waitFor({ state: "detached" });
    await page.getByRole("button", { name: "01 HOME", exact: true }).waitFor();
    await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    assert.equal(await page.locator(".app-startup-retry").count(), 0);
    assert.equal(await page.evaluate(() => document.documentElement.dataset.interface), "mobile");
    assert.equal(await page.evaluate(() => window.sawStartupRetry), failures > 3);
    assert.equal(await page.evaluate(() => window.startupAttempts), failures > 3 ? 5 : failures + 1);
    assert.deepEqual(errors, []);
    await page.screenshot({ path: `${output}/${failures > 3 ? "manual" : "automatic"}-recovery.png` });
    console.log(`PASS Android startup: ${failures > 3 ? "bounded exhaustion and manual retry" : "automatic recovery after two transient failures"}`);
  } finally { await context.close(); }
}

try {
  await scenario(2);
  await scenario(100);
} finally { await browser.close(); }
