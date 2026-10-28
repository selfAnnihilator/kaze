/** Compare the independent production composition with the untouched approved study. */
import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import { installDualInterfaceFixture } from "../tests/fixtures/dualInterfaceBackend.mjs";
const { chromium } = await import(
  process.env.KAZE_PLAYWRIGHT_MODULE ||
    "/home/abhi/.cache/ms-playwright-go/1.57.0/package/index.mjs"
);
const base = process.env.KAZE_TEST_URL || "http://127.0.0.1:5194";
const out = "/tmp/kaze-brutalist-restored";
await mkdir(out, { recursive: true });
const browser = await chromium.launch({
  executablePath: process.env.KAZE_CHROMIUM || "/usr/bin/chromium",
  headless: true,
  args: ["--no-sandbox"],
});
const context = await browser.newContext({
  viewport: { width: 1440, height: 900 },
});
await context.addInitScript({
  content: `sessionStorage.setItem('test.interface-style','brutalist');(${installDualInterfaceFixture.toString()})();`,
});
await context.route("**/src/services/api.ts*", (route) =>
  route.fulfill({
    contentType: "text/javascript",
    body: `export const isTauri=()=>false;export const executeQuery=q=>window.__kazeFixture.query(q);export const dispatchCommand=c=>window.__kazeFixture.command(c);export const subscribeBackendEvents=async cb=>{window.__kazeFixture.listeners.add(cb);return()=>window.__kazeFixture.listeners.delete(cb);};`,
  }),
);
const page = await context.newPage(),
  reference = await context.newPage(),
  errors = [],
  evidence = [];
page.on("pageerror", (error) => errors.push(error.message));
reference.on("pageerror", (error) => errors.push(error.message));
page.setDefaultTimeout(10000);
reference.setDefaultTimeout(10000);
const rect = (target, selector) =>
  target.locator(selector).first().boundingBox();
const close = (actual, expected, keys, label) => {
  for (const key of keys)
    assert.ok(
      Math.abs(actual[key] - expected[key]) <= 2,
      `${label}.${key}: ${actual[key]} vs ${expected[key]}`,
    );
};
const nav = (name) =>
  page
    .locator(".press-header,.press-account-navigation,.press-deck-controls")
    .getByRole("button", { name, exact: true })
    .click();
try {
  await page.goto(base);
  await page.locator(".press-home").waitFor();
  await reference.goto(`${base}/?lab=brutalist`);
  await reference.locator(".bk-listen").waitFor();
  await Promise.all([
    page.evaluate(() => document.fonts.ready),
    reference.evaluate(() => document.fonts.ready),
  ]);
  for (const [width, height] of [
    [1920, 1080],
    [1440, 900],
    [1024, 600],
    [800, 700],
  ]) {
    await page.setViewportSize({ width, height });
    await reference.setViewportSize({ width, height });
    await nav("Open Now Playing");
    await reference.locator(".bk-header nav button").nth(0).click();
    await page.waitForTimeout(100);
    const selectors = [
      [".press-header", ".bk-header"],
      [".press-wordmark", ".bk-wordmark"],
      [".press-deck", ".bk-deck"],
      [".interface-toolbar", ".bk-toolbar"],
      [".press-poster-cover", ".bk-poster-cover"],
      [".press-ticket-row", ".bk-ticket-row"],
    ];
    const measurements = [];
    for (const [production, study] of selectors) {
      const actual = await rect(page, production),
        expected = await rect(reference, study);
      close(actual, expected, ["x", "y", "width", "height"], production);
      measurements.push({ production, actual, expected });
    }
    assert.equal(
      await page
        .locator(
          ".sidebar,.player-bar,.spatial-navigation,.spatial-home,.spatial-discovery-fixed",
        )
        .count(),
      0,
    );
    assert.equal(await page.locator(".press-header nav button").count(), 4);
    assert.equal(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth + 1,
      ),
      true,
    );
    const profile = await rect(
        page,
        ".press-account-navigation .profile-navigation",
      ),
      settings = await rect(
        page,
        ".press-account-navigation [aria-label=Settings]",
      );
    assert.ok(
      profile.y >= settings.y + settings.height &&
        profile.y + profile.height <= height,
    );
    await page.screenshot({ path: `${out}/poster-${width}.png` });
    await reference.screenshot({
      path: `${out}/reference-poster-${width}.png`,
    });
    await nav("Discover Music");
    await reference.locator(".bk-header nav button").nth(1).click();
    await page.locator(".press-discover").waitFor();
    await page.waitForTimeout(100);
    close(
      await rect(page, ".press-discover .press-view-heading"),
      await rect(reference, ".bk-discover .bk-view-heading"),
      ["x", "y", "width", "height"],
      "Discovery heading",
    );
    const sleeveTitle = await rect(page, ".discovery-card-title");
    assert.ok(
      sleeveTitle.height > 15,
      "sleeve titles must not collapse under the cover",
    );
    if (height >= 900) {
      const volume = await rect(page, '.press-deck input[aria-label="Volume"]');
      const account = await rect(page, ".press-account-navigation");
      assert.ok(
        volume.y >= 0 && volume.y + volume.height <= account.y,
        "volume remains visible above account controls",
      );
    }
    const fixedHeading = await rect(
      page,
      ".press-discover .press-view-heading",
    );
    const wheelArea = await rect(page, ".press-discover .chapter-stage");
    await page.mouse.move(
      wheelArea.x + 80,
      wheelArea.y + Math.min(60, wheelArea.height / 2),
    );
    await page.mouse.wheel(0, 120);
    await page.waitForFunction(
      () => document.querySelector(".press-bin-controls select")?.value === "1",
    );
    close(
      await rect(page, ".press-discover .press-view-heading"),
      fixedHeading,
      ["x", "y", "width", "height"],
      "Wheel keeps the heading fixed",
    );
    assert.equal(
      await page.locator(".main-content").evaluate((el) => el.scrollTop),
      0,
    );
    assert.equal(
      await page.locator(".press-discover").evaluate((el) => el.scrollTop),
      0,
    );
    await page.waitForTimeout(210);
    await page.mouse.wheel(0, -120);
    await page.waitForFunction(
      () => document.querySelector(".press-bin-controls select")?.value === "0",
    );
    assert.equal(
      await page.locator(".discovery-chapter").count(),
      1,
      "only the selected real shelf mounts",
    );
    assert.ok((await page.locator(".discovery-track-card").count()) < 20);
    await page.screenshot({ path: `${out}/discovery-${width}.png` });
    await nav("Library");
    await reference.locator(".bk-header nav button").nth(2).click();
    await page.locator(".press-record-spine").first().waitFor();
    close(
      await rect(page, ".press-tabs"),
      await rect(reference, ".bk-tabs"),
      ["x", "y", "height"],
      "Library tabs",
    );
    // Production's fixed, scrollbar-free archive uses the full pane; the
    // original study still reserves a 15px vertical scrollbar gutter.
    close(await rect(page, ".press-tabs"), await rect(page, ".interface-toolbar"), ["width"], "Library tabs fill the pane");
    assert.ok(
      (await page.locator(".press-record-spine").count()) < 30,
      "130 real albums stay windowed",
    );
    await page.bringToFront();
    const shelf = page.locator(".press-albums .windowed-strip");
    await shelf.evaluate((el) => { el.scrollTop = 0; });
    const shelfBefore = await rect(page, ".press-albums .windowed-strip");
    const footerBefore = await rect(page, ".press-archive-footer");
    const spineBefore = await rect(page, ".press-record-spine");
    await page.mouse.move(shelfBefore.x + 50, shelfBefore.y + Math.min(40, shelfBefore.height / 2));
    await page.mouse.wheel(0, 250);
    await page.waitForTimeout(220);
    close(await rect(page, ".press-albums .windowed-strip"), shelfBefore, ["x", "y", "width", "height"], "Album viewport stays fixed");
    close(await rect(page, ".press-archive-footer"), footerBefore, ["x", "y", "width", "height"], "Album footer stays fixed");
    assert.equal(await page.locator(".press-albums").evaluate((el) => el.scrollTop), 0);
    assert.ok(footerBefore.y + footerBefore.height <= height, "Album arrows remain visible");
    if (height >= 900) {
      close(await rect(page, ".press-record-spine"), spineBefore, ["x", "y", "width", "height"], "Album spines do not nudge upward");
      assert.equal(await shelf.evaluate((el) => el.scrollTop), 0);
    }
    const leftBefore = await shelf.evaluate((el) => el.scrollLeft);
    await page.getByRole("button", { name: "Later records", exact: true }).click();
    await page.waitForFunction((before) => document.querySelector(".press-albums .windowed-strip").scrollLeft > before, leftBefore);
    await shelf.evaluate((el) => { el.scrollTop = 0; });
    await page
      .locator(".press-albums .windowed-strip")
      .evaluate((el) => (el.scrollLeft = el.scrollWidth));
    await page.waitForFunction(
      () =>
        [...document.querySelectorAll(".press-record-spine > span")].at(-1)
          ?.textContent === "130",
    );
    assert.equal(
      await page.locator(".press-record-spine > span").last().textContent(),
      "130",
    );
    await page.screenshot({ path: `${out}/library-${width}.png` });
    evidence.push({ width, height, measurements });
    console.log(
      `PASS ${width}×${height}: reference shell, record poster, Discovery heading, Library tabs and bounded record spines`,
    );
  }
  await page.setViewportSize({ width: 1440, height: 900 });
  await nav("Library");
  await page.getByRole("tab", { name: "Playlists", exact: true }).click();
  assert.equal(
    await page.locator(".press-playlist-number").textContent(),
    "01",
  );
  assert.equal(
    await page
      .locator(".playlist-card")
      .filter({ hasText: "Liked Songs" })
      .count(),
    0,
  );
  const playlistRow = await rect(page, ".playlist-card"),
    playlistHeading = await rect(page, ".playlists-view > .view-header");
  close(playlistRow, playlistHeading, ["x", "width"], "Aligned playlist index");
  await nav("Open Now Playing");
  await page
    .locator(".press-deck-controls")
    .getByRole("button", { name: "Full Screen", exact: true })
    .click();
  await page.locator(".press-fullscreen .press-poster-cover").waitFor();
  assert.equal(
    await page
      .locator(
        ".press-fullscreen .playback-stage,.press-fullscreen .player-bar",
      )
      .count(),
    0,
  );
  await page.locator(".press-fullscreen .press-deck").waitFor();
  await page.screenshot({ path: `${out}/fullscreen.png` });
  await page.keyboard.press("Escape");
  await page.locator(".fullscreen-player").waitFor({ state: "detached" });
  console.log(
    "PASS aligned numbered playlists, Liked excluded from custom playlists and independent full screen poster/deck",
  );
  await nav("Discover Music");
  const discovery = page.locator(".press-discover");
  const headingBeforeWheel = await rect(
    page,
    ".press-discover .press-view-heading",
  );
  const bodyBeforeWheel = await rect(page, ".press-discover");
  const wheelTarget = page.locator(".press-discover .windowed-strip");
  for (const expected of [1, 2, 3, 3, 2, 1, 0, 0]) {
    const current = Number(
      await page
        .getByRole("combobox", { name: "Discovery selection" })
        .inputValue(),
    );
    const delta =
      expected < current || (expected === 0 && current === 0) ? -90 : 90;
    await page.waitForTimeout(210);
    const target = (await wheelTarget.count()) ? wheelTarget : discovery;
    await target.evaluate((el, deltaY) => {
      for (let burst = 0; burst < 8; burst++)
        el.dispatchEvent(
          new WheelEvent("wheel", { deltaY, bubbles: true, cancelable: true }),
        );
    }, delta);
    await page.waitForFunction(
      (value) =>
        document.querySelector(".press-bin-controls select")?.value ===
        String(value),
      expected,
    );
    close(
      await rect(page, ".press-discover .press-view-heading"),
      headingBeforeWheel,
      ["x", "y", "width", "height"],
      "Fixed Discovery heading",
    );
    close(
      await rect(page, ".press-discover"),
      bodyBeforeWheel,
      ["x", "y", "width", "height"],
      "Fixed Discovery stage",
    );
    assert.equal(
      await page.locator(".main-content").evaluate((el) => el.scrollTop),
      0,
    );
    assert.equal(await discovery.evaluate((el) => el.scrollTop), 0);
  }
  await page.bringToFront();
  const horizontalShelf = page.locator(".press-discover .windowed-strip");
  const horizontalBox = await horizontalShelf.boundingBox();
  await page.mouse.move(horizontalBox.x + 100, horizontalBox.y + 80);
  await page.mouse.wheel(400, 0);
  await page.waitForTimeout(300);
  assert.ok(
    (await horizontalShelf.evaluate((el) => el.scrollLeft)) > 100,
    "horizontal browsing remains native",
  );
  assert.equal(
    await page
      .getByRole("combobox", { name: "Discovery selection" })
      .inputValue(),
    "0",
  );
  await horizontalShelf.evaluate((el) =>
    el.scrollTo({ left: 0, behavior: "instant" }),
  );
  console.log(
    "PASS vertical wheel changes one bin in place per gesture, clamps at both ends and preserves horizontal browsing",
  );
  const before = await page.evaluate(
    () => window.__kazeFixture.commands.length,
  );
  for (let i = 0; i < 4; i++) {
    await page
      .getByRole("combobox", { name: "Discovery selection" })
      .selectOption(String(i));
    assert.equal(
      await page
        .locator(".discovery-chapter")
        .getAttribute("data-chapter-index"),
      String(i),
    );
  }
  await page.getByRole("button", { name: "Next discovery chapter" }).click();
  assert.equal(
    await page.locator(".discovery-chapter").getAttribute("data-chapter-index"),
    "0",
  );
  const wall = page.locator(".press-discover .windowed-strip");
  const box = await wall.boundingBox();
  await page.mouse.move(box.x + 300, box.y + 100);
  await page.mouse.down();
  await page.mouse.move(box.x + 50, box.y + 100, { steps: 12 });
  await page.mouse.up();
  assert.ok((await wall.evaluate((el) => el.scrollLeft)) > 150);
  assert.equal(
    await page.evaluate(() => window.__kazeFixture.commands.length),
    before,
    "changing bins and dragging never plays a track",
  );
  console.log(
    "PASS independent Discovery selection, wrapped chapter arrows and native sleeve dragging",
  );
  await nav("Home");
  const homeFlow = page.locator(".home-flow");
  await homeFlow
    .getByRole("button", { name: "Pause Flow", exact: true })
    .click();
  await homeFlow
    .getByRole("button", { name: "Start Flow", exact: true })
    .click();
  await page
    .locator(".press-deck-controls")
    .getByRole("button", { name: "Pause Flow", exact: true })
    .click();
  await page
    .locator(".press-deck-controls")
    .getByRole("button", { name: "Start Flow", exact: true })
    .click();
  assert.ok(await page.getByRole("slider", { name: "Flow Steering" }).count());
  console.log(
    "PASS actual Home and deck Flow actions with the thick familiarity bar",
  );
  const homeArtwork = page.locator(".press-home-artwork");
  await homeArtwork.locator(".press-home-next").click();
  await page.waitForFunction(() => document.querySelector(".press-home-previous")?.title === "Previous: Kerala");
  assert.deepEqual(
    await homeArtwork.locator("button").evaluateAll(elements => elements.map(element => element.title)),
    ["Previous: Kerala", "Current: Feather", "Next: Friday Morning"],
  );
  const [previousCover, currentCover, nextCover] = await Promise.all(
    [".press-home-previous", ".press-poster-cover", ".press-home-next"].map(selector => rect(page, `.press-home-artwork ${selector}`)),
  );
  assert.ok(previousCover.x < currentCover.x && currentCover.x < nextCover.x);
  assert.ok(previousCover.width < currentCover.width && nextCover.width < currentCover.width);
  await homeArtwork.locator(".press-home-previous").focus();
  await page.keyboard.press("Enter");
  await page.waitForFunction(() => document.querySelector(".press-home-artwork .press-poster-cover")?.title === "Current: Kerala");
  await page.evaluate(() => window.__kazeFixture.command({ command: "ClearQueue" }));
  await page.waitForFunction(() => document.querySelector(".press-home-next")?.disabled);
  assert.equal(await homeArtwork.locator(".press-home-next .music-artwork svg").count(), 1);
  assert.deepEqual(await homeArtwork.locator("button").evaluateAll(elements => elements.map(element => getComputedStyle(element).opacity)), ["1", "1", "1"]);
  console.log("PASS Home previous/current/next identities, smaller side covers, transport actions and opaque placeholders");
  await page.setViewportSize({ width: 390, height: 844 });
  await nav("Home");
  assert.equal(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth + 1,
    ),
    true,
  );
  await page.locator(".press-mobile-track a").click();
  await page
    .locator(".press-account-navigation")
    .getByRole("button", { name: "Profile", exact: true })
    .click();
  await page.getByText("Daily Music Time", { exact: true }).waitFor();
  await page.screenshot({ path: `${out}/narrow.png` });
  console.log(
    "PASS narrow preview composition, compact deck link and account access",
  );
  assert.deepEqual(errors, []);
  await writeFile(`${out}/geometry.json`, JSON.stringify(evidence, null, 2));
} finally {
  await page.screenshot({ path: `${out}/last.png` });
  await browser.close();
}
