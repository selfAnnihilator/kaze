/** Focused Settings flows with disposable IPC data; no native account or files touched. */
import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import { installDualInterfaceFixture } from "../tests/fixtures/dualInterfaceBackend.mjs";
const { chromium } = await import(
  process.env.KAZE_PLAYWRIGHT_MODULE ||
    "/home/abhi/.cache/ms-playwright-go/1.57.0/package/index.mjs"
);
const browser = await chromium.launch({
  executablePath: process.env.KAZE_CHROMIUM || "/usr/bin/chromium",
  headless: true,
  args: ["--no-sandbox"],
});
const out = "/tmp/kaze-settings-simple";
await mkdir(out, { recursive: true });
try {
  for (const style of ["spatial", "brutalist"]) {
    const context = await browser.newContext({
      viewport: { width: 1888, height: 1004 },
    });
    await context.addInitScript({
      content: `sessionStorage.setItem('test.interface-style',sessionStorage.getItem('test.interface-style')||'${style}');(${installDualInterfaceFixture.toString()})();`,
    });
    await context.addInitScript(() => {
      const f = window.__kazeFixture,
        query = f.query.bind(f),
        command = f.command.bind(f);
      f.folders = [{ id: "f1", path: "/fixture/music", track_count: 1036 }];
      f.cache = {
        total_size_bytes: 256 * 1024 ** 2,
        file_count: 50,
        max_size_bytes: 1024 ** 3,
        partial_file_count: 1,
      };
      f.failCommand = null;
      f.query = async (q) => {
        if (q.query === "GetCurrentUser")
          return {
            type: "CurrentUser",
            data: sessionStorage.getItem("logged-out")
              ? null
              : {
                  id: "u1",
                  username: "listener",
                  display_name: "Music Listener",
                  created_at: 1,
                },
          };
        if (q.query === "GetOnboardingStatus")
          return {
            type: "OnboardingStatus",
            completed: true,
            default_music_dir: "/fixture",
            configured_folders: f.folders,
          };
        if (q.query === "GetRemoteAudioCacheStats") {
          if (f.failCacheQuery) throw Error("Fixture storage unavailable");
          return { type: "RemoteAudioCacheStats", data: f.cache };
        }
        return query(q);
      };
      f.command = async (c) => {
        if (f.failCommand === c.command) {
          f.commands.push(c);
          throw Error("Fixture failure");
        }
        if (
          f.deferLogout &&
          (c.command === "Logout" || c.command === "LogoutAll")
        ) {
          f.logoutAttempts = (f.logoutAttempts || 0) + 1;
          await new Promise((resolve) => {
            f.releaseLogout = resolve;
          });
        }
        const result = await command(c),
          p = c.payload || {};
        if (c.command === "SetCloseButtonBehavior")
          f.settings.close_button_behavior = p.behavior;
        if (c.command === "AddLibraryFolder")
          f.folders.push({ id: "added", path: p.path, track_count: 0 });
        if (c.command === "RemoveLibraryFolder")
          f.folders = f.folders.filter((folder) => folder.id !== p.folder_id);
        if (c.command === "ClearRemoteAudioCache")
          f.cache = { ...f.cache, total_size_bytes: 0, file_count: 0 };
        if (c.command === "ScanLibrary")
          setTimeout(
            () =>
              f.emit({
                event: "LibraryScanCompleted",
                payload: { tracks_found: 1036 },
              }),
            40,
          );
        if (c.command === "Logout" || c.command === "LogoutAll") {
          sessionStorage.setItem("logged-out", "true");
          sessionStorage.setItem("last-logout-command", c.command);
        }
        return result;
      };
    });
    await context.route("**/src/services/api.ts*", (route) =>
      route.fulfill({
        contentType: "text/javascript",
        body: `export const isTauri=()=>false;export const executeQuery=q=>window.__kazeFixture.query(q);export const dispatchCommand=c=>window.__kazeFixture.command(c);export const subscribeBackendEvents=async cb=>{window.__kazeFixture.listeners.add(cb);return()=>window.__kazeFixture.listeners.delete(cb)};`,
      }),
    );
    const page = await context.newPage(),
      errors = [];
    page.on("pageerror", (error) => errors.push(error.message));
    page.setDefaultTimeout(10000);
    await page.goto(process.env.KAZE_TEST_URL || "http://127.0.0.1:5194");
    const nav = (name) =>
      page
        .locator(".sidebar,.press-account-navigation")
        .getByRole("button", { name, exact: true })
        .click();
    const root = page.locator(".settings-view");
    const main = () =>
      page
        .getByRole("button", { name: "Back to Settings", exact: true })
        .click();
    const section = (name) =>
      root.getByRole("button", { name: new RegExp(`^${name}`) }).click();
    await nav("Settings");
    await root
      .getByRole("heading", { name: "Settings", exact: true })
      .waitFor();
    const playback = await page.evaluate(() =>
      JSON.stringify(window.__kazeFixture.playback),
    );
    for (const [width, height] of [
      [1888, 1004],
      [1440, 900],
      [1024, 600],
      [800, 700],
      [390, 844],
    ]) {
      await page.setViewportSize({ width, height });
      const choices = root.locator(".settings-style-options > button");
      const a = await choices.nth(0).boundingBox(),
        b = await choices.nth(1).boundingBox();
      assert.ok(b.y >= a.y + a.height, "style choices follow top to bottom");
      const content = await root.boundingBox();
      assert.ok(
        content.x >= 0 && content.x + content.width <= width + 1,
        `settings in bounds ${width}`,
      );
      assert.ok(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= innerWidth + 1,
        ),
      );
      for (const selector of [
        "#settings-close-behavior",
        ".settings-section-link",
        ".settings-logout",
      ]) {
        const target = root.locator(selector).first();
        await target.scrollIntoViewIfNeeded();
        const box = await target.boundingBox();
        assert.ok(box.x >= 0 && box.x + box.width <= width + 1);
      }
      if (width === 1888 || width === 1024) {
        await page
          .locator(".main-content")
          .evaluate((el) => el.scrollTo({ top: 0, behavior: "instant" }));
        await page.screenshot({ path: `${out}/${style}-${width}.png` });
      }
      console.log(
        `PASS ${style} ${width}: vertical Settings, controls reachable, no horizontal overflow`,
      );
    }
    await page.setViewportSize({ width: 1440, height: 900 });
    for (const label of [
      "Audio Engine",
      "Metadata Providers",
      "Soulseek / Slskd",
      "Cloud & Sync",
    ])
      assert.equal(await root.getByText(label, { exact: true }).count(), 0);
    assert.equal(
      await root
        .getByText(/SQLite|PBKDF2|LRU|Daemon|worker|Auto-evicts/)
        .count(),
      0,
    );
    const close = root.getByRole("combobox", {
      name: "Window Close Behavior",
      exact: true,
    });
    await close.selectOption("quit_app");
    await page.waitForFunction(
      () => window.__kazeFixture.settings.close_button_behavior === "quit_app",
    );
    await page.evaluate(
      () => (window.__kazeFixture.failCommand = "SetCloseButtonBehavior"),
    );
    await close.selectOption("minimize_to_tray");
    await root
      .getByRole("alert")
      .filter({ hasText: "Couldn't save" })
      .waitFor();
    assert.equal(await close.inputValue(), "quit_app");
    await page.evaluate(() => (window.__kazeFixture.failCommand = null));
    await close.selectOption("minimize_to_tray");
    await page.waitForFunction(
      () =>
        window.__kazeFixture.settings.close_button_behavior ===
        "minimize_to_tray",
    );
    console.log(
      `PASS ${style}: close behavior saves and retains prior value on failure`,
    );
    await section("Music Folders");
    await root
      .getByRole("heading", { name: "Music Folders", exact: true })
      .waitFor();
    assert.equal(
      await root.getByRole("group", { name: "Interface Style" }).count(),
      0,
      "detail replaces outer page",
    );
    const path = root.getByLabel("Folder path", { exact: true });
    await path.fill("/fixture/new-music");
    await page.evaluate(
      () => (window.__kazeFixture.failCommand = "AddLibraryFolder"),
    );
    await root.getByRole("button", { name: "Add Folder", exact: true }).click();
    await root.getByRole("alert").filter({ hasText: "Couldn't add" }).waitFor();
    assert.equal(await path.inputValue(), "/fixture/new-music");
    await page.evaluate(() => (window.__kazeFixture.failCommand = null));
    await root.getByRole("button", { name: "Add Folder", exact: true }).click();
    await root.getByText("/fixture/new-music", { exact: true }).waitFor();
    await root
      .getByRole("button", {
        name: "Remove folder /fixture/new-music",
        exact: true,
      })
      .click();
    await root
      .getByText("/fixture/new-music", { exact: true })
      .waitFor({ state: "detached" });
    await page.evaluate(
      () => (window.__kazeFixture.failCommand = "ScanLibrary"),
    );
    await root
      .getByRole("button", { name: "Rescan Library", exact: true })
      .click();
    await root
      .getByRole("alert")
      .filter({ hasText: "Couldn't scan" })
      .waitFor();
    assert.equal(
      await root
        .getByRole("button", { name: "Rescan Library", exact: true })
        .isEnabled(),
      true,
    );
    await page.evaluate(() => (window.__kazeFixture.failCommand = null));
    await root
      .getByRole("button", { name: "Rescan Library", exact: true })
      .click();
    await root
      .getByRole("button", { name: "Rescan Library", exact: true })
      .waitFor();
    await page.evaluate(() => {
      const fixture = window.__kazeFixture;
      fixture.folders = Array.from({ length: 80 }, (_, i) => ({
        id: `folder-${i}`,
        path: `/fixture/music-${i}`,
        track_count: i,
      }));
      fixture.emit({
        event: "LibraryScanCompleted",
        payload: { tracks_found: 1036 },
      });
    });
    const folders = root.locator('[aria-label="Music folders"]');
    await folders.waitFor();
    await folders.evaluate((el) => (el.scrollTop = el.scrollHeight));
    await root.getByText("/fixture/music-79", { exact: true }).waitFor();
    assert.ok(
      (await root.locator(".settings-folder").count()) < 30,
      "large folder lists remain windowed",
    );
    await main();
    assert.equal(
      await page.locator(".main-content").evaluate((el) => el.scrollTop),
      0,
    );
    console.log(
      `PASS ${style}: dedicated folder section add/remove/rescan, failures recover`,
    );
    await section("System & Storage");
    await root.getByText("256.0 MB", { exact: true }).waitFor();
    await page.evaluate(
      () => (window.__kazeFixture.failCommand = "ClearRemoteAudioCache"),
    );
    await root
      .getByRole("button", { name: "Clear Cache", exact: true })
      .click();
    await root
      .getByRole("alert")
      .filter({ hasText: "Couldn't clear" })
      .waitFor();
    await page.evaluate(() => (window.__kazeFixture.failCommand = null));
    await root
      .getByRole("button", { name: "Clear Cache", exact: true })
      .click();
    await root.getByText("0.0 MB", { exact: true }).waitFor();
    await root
      .getByRole("status")
      .filter({ hasText: "Temporary audio cleared" })
      .waitFor();
    await page.screenshot({ path: `${out}/${style}-storage.png` });
    await nav("Profile");
    await page.getByText("Daily Music Time", { exact: true }).waitFor();
    await page.getByRole("button", { name: "Go back", exact: true }).click();
    await root
      .getByRole("heading", { name: "System & Storage", exact: true })
      .waitFor();
    for (const [width, height] of [
      [1024, 600],
      [390, 844],
    ]) {
      await page.setViewportSize({ width, height });
      const clear = root.getByRole("button", {
        name: "Clear Cache",
        exact: true,
      });
      await clear.scrollIntoViewIfNeeded();
      const box = await clear.boundingBox();
      assert.ok(box.x >= 0 && box.x + box.width <= width + 1);
      await root
        .getByRole("button", { name: "Back to Settings", exact: true })
        .scrollIntoViewIfNeeded();
      assert.ok(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= innerWidth + 1,
        ),
      );
    }
    await page.setViewportSize({ width: 1440, height: 900 });
    await main();
    await page.evaluate(() => (window.__kazeFixture.failCacheQuery = true));
    await section("System & Storage");
    await root
      .getByRole("alert")
      .filter({ hasText: "Couldn't load" })
      .waitFor();
    await main();
    await page.evaluate(() => (window.__kazeFixture.failCacheQuery = false));
    console.log(
      `PASS ${style}: cache usage/clear, query and command errors shown honestly`,
    );
    const other = style === "spatial" ? "Brutalist" : "Spatial";
    await root.getByRole("button", { name: new RegExp(`^${other}`) }).click();
    await page.waitForFunction(
      (style) => document.documentElement.dataset.interface === style,
      other.toLowerCase(),
    );
    await root
      .getByRole("button", {
        name: new RegExp(`^${style === "spatial" ? "Spatial" : "Brutalist"}`),
      })
      .click();
    await page.waitForFunction(
      (style) => document.documentElement.dataset.interface === style,
      style,
    );
    await nav("Profile");
    await page.getByText("Daily Music Time", { exact: true }).waitFor();
    assert.equal(
      await page.getByRole("button", { name: "Log Out", exact: true }).count(),
      0,
    );
    assert.equal(
      await page.evaluate(() => JSON.stringify(window.__kazeFixture.playback)),
      playback,
    );
    await nav("Settings");
    const dialog = page.getByRole("dialog", {
      name: "Where do you want to log out?",
      exact: true,
    });
    const trigger = root.getByRole("button", { name: "Log Out", exact: true });
    const choices = async () => {
      await trigger.click();
      await dialog.waitFor();
    };
    const initialCommands = await page.evaluate(
      () => window.__kazeFixture.commands.length,
    );
    for (const [width, height] of [
      [1440, 900],
      [1024, 600],
      [390, 844],
    ]) {
      await page.setViewportSize({ width, height });
      await choices();
      const box = await dialog.boundingBox();
      assert.ok(
        box.x >= 0 &&
          box.y >= 0 &&
          box.x + box.width <= width + 1 &&
          box.y + box.height <= height + 1,
      );
      await page.keyboard.press("Tab");
      assert.equal(
        await dialog.evaluate((el) => el.contains(document.activeElement)),
        true,
        "native dialog traps keyboard focus",
      );
      await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
      await dialog.waitFor({ state: "detached" });
      assert.equal(
        await trigger.evaluate((el) => el === document.activeElement),
        true,
        "focus returns to logout trigger",
      );
    }
    await choices();
    await page.keyboard.press("Escape");
    await dialog.waitFor({ state: "detached" });
    assert.equal(
      await page.evaluate(() => window.__kazeFixture.commands.length),
      initialCommands,
      "opening, Escape and Cancel never log out",
    );
    await page.setViewportSize({ width: 1440, height: 900 });
    for (const [scope, command] of [
      ["This device", "Logout"],
      ["All devices", "LogoutAll"],
    ]) {
      await page.evaluate(
        (command) => (window.__kazeFixture.failCommand = command),
        command,
      );
      await choices();
      await dialog
        .getByRole("button", { name: new RegExp(`^${scope}`) })
        .click();
      await dialog
        .getByRole("alert")
        .filter({ hasText: "Couldn't log out" })
        .waitFor();
      assert.equal(
        await dialog
          .getByRole("button", { name: "Cancel", exact: true })
          .isEnabled(),
        true,
      );
      await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
    }
    await page.evaluate(() => (window.__kazeFixture.failCommand = null));
    await choices();
    await Promise.all([
      page.waitForEvent("load"),
      dialog.getByRole("button", { name: /^This device/ }).click(),
    ]);
    assert.equal(
      await page.evaluate(() => sessionStorage.getItem("last-logout-command")),
      "Logout",
    );
    await nav("Settings");
    await root.getByRole("button", { name: "Sign In", exact: true }).click();
    await page
      .getByRole("heading", { name: "Welcome Back", exact: true })
      .waitFor();
    await page.evaluate(() => sessionStorage.removeItem("logged-out"));
    await page.reload();
    await nav("Settings");
    await page.evaluate(() => (window.__kazeFixture.deferLogout = true));
    await choices();
    await dialog.getByRole("button", { name: /^All devices/ }).click();
    await dialog
      .getByRole("status")
      .filter({ hasText: "Logging out" })
      .waitFor();
    assert.equal(
      await dialog.getByRole("button", { name: /^This device/ }).isDisabled(),
      true,
    );
    assert.equal(
      await dialog.getByRole("button", { name: /^All devices/ }).isDisabled(),
      true,
    );
    assert.equal(
      await dialog
        .getByRole("button", { name: "Cancel", exact: true })
        .isDisabled(),
      true,
    );
    await page.keyboard.press("Escape");
    assert.equal(
      await dialog.isVisible(),
      true,
      "pending logout cannot dismiss or send another request",
    );
    assert.equal(
      await page.evaluate(() => window.__kazeFixture.logoutAttempts),
      1,
    );
    await Promise.all([
      page.waitForEvent("load"),
      page.evaluate(() => window.__kazeFixture.releaseLogout()),
    ]);
    assert.equal(
      await page.evaluate(() => sessionStorage.getItem("last-logout-command")),
      "LogoutAll",
    );
    await nav("Settings");
    await root.getByRole("button", { name: "Sign In", exact: true }).waitFor();
    assert.deepEqual(errors, []);
    console.log(
      `PASS ${style}: style switching preserves playback; Profile has no logout; logout scope choice, Cancel/Escape, failure/retry, pending guards and guest sign-in work`,
    );
    await context.close();
  }
} finally {
  await browser.close();
}
