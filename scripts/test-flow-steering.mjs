/** Real App: steering changes refresh confirmed session/queue data in both themes. */
import assert from "node:assert/strict";
import { installDualInterfaceFixture } from "../tests/fixtures/dualInterfaceBackend.mjs";
const { chromium } = await import(process.env.KAZE_PLAYWRIGHT_MODULE || "/home/abhi/.cache/ms-playwright-go/1.57.0/package/index.mjs");
const browser = await chromium.launch({ executablePath: process.env.KAZE_CHROMIUM || "/usr/bin/chromium", headless: true, args: ["--no-sandbox"] });
try {
  for (const style of ["spatial", "brutalist"]) {
    const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
    await context.addInitScript(installDualInterfaceFixture);
    await context.addInitScript(style => {
      sessionStorage.setItem("test.interface-style", style);
      const f = window.__kazeFixture, command = f.command.bind(f);
      f.settings.interface_style = style;
      f.command = async c => {
        const result = await command(c);
        if (c.command === "SetFlowSteering") f.flow.flow_label = c.payload.familiarity_ratio >= .75
          ? "Familiar mix · Your favorites" : "Rediscovery mix · Deep cuts";
        return result;
      };
    }, style);
    await context.route("**/src/services/api.ts*", route => route.fulfill({ contentType: "text/javascript", body: `export const isTauri=()=>false;export const executeQuery=q=>window.__kazeFixture.query(q);export const dispatchCommand=c=>window.__kazeFixture.command(c);export const subscribeBackendEvents=async cb=>{window.__kazeFixture.listeners.add(cb);return ()=>window.__kazeFixture.listeners.delete(cb);};` }));
    const page = await context.newPage(), errors = [];
    page.on("pageerror", error => errors.push(error.message));
    await page.goto(process.env.KAZE_TEST_URL || "http://127.0.0.1:5194");
    const slider = page.getByRole("slider", { name: "Flow Steering" });
    await slider.waitFor();
    await page.waitForTimeout(400);
    const baseline = await page.evaluate(() => ({
      track: window.__kazeFixture.playback.current_track.id,
      commands: window.__kazeFixture.commands.length,
      sessionQueries: window.__kazeFixture.queries.filter(q => q.query === "GetFlowSession").length,
      playbackQueries: window.__kazeFixture.queries.filter(q => q.query === "GetPlaybackState").length,
    }));
    await slider.focus();
    await slider.press("End");
    await slider.press("Home");
    await slider.press("End");
    await page.getByText("Familiar mix · Your favorites", { exact: true }).waitFor();
    const changed = await page.evaluate(() => ({
      commands: window.__kazeFixture.commands,
      queries: window.__kazeFixture.queries,
      track: window.__kazeFixture.playback.current_track.id,
      subscriptions: window.__kazeFixture.listeners.size,
    }));
    const steering = changed.commands.slice(baseline.commands).filter(c => c.command === "SetFlowSteering");
    assert.equal(steering.length, 1, "rapid keyboard changes must share the 250ms debounce");
    assert.equal(steering[0].payload.familiarity_ratio, 1);
    assert.equal(changed.commands.slice(baseline.commands).filter(c => ["StartFlow", "PlayTrack", "Pause", "Stop"].includes(c.command)).length, 0);
    assert.equal(changed.track, baseline.track);
    assert.ok(changed.queries.filter(q => q.query === "GetFlowSession").length > baseline.sessionQueries);
    assert.ok(changed.queries.filter(q => q.query === "GetPlaybackState").length > baseline.playbackQueries);
    assert.equal(changed.subscriptions, 1);
    await slider.press("Home");
    await page.getByText("Rediscovery mix · Deep cuts", { exact: true }).waitFor();
    assert.match(await slider.getAttribute("aria-valuetext"), /online discoveries/);
    if (style === "brutalist") {
      const commands = () => page.evaluate(() => window.__kazeFixture.commands.filter(c => c.command === "SetFlowSteering"));
      const beforeDrag = (await commands()).length;
      const box = await slider.boundingBox();
      await page.mouse.move(box.x + box.width * .2, box.y + box.height / 2);
      await page.mouse.down();
      await page.mouse.move(box.x + box.width * .8, box.y + box.height / 2, { steps: 12 });
      assert.equal(Number(await slider.inputValue()), .8);
      await page.waitForTimeout(350);
      assert.equal((await commands()).length, beforeDrag, "drag preview cannot send steering commands");
      await page.mouse.up();
      await page.getByText("Familiar mix · Your favorites", { exact: true }).waitFor();
      const saved = await commands();
      assert.equal(saved.length, beforeDrag + 1, "release commits once");
      assert.equal(saved.at(-1).payload.familiarity_ratio, .8);
      console.log("PASS Brutalist bar: mouse drag previews live and commits once on release");
    }
    assert.deepEqual(errors, []);
    console.log(`PASS ${style}: immediate confirmed label/queue refresh, debounce, current playback continuity and one subscription`);
    await context.close();
  }
} finally { await browser.close(); }
