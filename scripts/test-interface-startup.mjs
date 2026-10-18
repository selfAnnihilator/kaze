/** First-paint regression with the real App and deliberately delayed settings. */
import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import { installDualInterfaceFixture } from "../tests/fixtures/dualInterfaceBackend.mjs";

const { chromium } = await import(
  process.env.KAZE_PLAYWRIGHT_MODULE ||
    "/home/abhi/.cache/ms-playwright-go/1.57.0/package/index.mjs"
);
const base = process.env.KAZE_TEST_URL || "http://127.0.0.1:5194";
const output = process.env.KAZE_TEST_OUTPUT || "/tmp/kaze-startup-animation";
await mkdir(output, { recursive: true });
const browser = await chromium.launch({
  executablePath: process.env.KAZE_CHROMIUM || "/usr/bin/chromium",
  headless: true,
  args: ["--no-sandbox"],
});

async function checkIntro(page, style, reducedMotion) {
  await page.waitForFunction(() => document.querySelector(".app-startup-icon")?.naturalWidth > 0);
  const announcement = await page.locator(".app-startup").ariaSnapshot();
  assert.match(announcement, /Opening Kaze/);
  assert.ok(!announcement.includes("KAZE"), "Decorative letters must not produce separate announcements");
  const geometry = await page.evaluate(() => {
    const rect = selector => {
      const { x, y, width, height } = document.querySelector(selector).getBoundingClientRect();
      return { x, y, width, height };
    };
    return { brand: rect(".app-startup-brand"), icon: rect(".app-startup-icon"), name: rect(".app-startup-name"), width: innerWidth, height: innerHeight };
  });
  assert.ok(Math.abs(geometry.brand.x + geometry.brand.width / 2 - geometry.width / 2) < 1);
  assert.ok(Math.abs(geometry.brand.y + geometry.brand.height / 2 - geometry.height / 2) < 1);
  assert.ok(geometry.name.y > geometry.icon.y + geometry.icon.height, "The name sits below the icon");
  assert.ok(Number(await page.locator(".app-startup-name").evaluate(node => getComputedStyle(node).fontWeight)) >= 700);
  assert.equal(await page.locator(".app-startup-ring").count(), style === "spatial" ? 1 : 0);
  assert.equal(await page.locator(".app-startup-frame").count(), style === "brutalist" ? 1 : 0);
  assert.equal(await page.locator(".app-startup-rail").count(), style === "brutalist" ? 1 : 0);
  const visibleLetters = () => page.evaluate(() => [...document.querySelectorAll(".app-startup-name > span")]
    .filter(letter => Number(getComputedStyle(letter).opacity) > .9).map(letter => letter.textContent).join(""));
  if (reducedMotion) {
    assert.equal(await visibleLetters(), "KAZE");
    assert.deepEqual(await page.locator(".app-startup-brand").evaluate(node => node.getAnimations({ subtree: true })
      .map(animation => ({ type: animation.constructor.name, name: animation.animationName, property: animation.transitionProperty,
        target: animation.effect?.target?.className, state: animation.playState }))), []);
  } else {
    const samples = [];
    const indicatorSamples = [];
    for (const time of [0, 600, 900, 2000]) {
      await page.locator(".app-startup-brand").evaluate((node, time) => {
        for (const animation of node.getAnimations({ subtree: true })) {
          animation.pause();
          animation.currentTime = time;
        }
      }, time);
      samples.push(await page.locator(".app-startup-name").evaluate(node => [...node.children].map(letter => {
        const style = getComputedStyle(letter);
        return { opacity: Number(style.opacity), x: new DOMMatrixReadOnly(style.transform).m41 };
      })));
      indicatorSamples.push(await page.locator(style === "spatial" ? ".app-startup-ring" : ".app-startup-rail > span")
        .evaluate(node => getComputedStyle(node).transform));
      if (time === 900) await page.screenshot({ path: `${output}/${style}-flowing-reveal.png` });
    }
    assert.ok(samples[0].every(letter => letter.opacity === 0));
    const wave = samples[1];
    assert.ok(wave[0].opacity > wave[1].opacity && wave[1].opacity > wave[2].opacity && wave[2].opacity > wave[3].opacity,
      "The fade must progress softly from the start of the word to the end");
    assert.ok(samples[2].every((letter, index) => letter.opacity > wave[index].opacity && letter.opacity > 0 && letter.opacity < 1),
      "All four letters must overlap in a continuous fade instead of appearing as typed steps");
    assert.ok(samples[2].every(letter => letter.x < 0 && letter.x > -12), "Letters drift gently into place");
    assert.ok(samples[3].every(letter => letter.opacity === 1 && Math.abs(letter.x) < .01));
    assert.notEqual(indicatorSamples[0], indicatorSamples[1], "Each theme's loading indicator moves");
    assert.equal(await page.locator(".app-startup-cursor").count(), 0);
    const after = await page.locator(".app-startup-brand").boundingBox();
    assert.deepEqual(after, geometry.brand, "The reveal must not move the centered composition");
  }
  await page.screenshot({ path: `${output}/${style}-${reducedMotion ? "reduced-motion" : "revealed-name"}.png` });
  console.log(`PASS ${style} startup animation: ${reducedMotion ? "still reduced-motion composition" : style === "spatial" ? "flowing fade and circular motion" : "flowing fade, bold lettering, square frame and red sweep"}`);
}

async function scenario(style, { failFirst = false, lab = false, defaultStyle = false, inspectIntro = false, reducedMotion = false, readyImmediately = false, slowApp = false, failApp = false } = {}) {
  const context = await browser.newContext({ reducedMotion: reducedMotion ? "reduce" : "no-preference" });
  let releaseApp;
  if (inspectIntro || slowApp) {
    const appReady = new Promise(resolve => { releaseApp = resolve; });
    await context.route("**/src/App.tsx*", async route => {
      await appReady;
      await route.continue();
    });
  }
  if (failApp) await context.route("**/src/App.tsx*", route => route.abort("failed"));
  await context.addInitScript({
    content: `
      sessionStorage.setItem('test.interface-style', ${JSON.stringify(style)});
      (${installDualInterfaceFixture.toString()})();
      if (${defaultStyle}) delete window.__kazeFixture.settings.interface_style;
      window.startupTrace = [];
      window.introTrace = [];
      window.startupQueries = [];
      window.settingsAttempts = 0;
      new MutationObserver(() => {
        if (document.querySelector('.app-startup') && window.startupMountedAt === undefined)
          window.startupMountedAt = performance.now();
        const intro = document.querySelector('.app-startup[data-startup-style]');
        if (intro && intro.dataset.startupStyle !== 'pending') {
          if (window.introMountedAt === undefined) window.introMountedAt = performance.now();
          window.introTrace.push(intro.dataset.startupStyle);
        }
        const shell = document.querySelector('.kaze-interface');
        if (shell) {
          if (window.appMountedAt === undefined) window.appMountedAt = performance.now();
          window.startupTrace.push({
            root: document.documentElement.dataset.interface,
            shell: shell.dataset.interface
          });
        }
      }).observe(document, {subtree:true, childList:true, attributes:true, attributeFilter:['data-interface','data-startup-style']});
      const query = window.__kazeFixture.query.bind(window.__kazeFixture);
      window.__kazeFixture.query = async q => {
        window.startupQueries.push(q.query);
        if (q.query === 'GetSettings') {
          window.settingsAttempts++;
          if (${failFirst} && window.settingsAttempts === 1) throw Error('test: unavailable settings');
          await new Promise(resolve => { window.releaseSettings = resolve; });
        }
        return query(q);
      };
    `,
  });
  await context.route("**/src/services/api.ts*", (route) =>
    route.fulfill({
      contentType: "text/javascript",
      body: `export const isTauri=()=>false;export const executeQuery=q=>window.__kazeFixture.query(q);export const dispatchCommand=c=>window.__kazeFixture.command(c);export const subscribeBackendEvents=async cb=>{window.__kazeFixture.listeners.add(cb);return()=>window.__kazeFixture.listeners.delete(cb);};`,
    }),
  );
  const page = await context.newPage();
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  try {
    await page.goto(lab ? `${base}/?lab=${style}` : base, { waitUntil: "domcontentloaded" });
    if (lab) {
      await page.locator(`.${style}-kaze`).waitFor();
      assert.deepEqual(await page.evaluate(() => window.startupQueries), []);
      assert.equal(await page.locator(".kaze-interface").count(), 0);
      console.log(`PASS ${style}: standalone study makes no startup IPC calls`);
      return;
    }
    if (failApp) {
      await page.getByRole("button", { name: "Try again", exact: true }).waitFor();
      await page.waitForFunction(() => typeof window.releaseSettings === "function");
      await page.evaluate(() => window.releaseSettings());
      await page.waitForTimeout(150);
      assert.equal(await page.getByRole("button", { name: "Try again", exact: true }).count(), 1);
      assert.equal(await page.locator("[data-startup-style],.kaze-interface").count(), 0,
        "A late settings response must not revive a failed startup");
      assert.deepEqual(errors, []);
      console.log("PASS startup cancellation: failed App code remains retryable after a late settings response");
      return;
    }
    if (failFirst) {
      await page.getByRole("button", { name: "Try again", exact: true }).waitFor();
      assert.equal(await page.locator(".kaze-interface").count(), 0);
      await page.getByRole("button", { name: "Try again", exact: true }).click();
    }
    await page.waitForFunction(() => typeof window.releaseSettings === "function");
    // Give React multiple frames to expose a premature default-theme mount.
    if (!readyImmediately) await page.waitForTimeout(250);
    assert.equal(await page.locator(".kaze-interface").count(), 0,
      "The app must not mount a default theme while saved settings are pending");
    assert.deepEqual(await page.evaluate(() => window.startupQueries),
      Array(failFirst ? 2 : 1).fill("GetSettings"));
    assert.equal(await page.locator(".app-startup").getAttribute("data-startup-style"), "pending");
    assert.equal(await page.locator(".app-startup-brand").evaluate(node => node.getAnimations({ subtree: true }).length), 0,
      "No guessed-theme animation may run before the preference loads");
    await page.evaluate(() => window.releaseSettings());
    if (inspectIntro || slowApp) {
      await page.locator(`.app-startup[data-startup-style="${style}"]`).waitFor();
      if (inspectIntro) await checkIntro(page, style, reducedMotion);
      if (slowApp) {
        await page.waitForTimeout(2200);
        assert.equal(await page.locator(`.app-startup[data-startup-style="${style}"]`).count(), 1,
          "The chosen theme remains visible while App code is delayed");
        assert.equal(await page.locator(".kaze-interface").count(), 0);
      }
      releaseApp();
    }
    await page.locator(".home-flow").waitFor();
    await page.waitForTimeout(250);
    const trace = await page.evaluate(() => window.startupTrace);
    assert.ok(trace.length > 0);
    assert.ok(trace.every((paint) => paint.root === style && paint.shell === style),
      `Every initial shell commit must use ${style}: ${JSON.stringify(trace)}`);
    assert.equal(await page.evaluate(() => window.settingsAttempts), failFirst ? 2 : 1,
      "Startup settings must be reused by App, including in Strict Mode");
    assert.ok((await page.evaluate(() => window.introTrace)).every(value => value === style), "The intro must never show the other theme");
    if (readyImmediately && !reducedMotion) {
      const duration = await page.evaluate(() => window.appMountedAt - window.introMountedAt);
      assert.ok(duration >= 1950, `The fast startup must finish its theme intro (${duration}ms)`);
      assert.ok(duration < 4000, `The intro must not keep an already-ready app waiting (${duration}ms)`);
    }
    assert.equal(await page.locator(".app-startup").count(), 0, "The startup screen unmounts after opening");
    assert.deepEqual(errors, []);
    console.log(`PASS ${style}: delayed settings${failFirst ? " and retry" : ""}${defaultStyle ? " with no saved preference" : ""} produce only the expected theme from the first commit`);
  } finally {
    releaseApp?.();
    await context.close();
  }
}

try {
  for (const style of ["spatial", "brutalist"]) await scenario(style, { inspectIntro: true });
  await scenario("brutalist", { readyImmediately: true });
  await scenario("spatial", { readyImmediately: true });
  for (const style of ["spatial", "brutalist"]) await scenario(style, { inspectIntro: true, reducedMotion: true });
  await scenario("brutalist", { slowApp: true });
  await scenario("brutalist", { failApp: true });
  await scenario("spatial", { defaultStyle: true });
  await scenario("brutalist", { failFirst: true });
  for (const style of ["brutalist", "spatial"]) await scenario(style, { lab: true });
} finally {
  await browser.close();
}
