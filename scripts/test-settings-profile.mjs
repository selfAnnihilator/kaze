/** Real App regression: Settings entry loading and the profile/photo shortcut. */
import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import { installDualInterfaceFixture } from "../tests/fixtures/dualInterfaceBackend.mjs";

const { chromium } = await import(process.env.KAZE_PLAYWRIGHT_MODULE || "/home/abhi/.cache/ms-playwright-go/1.57.0/package/index.mjs");
const base = process.env.KAZE_TEST_URL || "http://127.0.0.1:5194";
const out = "/tmp/kaze-settings-profile";
await mkdir(out, { recursive: true });
const browser = await chromium.launch({ executablePath: process.env.KAZE_CHROMIUM || "/usr/bin/chromium", headless: true, args: ["--no-sandbox"] });
try {
  for (const style of ["spatial", "brutalist"]) {
    const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
    await context.addInitScript({ content: `
      sessionStorage.setItem('test.interface-style', ${JSON.stringify(style)});
      (${installDualInterfaceFixture.toString()})();
      window.profileFixtureUser = {id:'u1', username:'listener', display_name:'Music Listener', created_at:1700000000, avatar_data_url:'/design-lab/artwork/dummy.jpg'};
      const query = window.__kazeFixture.query.bind(window.__kazeFixture);
      window.__kazeFixture.query = q => q.query === 'GetCurrentUser'
        ? Promise.resolve({type:'CurrentUser',data:window.profileFixtureUser}) : query(q);
    ` });
    await context.route("**/src/services/api.ts*", route => route.fulfill({ contentType: "text/javascript", body: `export const isTauri=()=>false;export const executeQuery=q=>window.__kazeFixture.query(q);export const dispatchCommand=c=>window.__kazeFixture.command(c);export const subscribeBackendEvents=async cb=>{window.__kazeFixture.listeners.add(cb);return()=>window.__kazeFixture.listeners.delete(cb);};` }));
    const page = await context.newPage(), errors = [];
    let settingsLoads = 0, lateSettingsLoads = 0;
    page.on("pageerror", error => errors.push(error.message));
    page.on("request", request => { if (request.url().includes("/views/SettingsView.tsx")) settingsLoads++; });
    try {
      await page.goto(base);
      await page.locator(".home-flow").waitFor();
      assert.ok(settingsLoads > 0, "Settings must load with App, before navigation");
      // This caused the original lazy route to reject and unmount the whole App.
      await context.route("**/src/components/views/SettingsView.tsx*", route => { lateSettingsLoads++; return route.abort("failed"); });
      const sidebar = page.locator(".sidebar, .press-account-navigation");
      const settings = sidebar.getByRole("button", { name: "Settings", exact: true });
      const profile = sidebar.getByRole("button", { name: "Profile", exact: true });
      await page.waitForFunction(() => document.querySelector(".profile-navigation img")?.naturalWidth > 0);
      assert.equal(await profile.getAttribute("title"), "Profile and stats");
      assert.equal(await profile.count(), 1);
      const settingsBox = await settings.boundingBox(), profileBox = await profile.boundingBox();
      assert.ok(profileBox.y >= settingsBox.y + settingsBox.height, "Profile sits below Settings");
      const playback = await page.evaluate(() => JSON.stringify(window.__kazeFixture.playback));
      await page.evaluate(() => { window.settingsAppIdentity = document.querySelector(".kaze-interface"); });
      await settings.click();
      await page.getByRole("heading", { name: "Settings", exact: true }).waitFor();
      for (const name of ["Music Folders", "System & Storage"]) {
        await page.getByRole("button", { name: new RegExp(name) }).click();
        await page.getByRole("heading", { name, exact: true }).waitFor();
        await page.getByRole("button", { name: "Back to Settings", exact: true }).click();
      }
      for (const name of ["Audio Engine", "Metadata Providers", "Soulseek / Slskd", "Cloud & Sync"])
        assert.equal(await page.getByRole("button", { name, exact: true }).count(), 0);
      assert.equal(await page.getByRole("button", { name: "Log Out", exact: true }).count(), 1);
      assert.equal(lateSettingsLoads, 0);
      assert.equal(await page.evaluate(() => window.settingsAppIdentity === document.querySelector(".kaze-interface")), true);
      console.log(`PASS ${style}: Settings opens all sections without a delayed module request or blanking App`);

      await profile.press("Enter");
      await page.getByRole("heading", { name: "listener", exact: true }).waitFor();
      await page.getByText("Daily Music Time", { exact: true }).waitFor();
      assert.equal(await page.getByRole("button", { name: "Log Out", exact: true }).count(), 0);
      assert.equal(await profile.getAttribute("aria-current"), "page");
      assert.equal(await page.evaluate(() => JSON.stringify(window.__kazeFixture.playback)), playback);
      await page.screenshot({ path: `${out}/${style}-profile.png` });
      console.log(`PASS ${style}: photo shortcut opens profile and listening statistics without changing playback`);

      await page.evaluate(() => window.__kazeFixture.emit({ event: "UserProfileUpdated", payload: { user: { ...window.profileFixtureUser, avatar_data_url: "data:image/png;base64,broken" } } }));
      await page.waitForFunction(() => !document.querySelector(".profile-navigation img") && document.querySelector(".profile-navigation-avatar").textContent === "M");
      await page.evaluate(() => window.__kazeFixture.emit({ event: "UserProfileUpdated", payload: { user: window.profileFixtureUser } }));
      await page.waitForFunction(() => document.querySelector(".profile-navigation img")?.naturalWidth > 0);
      await page.setViewportSize({ width: 1024, height: 600 });
      const compactBox = await profile.boundingBox();
      assert.ok(compactBox.y >= 0 && compactBox.y + compactBox.height <= 600);
      await page.evaluate(() => window.__kazeFixture.emit({ event: "SessionChanged", payload: { user: null } }));
      await page.waitForFunction(() => !!document.querySelector(".profile-navigation-avatar svg"));
      await profile.click();
      await page.getByRole("heading", { name: "Guest Listener", exact: true }).waitFor();
      await page.getByText("Daily Music Time", { exact: true }).waitFor();
      assert.deepEqual(errors, []);
      console.log(`PASS ${style}: live photo updates, broken-photo initials and signed-out fallback remain usable`);
    } finally {
      await page.screenshot({ path: `${out}/${style}-last.png` });
      await context.close();
    }
  }
} finally {
  await browser.close();
}
