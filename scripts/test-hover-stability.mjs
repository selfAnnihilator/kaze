/** Exercise actual pointer edges, title overflow and deck geometry with private fixture data. */
import assert from "node:assert/strict";
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
const base = process.env.KAZE_TEST_URL || "http://127.0.0.1:5194";
const longTitle =
  "Uunan And The Stone Storage Room (One Piece) — A Very Long Song Name For Testing Overflow";
const sameBox = (a, b, label) => {
  for (const key of ["x", "y", "width", "height"])
    assert.ok(Math.abs(a[key] - b[key]) < 0.1, `${label}: ${key} moved`);
};
try {
  for (const style of ["brutalist", "spatial"]) {
    const context = await browser.newContext({
      viewport: { width: 1888, height: 1004 },
    });
    await context.addInitScript({
      content: `sessionStorage.setItem('test.interface-style','${style}');(${installDualInterfaceFixture.toString()})();`,
    });
    await context.route("**/src/services/api.ts*", (route) =>
      route.fulfill({
        contentType: "text/javascript",
        body: `export const isTauri=()=>false;export const executeQuery=async q=>{const r=await window.__kazeFixture.query(q);if(q.query==='GetStatsOverview')r.data.monthly_graph=Array.from({length:31},(_,i)=>({day:i+1,date:'2026-10-'+String(i+1).padStart(2,'0'),total_seconds:i%3===0?(i+1)*120:0}));return r};export const dispatchCommand=c=>window.__kazeFixture.command(c);export const subscribeBackendEvents=async cb=>{window.__kazeFixture.listeners.add(cb);return()=>window.__kazeFixture.listeners.delete(cb)};`,
      }),
    );
    const page = await context.newPage();
    page.setDefaultTimeout(10000);
    const errors = [];
    page.on("pageerror", (error) => errors.push(error.message));
    await page.goto(base);
    await page
      .locator(style === "brutalist" ? ".press-home" : ".spatial-home")
      .waitFor();
    await page.evaluate(() => document.fonts.ready);
    const nav = (name) =>
      page
        .locator(".press-header,.spatial-navigation")
        .getByRole("button", { name, exact: true })
        .click();
    const setTitle = (title) =>
      page.evaluate(async (title) => {
        window.__kazeFixture.tracks[0].title = title;
        await window.__kazeFixture.command({
          command: "PlayTrack",
          payload: { track_id: "t0" },
        });
      }, title);
    const hover = async (target, label, edge = true) => {
      await page.mouse.move(5, 5);
      await target.scrollIntoViewIfNeeded();
      await page.waitForTimeout(250);
      const before = await target.boundingBox();
      const point = await target.evaluate((el, edge) => {
        const r = el.getBoundingClientRect(),
          x = r.x + r.width / 2;
        for (const y of [
          edge ? r.bottom - 1 : r.y + r.height / 2,
          Math.min(r.y + r.height / 2, innerHeight / 2),
          r.y + 10,
        ]) {
          if (el.contains(document.elementFromPoint(x, y))) return { x, y };
        }
        throw Error("No exposed hover target");
      }, edge);
      await target.evaluate((el) => {
        el.dataset.enters = "0";
        el.dataset.leaves = "0";
        el.addEventListener(
          "mouseenter",
          () => (el.dataset.enters = String(+el.dataset.enters + 1)),
        );
        el.addEventListener(
          "mouseleave",
          () => (el.dataset.leaves = String(+el.dataset.leaves + 1)),
        );
      });
      for (let i = 0; i < 24; i++) {
        await page.mouse.move(point.x, point.y);
        await page.waitForTimeout(20);
      }
      const events = await target.evaluate((el) => ({
        enters: +el.dataset.enters,
        leaves: +el.dataset.leaves,
      }));
      assert.deepEqual(events, { enters: 1, leaves: 0 }, label);
      sameBox(before, await target.boundingBox(), label);
      console.log(`PASS ${style}: ${label}`);
    };
    if (style === "brutalist") {
      const boxes = () =>
        page.locator("#press-deck").evaluate((deck) =>
          Object.fromEntries(
            [
              ".press-play-main",
              ".press-skips",
              ".press-position",
              ".press-options-row",
              ".press-deck-next",
              ".press-deck-bottom",
              ".press-account-navigation",
            ].map((selector) => {
              const r = deck.querySelector(selector).getBoundingClientRect();
              return [
                selector,
                { x: r.x, y: r.y, width: r.width, height: r.height },
              ];
            }),
          ),
        );
      for (const [width, height] of [
        [1888, 1004],
        [1440, 900],
        [1024, 600],
        [800, 700],
        [390, 844],
      ]) {
        await page.setViewportSize({ width, height });
        await setTitle("Kerala");
        await page.waitForTimeout(50);
        const short = await boxes();
        await setTitle(longTitle);
        await page.waitForTimeout(50);
        const long = await boxes();
        for (const selector of Object.keys(short))
          sameBox(short[selector], long[selector], `${width} ${selector}`);
        const title = page.locator("#press-deck h2 .overflow-title");
        assert.equal(await title.getAttribute("data-overflow"), "true");
        const rows = page.locator("#press-deck .press-deck-next-song");
        if (await rows.first().isVisible()) {
          const last = await rows.last().boundingBox(),
            bottom = long[".press-deck-bottom"];
          assert.ok(
            last.y + last.height <= bottom.y,
            "queue must not overlap Flow",
          );
        }
        console.log(`PASS fixed deck and truncated title: ${width}x${height}`);
      }
      await page.setViewportSize({ width: 1888, height: 1004 });
      await page
        .locator("#press-deck")
        .getByRole("button", { name: "Open Now Playing", exact: true })
        .click();
      const title = page.locator(".press-listen h1 .overflow-title");
      await title.waitFor();
      await page.waitForTimeout(50);
      assert.equal(await title.getAttribute("data-overflow"), "true");
      await page.screenshot({ path: "/tmp/kaze-stable-title.png" });
      const before = await title.boundingBox();
      await title.hover();
      await page.waitForTimeout(500);
      const ending = await title
        .locator(".overflow-title-reveal")
        .evaluate((el) => {
          const animation = el.getAnimations()[0];
          if (!animation) return null;
          animation.pause();
          const timing = animation.effect.getTiming();
          animation.currentTime =
            Number(timing.delay) + Number(timing.duration) * 0.95;
          const box = el.getBoundingClientRect(),
            frame = el.parentElement.getBoundingClientRect();
          return {
            transform: getComputedStyle(el).transform,
            right: box.right,
            frameRight: frame.right,
          };
        });
      assert.ok(
        ending && ending.transform !== "none",
        "long title reveals its end",
      );
      assert.ok(
        Math.abs(ending.right - ending.frameRight) < 2,
        "final letters reach the visible title frame",
      );
      sameBox(before, await title.boundingBox(), "marquee hit area");
      await page.mouse.move(5, 5);
      await title.focus();
      assert.equal(
        await title
          .locator(".overflow-title-reveal")
          .evaluate((el) => getComputedStyle(el).opacity),
        "1",
      );
      await page.emulateMedia({ reducedMotion: "reduce" });
      assert.equal(
        await title
          .locator(".overflow-title-label")
          .evaluate((el) => getComputedStyle(el).opacity),
        "1",
      );
      assert.equal(
        await title
          .locator(".overflow-title-reveal")
          .evaluate((el) => getComputedStyle(el).display),
        "none",
      );
      await page.emulateMedia({ reducedMotion: "no-preference" });
      await page
        .locator("#press-deck")
        .getByRole("button", { name: "Full Screen", exact: true })
        .click();
      await page.locator(".fullscreen-player h1 .overflow-title").waitFor();
      assert.equal(
        await page
          .locator(".fullscreen-player h1 .overflow-title")
          .getAttribute("data-overflow"),
        "true",
      );
      await page.keyboard.press("Escape");
      await page.locator(".fullscreen-player").waitFor({ state: "detached" });
      await page
        .locator(".press-account-navigation")
        .getByRole("button", { name: "Profile", exact: true })
        .click();
      const bar = page
        .locator('.listening-graph-bar[data-has-time="true"]')
        .first();
      await bar.waitFor();
      const color = await bar.evaluate(
        (el) => getComputedStyle(el).backgroundColor,
      );
      assert.equal(
        color,
        await page
          .locator("#press-deck .press-play-main")
          .evaluate((el) => getComputedStyle(el).backgroundColor),
      );
      await bar.scrollIntoViewIfNeeded();
      await bar.hover();
      assert.equal(
        await bar.evaluate((el) => getComputedStyle(el).backgroundColor),
        color,
      );
      await page.screenshot({ path: "/tmp/kaze-red-graph.png" });
      console.log(
        "PASS red activity bars, title reveal, keyboard, reduced motion and fullscreen",
      );
      await nav("Home");
    }
    await hover(
      page.locator(
        style === "brutalist" ? ".press-home-next" : ".spatial-flow-front",
      ),
      "Home cover keeps its target",
      false,
    );
    await nav("Discover Music");
    await page.locator(".discovery-card-body").first().waitFor();
    await hover(
      page.locator(".discovery-card-body").first(),
      "Discovery edge remains hovered",
    );
    await nav("Library");
    const album = page
      .locator(style === "brutalist" ? ".press-record-spine" : ".album-card")
      .first();
    await album.waitFor();
    await hover(album, "album edge remains hovered");
    assert.deepEqual(errors, []);
    await context.close();
  }
} finally {
  await browser.close();
}
