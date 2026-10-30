/** Production layout regressions against the accepted Spatial study. */
import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import { installDualInterfaceFixture } from "../tests/fixtures/dualInterfaceBackend.mjs";
const { chromium } =
  await import(process.env.KAZE_PLAYWRIGHT_MODULE || "/home/abhi/.cache/ms-playwright-go/1.57.0/package/index.mjs");
const out = process.env.KAZE_TEST_OUTPUT || "/tmp/kaze-spatial-restored";
await mkdir(out, { recursive: true });
const browser = await chromium.launch({
  executablePath: process.env.KAZE_CHROMIUM || "/usr/bin/chromium",
  headless: true,
  args: ["--no-sandbox"],
});
const context = await browser.newContext({
  viewport: { width: 1440, height: 900 },
});
await context.addInitScript(installDualInterfaceFixture);
await context.addInitScript(() => {
  const backend = window.__kazeFixture;
  const query = backend.query.bind(backend);
  backend.query = (request) =>
    request.query === "GetDiscoveryRecommendations"
      ? Promise.resolve({
          type: "DiscoveryRecommendations",
          data: backend.recs.map((rec, i) => ({
            ...rec,
            title: `New ${rec.title}`,
            recommendation_reason:
              i % 2
                ? "Similar artists from your taste"
                : "Top genre from your taste",
          })),
        })
      : query(request);
});
await context.route("**/src/services/api.ts*", (route) =>
  route.fulfill({
    contentType: "text/javascript",
    body: `export const isTauri=()=>false;export const executeQuery=q=>window.__kazeFixture.query(q);export const dispatchCommand=c=>window.__kazeFixture.command(c);export const subscribeBackendEvents=async cb=>{window.__kazeFixture.listeners.add(cb);return ()=>window.__kazeFixture.listeners.delete(cb);};`,
  }),
);
const page = await context.newPage();
const errors = [];
page.on("pageerror", (e) => errors.push(e.message));
const nav = async (name) => {
  if (name === "Now Playing") return page.locator(".kaze-interface > .player-bar").getByRole("button", { name: "Open Now Playing", exact: true }).click();
  const direct = page.locator(".sidebar").getByRole("button", { name, exact: true });
  if (await direct.count()) return direct.click();
  if (["Albums", "Downloaded Tracks", "Playlists"].includes(name)) {
    await page.locator(".sidebar").getByRole("button", { name: "Library", exact: true }).click();
    return page.getByRole("tab", { name: name === "Downloaded Tracks" ? "Tracks" : name, exact: true }).click();
  }
  await page.getByRole("button", { name: "Library and more", exact: true }).click();
  await page.getByRole("menuitem", { name, exact: true }).click();
};
try {
  await page.goto(process.env.KAZE_TEST_URL || "http://127.0.0.1:5194");
  await page.locator(".home-flow").waitFor();
  await nav("Discover Music");
  await page.locator(".windowed-strip").first().waitFor();
  await page.screenshot({ path: `${out}/discovery.png` });
  assert.equal(
    await page.getByRole("separator", { name: "Resize navigation" }).count(),
    1,
    "Spatial must use the accepted adjustable icon rail",
  );
  assert.equal(
    await page.locator(".spatial-discovery-artwork").count(),
    1,
    "Spatial must have the large artwork pane from the preview",
  );
  console.log("PASS accepted Spatial shell and artwork structure");
  const main = page.locator(".main-content");
  const rail = page.locator(".spatial-navigation");
  const separator = page.getByRole("separator", { name: "Resize navigation" });
  assert.equal(Math.round((await rail.boundingBox()).width), 72);
  assert.equal(await rail.locator(".nav-section-title,.logo-title").count(), 0);
  await separator.focus();
  await separator.press("End");
  assert.equal(Math.round((await rail.boundingBox()).width), 160);
  await separator.press("Home");
  await separator.press("ArrowRight");
  assert.equal(Math.round((await rail.boundingBox()).width), 64);
  const box = await separator.boundingBox();
  await page.mouse.move(box.x + box.width / 2, box.y + 50);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width / 2 + 38, box.y + 50, { steps: 8 });
  await page.mouse.up();
  assert.equal(Math.round((await rail.boundingBox()).width), 102);
  await separator.focus();
  await separator.press("Home");
  await separator.press("ArrowRight");
  await separator.press("ArrowRight");
  console.log("PASS icon rail pointer/keyboard resize with bounded widths");
  const playPreview = page.locator(".spatial-preview-play");
  assert.equal(await playPreview.getAttribute("aria-label"), "Pause Kerala");
  await playPreview.click();
  await page.waitForFunction(() => document.querySelector(".spatial-preview-play")?.getAttribute("aria-label") === "Play Kerala");
  await playPreview.click();
  await page.waitForFunction(() => document.querySelector(".spatial-preview-play")?.getAttribute("aria-label") === "Pause Kerala");
  console.log("PASS large preview play/pause follows real playback state");
  const strip = page.locator("#discovery-chapter-0 .windowed-strip");
  const before = await page.evaluate(
    () => window.__kazeFixture.commands.length,
  );
  await strip.locator(".discovery-track-card").nth(1).hover();
  await page.waitForFunction(
    () =>
      document.querySelector(".spatial-discovery-caption h2")?.textContent ===
      "Feather",
  );
  assert.equal(
    await page.evaluate(() => window.__kazeFixture.commands.length),
    before,
    "hover must not play or queue",
  );
  await strip
    .locator(".discovery-track-card")
    .nth(2)
    .getByRole("button")
    .first()
    .focus();
  await page.waitForFunction(
    () =>
      document.querySelector(".spatial-discovery-caption h2")?.textContent ===
      "Friday Morning",
  );
  assert.equal(
    await page.evaluate(() => window.__kazeFixture.commands.length),
    before,
    "focus must not start playback",
  );
  const stage = page.locator(".spatial-discovery-fixed");
  const initial = await stage.boundingBox();
  await stage.focus();
  await stage.press("PageDown");
  assert.equal(
    await page.locator(".chapter-nav button[aria-current]").textContent(),
    "02For You",
  );
  assert.deepEqual(await stage.boundingBox(), initial);
  await stage.dispatchEvent("wheel", { deltaY: 80, bubbles: true });
  for (let i = 0; i < 8; i++)
    await stage.dispatchEvent("wheel", { deltaY: 80, bubbles: true });
  assert.equal(
    await page.locator(".chapter-nav button[aria-current]").textContent(),
    "03Mixes For You",
    "one chapter per wheel gesture",
  );
  assert.equal(await main.evaluate((el) => el.scrollTop), 0);
  console.log(
    "PASS hover/focus preview, fixed chapters and wheel gesture gating",
  );
  for (const [width, height] of [
    [1920, 1080],
    [1440, 900],
    [1024, 600],
    [800, 700],
    [800, 600],
    [940, 1000],
    [800, 1000],
  ]) {
    await page.setViewportSize({ width, height });
    await nav("Discover Music");
    for (let i = 0; i < 4; i++) {
      await page.locator(".chapter-nav button").nth(i).click();
      await page.waitForTimeout(120);
      if (width === 1920 && i === 1) {
        await page.locator("#discovery-chapter-1 .discovery-track-card").nth(1).hover();
        await page.waitForFunction(() => document.querySelector(".spatial-discovery-caption h2")?.textContent === "New Feather");
        assert.equal(await page.locator(".discovery-shelf-filters").count(), 0);
      }
      const geometry = await page.evaluate(() => {
        const rect = (el) => {
          const r = el.getBoundingClientRect();
          return {
            x: r.x,
            y: r.y,
            right: r.right,
            bottom: r.bottom,
            width: r.width,
            height: r.height,
          };
        };
        const active = document.querySelector(
          ".discovery-chapter:not([hidden])",
        );
        const strip = active.querySelector(".horizontal-scroll-row");
        return {
          mainScroll: document.querySelector(".main-content").scrollTop,
          overflow: document.documentElement.scrollWidth > innerWidth + 1,
          footer: rect(document.querySelector(".kaze-interface > .player-bar")),
          art: rect(document.querySelector(".spatial-discovery-cover")),
          panel: rect(document.querySelector(".spatial-discovery-panel")),
          caption: rect(document.querySelector(".spatial-discovery-caption")),
          copy: rect(document.querySelector(".spatial-discovery-copy")),
          detail: rect(document.querySelector(".spatial-discovery-detail")),
          strip: strip ? rect(strip) : null,
          shelfScrolls: (() => {
            const shelf = document.querySelector(".chapter-stage");
            return getComputedStyle(shelf).overflowY === "auto" && shelf.scrollHeight > shelf.clientHeight + 1;
          })(),
          buttons: [...active.querySelectorAll("button")]
            .filter((el) => el.getBoundingClientRect().width > 0)
            .map(rect),
          stage: rect(document.querySelector(".spatial-discovery-fixed")),
        };
      });
      assert.equal(geometry.mainScroll, 0);
      assert.equal(geometry.overflow, false);
      assert.ok(
        geometry.art.width > 180,
        `${width} large cover remains visible`,
      );
      assert.ok(geometry.art.right <= geometry.panel.x - 8,
        `${width}×${height} preview overlaps the right panel: ${JSON.stringify({art:geometry.art,panel:geometry.panel})}`);
      assert.ok(
        geometry.art.bottom <= geometry.caption.y + 12,
        `${width} art/caption overlap`,
      );
      assert.ok(
        geometry.copy.bottom <= geometry.detail.y + 1,
        `${width} chapter text overlaps details`,
      );
      if (geometry.strip) {
        assert.ok(
          geometry.detail.bottom <= geometry.strip.y + 1,
          `${width} details overlap shelf`,
        );
        for (const button of geometry.buttons.filter(
          (r) => r.x >= geometry.strip.x && r.x < geometry.strip.right,
        )) {
          assert.ok(
            button.bottom <=
              (geometry.shelfScrolls ? geometry.strip.bottom : Math.min(geometry.strip.bottom, geometry.stage.bottom, geometry.footer.y)) + 1,
            `${width} chapter ${i} action clipped: ${JSON.stringify(geometry)}`,
          );
        }
        if (geometry.shelfScrolls) {
          const action = page.locator('.discovery-chapter button').last();
          await action.scrollIntoViewIfNeeded();
          const box = await action.boundingBox();
          assert.ok(box.y + box.height <= geometry.footer.y + 1,
            "Overflowed shelf actions must be reachable above the player");
        }
      }
      assert.equal(await page.locator(".discovery-hero").isVisible(), false);
      await page.screenshot({
        path: `${out}/discovery-${width}-chapter-${i}.png`,
      });
    }
    await nav("Now Playing");
    await page.waitForTimeout(120);
    const sleeve = page.locator(".stage-current");
    const sleeveBefore = await sleeve.boundingBox();
    await page.locator(".stage-sequence").hover();
    await page.mouse.wheel(0, 600);
    assert.equal(await main.evaluate((el) => el.scrollTop), 0);
    assert.deepEqual(await sleeve.boundingBox(), sleeveBefore);
    const footerTop = (
      await page.locator(".kaze-interface > .player-bar").boundingBox()
    ).y;
    const caption = await page.locator(".stage-caption").boundingBox();
    assert.ok(caption.y + caption.height <= footerTop);
    assert.ok(
      sleeveBefore.height > 100 &&
        sleeveBefore.y + sleeveBefore.height <= caption.y,
    );
    await page.screenshot({ path: `${out}/playing-${width}.png` });
    console.log(
      `PASS ${width}×${height}: all chapters and fixed Now Playing fit`,
    );
  }
  await page.setViewportSize({ width: 1440, height: 900 });
  await nav("Discover Music");
  await page.locator(".chapter-nav button").nth(2).click();
  await page.locator(".spatial-preview-title").click();
  await page.locator(".collection-view").waitFor();
  assert.equal(await main.getAttribute("data-fixed-stage"), null);
  assert.equal(await main.evaluate(el => getComputedStyle(el).overflowY), "auto");
  assert.ok(await main.evaluate(el => { el.scrollTop = 200; return el.scrollTop > 0; }));
  console.log("PASS collections opened from Discovery retain normal scrolling");
  assert.deepEqual(errors, []);
} finally {
  await browser.close();
}
