/** Keep the approved Spatial study's geometry while testing real feature adapters. */
import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import { installDualInterfaceFixture } from "../tests/fixtures/dualInterfaceBackend.mjs";
const { chromium } = await import(
  process.env.KAZE_PLAYWRIGHT_MODULE ||
    "/home/abhi/.cache/ms-playwright-go/1.57.0/package/index.mjs"
);
const out = "/tmp/kaze-preview-parity";
await mkdir(out, { recursive: true });
const browser = await chromium.launch({
  executablePath: process.env.KAZE_CHROMIUM || "/usr/bin/chromium",
  headless: true,
  args: ["--no-sandbox"],
});
const context = await browser.newContext();
await context.addInitScript(installDualInterfaceFixture);
await context.route("**/src/services/api.ts*", (route) =>
  route.fulfill({
    contentType: "text/javascript",
    body: `export const isTauri=()=>false;export const executeQuery=q=>window.__kazeFixture.query(q);export const dispatchCommand=c=>window.__kazeFixture.command(c);export const subscribeBackendEvents=async cb=>{window.__kazeFixture.listeners.add(cb);return ()=>window.__kazeFixture.listeners.delete(cb);};`,
  }),
);
const page = await context.newPage();
const errors = [];
page.on("pageerror", (error) => errors.push(error.message));
const base = process.env.KAZE_TEST_URL || "http://127.0.0.1:5194";
const geometry = (selectors) =>
  page.evaluate(
    (selectors) =>
      selectors.map((selector) => {
        const el = document.querySelector(selector),
          r = el.getBoundingClientRect();
        return { x: r.x, y: r.y, width: r.width, height: r.height };
      }),
    selectors,
  );
const nav = async (name) => {
  const button = page
    .locator(".sidebar")
    .getByRole("button", { name, exact: true });
  if (await button.count()) return button.click();
  await page
    .getByRole("button", { name: "Library and more", exact: true })
    .click();
  await page.getByRole("menuitem", { name, exact: true }).click();
};
const steering = () =>
  page.evaluate(() =>
    window.__kazeFixture.commands.filter(
      (command) => command.command === "SetFlowSteering",
    ),
  );
try {
  for (const [width, height] of [
    [1920, 1080],
    [1440, 900],
    [1024, 600],
    [800, 700],
  ]) {
    await page.setViewportSize({ width, height });
    await page.goto(`${base}/?lab=spatial`);
    await page.locator(".sp-flow").waitFor();
    await page.evaluate(() => document.fonts.ready);
    const reference = await geometry([
      ".sp-flow",
      ".sp-flow-art",
      ".sp-flow-front",
      ".sp-rail",
      ".sp-topbar",
    ]);
    await page.screenshot({ path: `${out}/reference-home-${width}.png` });
    await page.goto(base);
    await page.locator(".spatial-flow").waitFor();
    await page.evaluate(() => document.fonts.ready);
    const actual = await geometry([
      ".spatial-flow",
      ".spatial-flow-artwork",
      ".spatial-flow-front",
      ".spatial-navigation",
      ".interface-toolbar",
    ]);
    for (let i = 0; i < reference.length; i++)
      for (const key of ["x", "y", "width", "height"]) {
        const tolerance = i === 4 && key === "width" ? 8 : 10;
        assert.ok(
          Math.abs(reference[i][key] - actual[i][key]) <= tolerance,
          `${width} layout ${i} ${key}: preview=${reference[i][key]}, app=${actual[i][key]}`,
        );
      }
    assert.equal(
      await page.locator(".spatial-navigation nav button").count(),
      4,
    );
    assert.equal(
      await page.locator('.home-view input[type="range"]').count(),
      0,
    );
    assert.equal(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth + 1,
      ),
      true,
    );
    const dialBox = await page
      .getByRole("slider", { name: "Flow Steering" })
      .boundingBox();
    const footer = await page
      .locator(".kaze-interface > .player-bar")
      .boundingBox();
    assert.ok(
      dialBox.x >= actual[0].x &&
        dialBox.x + dialBox.width <= actual[0].x + actual[0].width &&
        dialBox.y + dialBox.height <= actual[0].y + actual[0].height,
      "dial fits the existing Flow composition",
    );
    await page.getByRole("slider", { name: "Flow Steering" }).focus();
    assert.ok(
      (await page.getByRole("slider", { name: "Flow Steering" }).boundingBox())
        .y +
        dialBox.height <=
        footer.y,
      "dial scrolls into view above the player",
    );
    await page.locator(".main-content").evaluate((el) => (el.scrollTop = 0));
    await page.screenshot({ path: `${out}/home-${width}.png` });
    console.log(
      `PASS ${width}×${height}: measured preview Home, sleeves, four-icon rail and circular dial`,
    );
  }
  await page.setViewportSize({ width: 1440, height: 900 });
  await nav("Home");
  const dial = page.getByRole("slider", { name: "Flow Steering" });
  await dial.focus();
  await dial.press("Home");
  await dial.press("ArrowRight");
  await dial.press("PageUp");
  assert.equal(await dial.getAttribute("aria-valuenow"), "0.15");
  await dial.press("End");
  assert.equal(await dial.getAttribute("aria-valuenow"), "1");
  await dial.press("Home");
  assert.equal(await dial.getAttribute("aria-valuenow"), "0");
  await page.waitForTimeout(300);
  assert.equal((await steering()).at(-1).payload.familiarity_ratio, 0);
  const before = (await steering()).length;
  const playback = await page.evaluate(() =>
    JSON.stringify(window.__kazeFixture.playback),
  );
  const r = await dial.boundingBox(),
    cx = r.x + r.width / 2,
    cy = r.y + r.height / 2;
  await page.mouse.move(cx, cy - r.height * 0.42);
  await page.mouse.down();
  for (let angle = 9; angle <= 81; angle += 9)
    await page.mouse.move(
      cx + Math.sin((angle * Math.PI) / 180) * r.width * 0.42,
      cy - Math.cos((angle * Math.PI) / 180) * r.height * 0.42,
    );
  await page.waitForTimeout(300);
  assert.equal(
    (await steering()).length,
    before,
    "dragging does not send commands or drive App state",
  );
  assert.equal(await dial.getAttribute("aria-valuenow"), "0.8");
  await page.mouse.up();
  await page.waitForTimeout(300);
  assert.equal((await steering()).length, before + 1);
  assert.equal((await steering()).at(-1).payload.familiarity_ratio, 0.8);
  assert.equal(
    await page.evaluate(() => JSON.stringify(window.__kazeFixture.playback)),
    playback,
    "steering preserves playback and queue",
  );
  await page.mouse.move(cx, cy - r.height * 0.42);
  await page.mouse.down();
  await dial.dispatchEvent("pointercancel", { pointerId: 1, bubbles: true });
  await page.mouse.up();
  assert.equal(await dial.getAttribute("aria-valuenow"), "0.8");
  console.log(
    "PASS dial keyboard bounds, pointer preview/commit/cancel, debounced command and playback continuity",
  );
  await nav("Library");
  await page.getByRole("tab", { name: "Albums", exact: true }).waitFor();
  assert.ok(await page.locator(".spatial-album-group").count());
  assert.ok((await page.locator(".album-card").count()) < 30);
  await page.screenshot({ path: `${out}/library.png` });
  await page.getByRole("tab", { name: "Tracks", exact: true }).click();
  await page.locator(".track-row").first().waitFor();
  await page.getByRole("tab", { name: "Playlists", exact: true }).click();
  await page.locator(".playlist-card").first().waitFor();
  await nav("Queue");
  assert.ok(await page.locator(".spatial-queue-future button").count());
  await page.screenshot({ path: `${out}/queue.png` });
  await page
    .getByRole("button", { name: "Library and more", exact: true })
    .click();
  await page.keyboard.press("Escape");
  assert.equal(await page.getByRole("menu").count(), 0);
  await nav("Notifications");
  await page
    .getByRole("heading", { name: "Notifications", exact: true })
    .waitFor();
  assert.equal(
    await page.locator(".main-content").getAttribute("data-view"),
    "notifications",
  );
  console.log(
    "PASS preview library tabs, bounded mixed shelves, queue path and additional feature menu",
  );
  for (const [width, height] of [[1920,1080],[1440,900],[1024,600],[800,700]]) {
    await page.setViewportSize({width,height});
    await nav("Library");
    await page.getByRole("tab", {name:"Playlists",exact:true}).click();
    await page.locator(".playlists-view .playlist-card").first().waitFor();
    const [header, playlists] = await geometry([".spatial-library-header", ".playlists-view"]);
    assert.ok(Math.abs(header.x-playlists.x)<=1 && Math.abs(header.width-playlists.width)<=1, `${width}: playlists align to Library`);
    assert.equal(await page.locator(".playlists-view .playlist-card").filter({hasText:"Liked Songs"}).count(),0);
    await page.screenshot({path:`${out}/playlists-aligned-${width}.png`});
    await page.getByRole("tab", {name:"Liked",exact:true}).click();
    await page.locator(".collection-track-row").first().waitFor();
    assert.equal(await page.getByRole("heading", {name:"Liked Songs",exact:true}).isVisible(),true);
    const [collection,sheet,row,footer] = await geometry([".collection-view--liked", ".collection-track-sheet", ".collection-track-row", ".kaze-interface > .player-bar"]);
    assert.ok(sheet.y-collection.y < 360, `${width}: compact Liked header ${JSON.stringify({collection,sheet,row,footer})}`);
    assert.ok(row.y+row.height <= footer.y+1, `${width}: first liked track stays above player ${JSON.stringify({row,footer})}`);
    await page.screenshot({path:`${out}/liked-compact-${width}.png`});
  }
  console.log("PASS aligned playlists, separate Liked access and compact one-track Liked header at four sizes");
  await page.setViewportSize({width:1440,height:900});
  await nav("Home");
  const sleeves = page.locator(".spatial-flow-artwork > button");
  const clickExposedSleeve = async (index) => {
    await page.locator(".main-content").evaluate(element => element.scrollTop = 0);
    const exposedPoint = () => sleeves.nth(index).evaluate(element => {
      const r = element.getBoundingClientRect();
      for (let row = 1; row < 10; row++) for (let column = 1; column < 10; column++) {
        const x = r.left + r.width * column / 10, y = r.top + r.height * row / 10;
        if (document.elementFromPoint(x,y)?.closest("button") === element) return {x,y};
      }
      return null;
    });
    let point = await exposedPoint();
    assert.ok(point, "overlapping sleeve has an exposed pointer target");
    await page.mouse.move(point.x, point.y);
    await page.waitForTimeout(400);
    point = await exposedPoint();
    assert.ok(point);
    await page.mouse.click(point.x, point.y);
  };
  assert.equal(await sleeves.count(), 3);
  assert.equal(await sleeves.nth(0).isDisabled(), true);
  assert.deepEqual(await sleeves.evaluateAll(elements => elements.map(element => getComputedStyle(element).opacity)), ["1", "1", "1"]);
  await clickExposedSleeve(2);
  await page.waitForFunction(() => document.querySelector(".spatial-flow-front").title === "Current: Feather");
  assert.deepEqual(await sleeves.evaluateAll(elements => elements.map(element => element.title)), ["Previous: Kerala", "Current: Feather", "Next: Friday Morning"]);
  await page.waitForFunction(() => [...document.querySelectorAll(".spatial-flow-artwork > button img")].filter(image => image.complete && image.naturalWidth > 0).length === 3);
  await page.locator(".main-content").evaluate(element => element.scrollTop = 0);
  await page.screenshot({ path: `${out}/home-playback-sleeves.png` });
  await clickExposedSleeve(0);
  await page.waitForFunction(() => document.querySelector(".spatial-flow-front").title === "Current: Kerala");
  assert.equal(await sleeves.nth(0).getAttribute("title"), "Previous: Feather");
  await page.evaluate(() => window.__kazeFixture.command({command:"ClearQueue"}));
  await page.waitForFunction(() => document.querySelector(".spatial-flow-back-1").disabled);
  assert.equal(await sleeves.nth(2).getAttribute("title"), "No next track");
  const placeholder = await sleeves.nth(2).locator(".music-artwork").evaluate(element => ({background:getComputedStyle(element).backgroundColor,icon:!!element.querySelector("svg"),opacity:getComputedStyle(element.parentElement).opacity}));
  assert.equal(placeholder.icon, true);
  assert.equal(placeholder.opacity, "1");
  assert.notEqual(placeholder.background, "rgba(0, 0, 0, 0)");
  await page.evaluate(async () => {
    const fixture = window.__kazeFixture;
    fixture.tracks[8].cover_art_url = "data:image/png;base64,broken";
    await fixture.command({command:"PlayTrack",payload:{track_id:"t8"}});
  });
  await page.waitForFunction(() => document.querySelector(".spatial-flow-front").title === "Current: Myth" && !!document.querySelector(".spatial-flow-front .music-artwork svg") && !document.querySelector(".spatial-flow-front img"));
  console.log("PASS Home previous/current/next order, actual transport actions and opaque missing/broken artwork placeholders");
  assert.deepEqual(errors, []);
} finally {
  await page.screenshot({ path: `${out}/last-state.png` });
  await browser.close();
}
