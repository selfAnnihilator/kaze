/** Navigation restores browsing context without retaining hidden views or touching playback. */
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
try {
  for (const [style, width, height] of ["brutalist", "spatial"].flatMap(
    (style) =>
      [
        [1888, 1004],
        [1440, 900],
        [1024, 600],
      ].map(([width, height]) => [style, width, height]),
  )) {
    const context = await browser.newContext({
      viewport: { width, height },
    });
    await context.addInitScript({
      content: `sessionStorage.setItem('test.interface-style','${style}');(${installDualInterfaceFixture.toString()})();`,
    });
    await context.addInitScript(() => {
      const fixture = window.__kazeFixture,
        query = fixture.query.bind(fixture);
      fixture.query = (request) =>
        request.query === "GetDiscoveryRecommendations"
          ? Promise.resolve({
              type: "DiscoveryRecommendations",
              data: fixture.recs.map((rec, i) => ({
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
        .locator(".press-header,.spatial-navigation")
        .getByRole("button", { name, exact: true })
        .click();
    const chapter = () =>
      page.locator(".discovery-chapter").getAttribute("data-chapter-index");
    const shelf = () =>
      page.locator(".discovery-chapter .horizontal-scroll-row");
    const select = async (index) => {
      await page.locator(".discovery-chapters").focus();
      const current = Number(await chapter());
      for (let i = current; i < index; i++)
        await page.keyboard.press("PageDown");
      for (let i = current; i > index; i--) await page.keyboard.press("PageUp");
      assert.equal(await chapter(), String(index));
    };
    const back = (keyboard) =>
      keyboard
        ? page.keyboard.press("Alt+ArrowLeft")
        : page.getByRole("button", { name: "Go back", exact: true }).click();
    const forward = (keyboard) =>
      keyboard
        ? page.keyboard.press("Alt+ArrowRight")
        : page.getByRole("button", { name: "Go forward", exact: true }).click();
    await nav("Discover Music");
    await page.locator(".discovery-card-body").first().waitFor();
    const wheelTarget = page.locator(
      style === "brutalist" ? ".press-bin-heading" : ".spatial-discovery-copy",
    );
    await wheelTarget.hover();
    await page.mouse.wheel(0, 120);
    await page.locator('[data-chapter-index="1"]').waitFor();
    await nav("Home");
    await back(false);
    await page.locator('[data-chapter-index="1"]').waitFor();
    console.log(
      `PASS ${style} ${width}×${height}: wheel-selected For You restored`,
    );
    const commands = await page.evaluate(
      () => window.__kazeFixture.commands.length,
    );
    const positions = new Map();
    for (const index of [0, 1, 2, 3]) {
      await select(index);
      if (await shelf().count()) {
        await shelf().evaluate((el) =>
          el.scrollTo({ left: 480, behavior: "instant" }),
        );
        await page.waitForTimeout(80);
        positions.set(index, await shelf().evaluate((el) => el.scrollLeft));
      }
      await nav("Home");
      assert.equal(
        await page.locator(".discovery-chapters").count(),
        0,
        "inactive view unmounted",
      );
      await back(index % 2 === 0);
      await page.locator(`[data-chapter-index="${index}"]`).waitFor();
      if (positions.has(index))
        assert.ok(
          Math.abs(
            (await shelf().evaluate((el) => el.scrollLeft)) -
              positions.get(index),
          ) <= 1,
          `chapter ${index} shelf restored ${await shelf().evaluate((el) => el.scrollLeft)} expected ${positions.get(index)}`,
        );
      if (index === 1)
        assert.equal(await page.locator(".discovery-shelf-filters").count(), 0);
      await forward(index % 2 !== 0);
      await page
        .locator(style === "brutalist" ? ".press-home" : ".spatial-home")
        .waitFor();
      await back(index % 2 !== 0);
      await page.locator(`[data-chapter-index="${index}"]`).waitFor();
      assert.equal(await page.locator(".discovery-chapter").count(), 1);
      console.log(
        `PASS ${style}: chapter ${index + 1}, shelf, Back/Forward button and shortcut`,
      );
    }
    // Every chapter's independent shelf offset survives subsequent route unmounts.
    for (const [index, left] of positions) {
      await select(index);
      assert.ok(
        Math.abs((await shelf().evaluate((el) => el.scrollLeft)) - left) <= 1,
      );
    }
    await select(3);
    await page.locator(".chart-card").first().click();
    await page.locator(".collection-view").waitFor();
    await back(false);
    await page.locator('[data-chapter-index="3"]').waitFor();
    await forward(true);
    await page.locator(".collection-view").waitFor();
    await back(true);
    await page.locator('[data-chapter-index="3"]').waitFor();
    assert.equal(
      await page.evaluate(() => window.__kazeFixture.commands.length),
      commands,
      "browsing never changes playback",
    );
    console.log(
      `PASS ${style}: collection round trip restores Discovery chapter`,
    );
    await nav("Home");
    await page
      .locator(style === "brutalist" ? ".press-home" : ".spatial-home")
      .waitFor();
    await page
      .locator(".main-content")
      .evaluate((el) => el.scrollTo({ top: 450, behavior: "instant" }));
    await page.waitForTimeout(80);
    const homeTop = await page
      .locator(".main-content")
      .evaluate((el) => el.scrollTop);
    assert.ok(homeTop > 0);
    await nav("Discover Music");
    await back(true);
    await page
      .locator(style === "brutalist" ? ".press-home" : ".spatial-home")
      .waitFor();
    assert.ok(
      Math.abs(
        (await page.locator(".main-content").evaluate((el) => el.scrollTop)) -
          homeTop,
      ) <= 1,
    );
    console.log(`PASS ${style}: ordinary page vertical position retained`);
    if (style === "brutalist") {
      await nav("Library");
      const spines = page.getByRole("region", {
        name: "Album spines",
        exact: true,
      });
      await spines.waitFor();
      await spines.evaluate((el) =>
        el.scrollTo({ left: 1400, behavior: "instant" }),
      );
      await page.waitForTimeout(80);
      const left = await spines.evaluate((el) => el.scrollLeft);
      await spines.locator("button").nth(3).click();
      await page.locator(".library-view").waitFor();
      await back(false);
      await spines.waitFor();
      assert.ok(
        Math.abs((await spines.evaluate((el) => el.scrollLeft)) - left) <= 1,
      );
      assert.ok((await spines.locator("button").count()) < 30);
      console.log(
        "PASS brutalist: Archive restores album shelf after opening a record, still windowed",
      );
    }
    assert.deepEqual(errors, []);
    await context.close();
  }
} finally {
  await browser.close();
}
