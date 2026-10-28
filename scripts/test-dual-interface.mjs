/** Real App + controlled IPC fixture. Requires a running Vite server and Playwright. */
import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import { installDualInterfaceFixture } from "../tests/fixtures/dualInterfaceBackend.mjs";
const { chromium } = await import(
  process.env.KAZE_PLAYWRIGHT_MODULE ||
    "/home/abhi/.cache/ms-playwright-go/1.57.0/package/index.mjs"
);
const base = process.env.KAZE_TEST_URL || "http://127.0.0.1:5194";
const out = process.env.KAZE_TEST_OUTPUT || "/tmp/kaze-dual-interface";
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
await context.route("**/src/services/api.ts*", (route) =>
  route.fulfill({
    contentType: "text/javascript",
    body: `export const isTauri=()=>false;export const executeQuery=q=>window.__kazeFixture.query(q);export const dispatchCommand=c=>window.__kazeFixture.command(c);export const subscribeBackendEvents=async cb=>{window.__kazeFixture.listeners.add(cb);return ()=>window.__kazeFixture.listeners.delete(cb);};`,
  }),
);
const page = await context.newPage(),
  errors = [];
page.setDefaultTimeout(10000);
page.on("pageerror", (error) => errors.push(error.message));
const nav = async (name) => {
  if (name === "Now Playing") return page.locator(":is(.kaze-interface > .player-bar, .press-deck-controls)").getByRole("button", { name: "Open Now Playing", exact: true }).click();
  const direct = page.locator(".sidebar, .press-header, .press-account-navigation").getByRole("button", { name, exact: true });
  if (await direct.count()) return direct.click();
  if (["Albums", "Downloaded Tracks", "Playlists"].includes(name)) {
    await page.locator(".sidebar, .press-header, .press-account-navigation").getByRole("button", { name: "Library", exact: true }).click();
    return page.getByRole("tab", { name: name === "Downloaded Tracks" ? "Tracks" : name, exact: true }).click();
  }
  await page.getByRole("button", { name: "Library and more", exact: true }).click();
  await page.getByRole("menuitem", { name, exact: true }).click();
};
const check = async (name, fn) => {
  try {
    await fn();
    console.log("PASS", name);
  } catch (error) {
    await page.screenshot({ path: `${out}/failure.png` });
    throw error;
  }
};
const commands = () => page.evaluate(() => window.__kazeFixture.commands);
const style = () => page.locator("html").getAttribute("data-interface");
const select = async (name) => {
  await page.getByRole("button", { name: new RegExp(`^${name}`) }).click();
  await page.waitForFunction(
    (s) => document.documentElement.dataset.interface === s,
    name.toLowerCase(),
  );
};
try {
  await page.goto(base);
  await page.locator(".home-flow").waitFor();
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(400);
  await check(
    "Real entry uses one backend subscription and preserves all Home sections",
    async () => {
      assert.equal(
        await page.evaluate(() => window.__kazeFixture.listeners.size),
        1,
      );
      for (const label of [
        "Recent Rotation",
        "Rediscover",
        "For You",
        "Library Moment",
        "Your Week",
      ])
        assert.ok(
          await page.getByRole("heading", { name: label, exact: true }).count(),
        );
      assert.equal(
        await page.locator(".spatial-kaze,.brutalist-kaze,audio").count(),
        0,
      );
    },
  );
  for (const name of ["Spatial", "Brutalist"]) {
    await nav("Settings");
    await select(name);
    assert.equal(await style(), name.toLowerCase());
    await check(
      `${name}: simplified Settings and style persistence`,
      async () => {
        for (const label of [
          "Music Folders",
          "System",
        ])
          assert.ok(
            await page.getByRole("button", { name: new RegExp(label) }).count(),
          );
        assert.equal(
          await page.evaluate(
            () => window.__kazeFixture.settings.interface_style,
          ),
          name.toLowerCase(),
        );
      },
    );
    await check(
      `${name}: playback switching keeps route, track, queue, position and Flow`,
      async () => {
        const before = await page.evaluate(() => ({
          playback: JSON.stringify(window.__kazeFixture.playback),
          flow: JSON.stringify(window.__kazeFixture.flow),
          queries: window.__kazeFixture.queries.filter(
            (q) => q.query === "GetTracks",
          ).length,
        }));
        await page.evaluate(() => {
          window.appIdentity = document.querySelector(".kaze-interface");
          window.mainIdentity = document.querySelector(".main-content");
        });
        await select(name === "Spatial" ? "Brutalist" : "Spatial");
        await select(name);
        const after = await page.evaluate(() => ({
          playback: JSON.stringify(window.__kazeFixture.playback),
          flow: JSON.stringify(window.__kazeFixture.flow),
          queries: window.__kazeFixture.queries.filter(
            (q) => q.query === "GetTracks",
          ).length,
        }));
        assert.deepEqual(after, before);
        assert.equal(
          await page.evaluate(
            () =>
              window.appIdentity ===
                document.querySelector(".kaze-interface") &&
              window.mainIdentity === document.querySelector(".main-content"),
          ),
          true,
        );
        assert.equal(
          await page.locator(".main-content").getAttribute("data-view"),
          "settings",
        );
      },
    );
    await nav("Home");
    await check(
      `${name}: Flow, steering, explanation and complete Home`,
      async () => {
        const homeFlow = page.locator(".home-flow");
        if (await homeFlow.getByRole("button", { name: "Pause Flow", exact: true }).count()) {
          await homeFlow.getByRole("button", { name: "Pause Flow", exact: true }).click();
        }
        await homeFlow.getByRole("button", { name: "Start Flow", exact: true }).click();
        await homeFlow.getByRole("button", { name: "Pause Flow", exact: true }).waitFor();
        assert.equal(await page.getByRole("button", { name: "Stop Flow", exact: true }).count(), 0);
        await page.getByRole("slider", { name: "Flow Steering" }).focus();
        await page.keyboard.press("Home");
        for (let step = 0; step < 4; step++) await page.keyboard.press("ArrowRight");
        await page.waitForTimeout(300);
        assert.ok(
          (await commands()).some((c) => c.command === "SetFlowSteering"),
        );
        await page
          .getByRole("button", { name: "Why this track?", exact: true })
          .first()
          .click();
        await page.locator(".modal-overlay").waitFor();
        await page.getByRole("button", { name: "Close", exact: true }).click();
        if (name === "Brutalist") {
          assert.deepEqual(await page.locator(".press-home-week-stats dd").allTextContents(), ["4h 32m", "47", "18"]);
          const placeholder = page.locator(".press-home-moment .music-artwork--placeholder");
          assert.equal(await placeholder.locator("svg").count(), 1);
          assert.equal(await placeholder.evaluate(element => getComputedStyle(element).backgroundColor !== getComputedStyle(element.parentElement).backgroundColor), true);
          assert.equal(await page.locator(".press-home-moment-art").getAttribute("aria-label"), "Open album Migration");
        }
      },
    );
    await check(
      `${name}: library bounded rows, likes, enqueue and playlist actions`,
      async () => {
        await nav("Downloaded Tracks");
        await page.locator(".library-view .track-row").first().waitFor();
        assert.ok(
          (await page.locator(".library-view .track-row").count()) < 35,
        );
        await page
          .getByRole("button", { name: "Like track", exact: true })
          .first()
          .click();
        await page
          .getByRole("button", { name: "Add to queue", exact: true })
          .last()
          .click();
        await page
          .getByRole("button", {
            name: /Add to playlist|In playlist \(click to manage\)/,
          })
          .first()
          .click();
        await page.locator(".modal-overlay").waitFor();
        await page.getByRole("button", { name: "Done", exact: true }).click();
      },
    );
    await check(
      `${name}: album shelves and full collection actions`,
      async () => {
        await nav("Albums");
        await page
          .getByRole("button", { name: "Open album migration", exact: true })
          .first()
          .click();
        await page.locator(".library-view").waitFor();
        assert.equal(
          await page.locator(".main-content").getAttribute("data-view"),
          "library",
        );
        await nav("Playlists");
        await page.locator(".playlist-card").filter({ hasText: "After hours" }).click();
        await page.locator(".collection-view").waitFor();
        assert.ok((await page.locator(".collection-track-row").count()) < 30);
        assert.ok(
          await page
            .getByRole("button", {
              name: /Play Collection|Pause Collection/,
              exact: true,
            })
            .count(),
        );
      },
    );
    await check(`${name}: playlists, Liked Songs and smart mixes`, async () => {
      await nav("Playlists");
      assert.ok(await page.getByText("After hours", { exact: true }).count());
      assert.equal(await page.locator(".playlists-view .playlist-card").filter({ hasText: "Liked Songs" }).count(), 0);
      await page.getByRole("tab", { name: "Liked", exact: true }).click();
      await page.locator(".collection-view--liked").waitFor();
      assert.equal(await page.getByRole("heading", { name: "Liked Songs", exact: true }).isVisible(), true);
      await page.locator(".collection-track-row").first().waitFor();
      if (name === "Brutalist") {
        assert.equal(await page.locator(".press-library-heading").isVisible(), true);
        assert.equal(await page.getByRole("tab", { name: "Liked", selected: true, exact: true }).count(), 1);
        assert.equal(await page.getByRole("tab", { selected: true }).count(), 1);
        assert.equal(await page.getByRole("tabpanel", { name: "Liked Songs", exact: true }).count(), 1);
        assert.equal(await page.locator(".collection-view--archive .collection-hero").count(), 0);
        assert.equal((await page.locator(".interface-route").textContent()).trim(), "Library");
        await page.getByRole("button", { name: "Go back", exact: true }).click();
        await page.locator(".playlists-view").waitFor();
        await page.getByRole("button", { name: "Go forward", exact: true }).click();
        await page.getByRole("tab", { name: "Liked", selected: true, exact: true }).waitFor();
      }
      if (name === "Spatial") await nav("Smart Mixes");
      else {
        await nav("Discover Music");
        await page.getByRole("combobox", { name: "Discovery selection" }).selectOption("2");
      }
      assert.ok(
        await page.getByText("Rediscovery Mix", { exact: true }).count(),
      );
    });
    await check(`${name}: discovery chapters and search`, async () => {
      await nav("Discover Music");
      await page.locator(".discovery-chapters").waitFor();
      for (let i = 0; i < 4; i++) {
        if (name === "Spatial") await page.locator(".chapter-nav button").nth(i).click();
        else await page.getByRole("combobox", { name: "Discovery selection" }).selectOption(String(i));
      }
      await page
        .getByRole("textbox", { name: "Search music online" })
        .fill("Bonobo");
      await page
        .getByRole("textbox", { name: "Search music online" })
        .press("Enter");
      await page.waitForTimeout(250);
      assert.ok(
        await page.getByText("Search Results", { exact: false }).count(),
      );
    });
    await check(
      `${name}: queue real index / remove / clear controls`,
      async () => {
        await nav("Queue");
        assert.ok(await page.locator(".queue-entry").count());
        await page.locator(".queue-entry .queue-track").first().click();
        assert.ok(
          (await commands()).some(
            (c) => c.command === "PlayQueueIndex" && c.payload.index > 0,
          ),
        );
        await page
          .locator('.queue-entry button[aria-label^="Remove"]')
          .first()
          .click();
        assert.ok((await commands()).some((c) => c.command === "DequeueTrack"));
      },
    );
    await check(
      `${name}: now playing, transport, keyboard and progress isolation`,
      async () => {
        assert.equal(await page.locator(".sidebar, .press-header, .press-account-navigation").getByRole("button", { name: "Now Playing", exact: true }).count(), 0);
        await nav("Now Playing");
        await page.locator(".playback-stage, .press-poster-page").waitFor();
        for (const key of ["Enter", "Space"]) {
          await nav("Home");
          const before = await page.evaluate(() => ({ playback: window.__kazeFixture.playback, commands: window.__kazeFixture.commands }));
          await page.locator(":is(.kaze-interface > .player-bar, .press-deck-controls)").getByRole("button", { name: "Open Now Playing", exact: true }).press(key);
          await page.locator(".playback-stage, .press-poster-page").waitFor();
          assert.deepEqual(await page.evaluate(() => ({ playback: window.__kazeFixture.playback, commands: window.__kazeFixture.commands })), before, "thumbnail navigation preserves playback");
        }
        if (
          await page
            .locator(".player-bar, .press-deck-controls")
            .getByRole("button", { name: "Play", exact: true })
            .count()
        )
          await page
            .locator(".player-bar, .press-deck-controls")
            .getByRole("button", { name: "Play", exact: true })
            .click();
        await page
          .locator(".player-bar, .press-deck-controls")
          .getByRole("button", { name: "Pause", exact: true })
          .click();
        await page
          .locator(".player-bar, .press-deck-controls")
          .getByRole("button", { name: "Play", exact: true })
          .click();
        await page
          .locator(".player-bar, .press-deck-controls")
          .getByRole("button", { name: "Toggle Shuffle", exact: true })
          .click();
        await page
          .locator(".player-bar, .press-deck-controls")
          .getByRole("button", { name: /Loop current song|Turn off song loop/ })
          .click();
        await page.getByRole("slider", { name: "Volume" }).fill("0.6");
        await page.getByRole("slider", { name: "Seek", exact: true }).focus();
        await page.keyboard.press("ArrowRight");
        await page.locator(".interface-route").click();
        await page.keyboard.press("Space");
        await page.evaluate(() => {
          window.chromeIdentity = [
            document.querySelector(".sidebar, .press-header"),
            document.querySelector(".global-search"),
          ];
          window.chromeMutations = 0;
          window.chromeObserver = new MutationObserver((m) => {
            window.chromeMutations += m.length;
          });
          window.chromeIdentity.forEach(
            (el) =>
              el &&
              window.chromeObserver.observe(el, {
                attributes: true,
                childList: true,
                subtree: true,
                characterData: true,
              }),
          );
        });
        for (let i = 0; i < 20; i++) {
          await page.evaluate(
            (i) =>
              window.__kazeFixture.emit({
                event: "PlaybackPositionChanged",
                payload: { position_secs: 50 + i / 10, duration_secs: 240 },
              }),
            i,
          );
          await page.waitForTimeout(15);
        }
        assert.equal(await page.evaluate(() => window.chromeMutations), 0);
        await page.evaluate(() => window.chromeObserver.disconnect());
        await page.screenshot({
          path: `${out}/${name.toLowerCase()}-player-1440.png`,
        });
      },
    );
    await check(`${name}: fullscreen, lyrics, buffering and seek availability`, async () => {
      await nav("Now Playing");
      const fullscreenSearch = page.getByRole("textbox", { name: "Search music online", exact: true });
      await fullscreenSearch.fill("");
      await fullscreenSearch.press("f");
      assert.equal(await fullscreenSearch.inputValue(), "f");
      assert.equal(await page.locator(".fullscreen-player").count(), 0, "F types normally in search");
      await fullscreenSearch.fill("");
      await fullscreenSearch.blur();
      const commandsBeforeFullscreen = (await commands()).length;
      await page.keyboard.press("f");
      await page.locator(".fullscreen-player").waitFor();
      await page.waitForFunction(() => !!document.fullscreenElement);
      await page.keyboard.press("f");
      await page.locator(".fullscreen-player").waitFor({ state: "detached" });
      await page.waitForFunction(() => !document.fullscreenElement);
      assert.equal((await commands()).length, commandsBeforeFullscreen, "F does not change playback");
      await page.locator(":is(.kaze-interface > .player-bar, .press-deck-controls)").getByRole("button", { name: "Full Screen", exact: true }).click();
      await page.locator(".fullscreen-player").waitFor();
      await page.locator(".fullscreen-player").getByRole("button", { name: "Show Lyrics", exact: true }).click();
      await page.waitForTimeout(150);
      assert.ok(await page.locator(".fullscreen-player").getByRole("button", { name: "Hide Lyrics", exact: true }).count());
      await page.keyboard.press("Escape");
      await page.locator(".fullscreen-player").waitFor({state:"detached"});
      await page.evaluate(() => window.__kazeFixture.emit({event:"PlaybackBuffering",payload:{track_id:"online:buffering"}}));
      await page.locator(":is(.kaze-interface > .player-bar, .press-deck-controls)").getByRole("button",{name:"Loading song",exact:true}).waitFor();
      assert.equal(await page.getByRole("slider",{name:"Seek",exact:true}).isDisabled(),true);
      await page.evaluate(() => window.__kazeFixture.emit({event:"PlaybackError",payload:{message:"Fixture stream unavailable"}}));
      assert.equal(await page.getByRole("slider",{name:"Seek",exact:true}).isEnabled(),true);
      await nav("Home");
    });
    await check(
      `${name}: ${name === "Spatial" ? "Downloads, Wishlist, Notifications, " : ""}Profile and history`,
      async () => {
        if (name === "Spatial") {
          await nav("Downloads");
          await page.getByRole("heading", { name: "Downloads", exact: true }).waitFor();
          await nav("Wishlist");
          await page.getByRole("heading", { name: /Wishlist/ }).waitFor();
          await nav("Notifications");
          await page.getByRole("heading", { name: /Notifications/ }).waitFor();
        }
        await nav("Profile");
        await page.getByText(/Log in|Sign in|Your listening|Profile|Music Listener/i).first().waitFor();
        await nav("Home");
        await page.keyboard.press("Alt+ArrowLeft");
        assert.equal(
          await page.locator(".main-content").getAttribute("data-view"),
          "stats",
        );
        await page.keyboard.press("Alt+ArrowRight");
        assert.equal(
          await page.locator(".main-content").getAttribute("data-view"),
          "home",
        );
      },
    );
    await check(
      `${name}: maximized, normal and narrow desktop layouts`,
      async () => {
        for (const [width, height] of [
          [1920, 1080],
          [1440, 900],
          [1024, 600],
          [800, 700],
        ]) {
          await page.setViewportSize({ width, height });
          await nav("Home");
          await page.locator(".main-content").evaluate(el=>el.scrollTop=0);
          await page.waitForTimeout(80);
          assert.equal(
            await page.evaluate(
              () => document.documentElement.scrollWidth <= innerWidth + 1,
            ),
            true,
          );
          const footer = await page
            .locator(":is(.kaze-interface > .player-bar, .press-deck-controls)")
            .boundingBox();
          assert.ok(footer.x >= 0 && footer.x + footer.width <= width + 1);
          await page.screenshot({
            path: `${out}/${name.toLowerCase()}-home-${width}.png`,
          });
          for (const destination of ["Now Playing", "Discover Music", "Albums", "Queue"]) {
            await nav(destination); await page.locator(".main-content").evaluate(el=>el.scrollTop=0); await page.waitForTimeout(80);
            assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1),true);
            if (destination === "Discover Music" && name === "Spatial" && height <= 700) {
              const bottom = (await page.locator(":is(.kaze-interface > .player-bar, .press-deck-controls)").boundingBox()).y;
              const overlaps = await page.locator('.discovery-chapter:not([hidden]) .discovery-track-card button').evaluateAll((buttons,bottom)=>buttons.filter(button=>{const r=button.getBoundingClientRect();return r.width>0&&r.left>=0&&r.left<innerWidth&&r.bottom>bottom+1;}).length,bottom);
              assert.equal(overlaps,0,"Discovery actions must remain above the persistent player");
            }
            if (destination === "Now Playing") {
              const caption = await page.locator(".stage-caption, .press-poster-annotation").boundingBox();
              assert.ok(caption.width > 100 && caption.x >= 0 && caption.x + caption.width <= width + 1);
            }
            await page.screenshot({path:`${out}/${name.toLowerCase()}-${destination.toLowerCase().replaceAll(" ","-")}-${width}.png`});
          }

        }
        await page.setViewportSize({ width: 1440, height: 900 });
      },
    );
  }
  await check(
    "Failed preference save restores selection and presents retryable error",
    async () => {
      await nav("Settings");
      const prior = await style();
      await page.evaluate(() => (window.__kazeFixture.failStyle = true));
      await page
        .getByRole("button", {
          name: new RegExp(prior === "spatial" ? "^Brutalist" : "^Spatial"),
        })
        .click();
      await page.getByRole("alert").filter({ hasText: "Couldn’t save Interface Style" }).waitFor();
      assert.equal(await style(), prior);
      await page.evaluate(() => (window.__kazeFixture.failStyle = false));
    },
  );
  await check(
    "Saved style loads after reload without legacy routing",
    async () => {
      const prior = await style();
      await page.reload();
      await page.locator(".home-flow").waitFor();
      await page.waitForTimeout(200);
      assert.equal(await style(), prior);
    },
  );
  for (const name of ["Spatial","Brutalist"]) {
    await check(`${name}: search rows download directly without a source dialog`, async () => {
      await nav("Settings"); await select(name); await nav("Discover Music");
      const search=page.getByRole("textbox",{name:"Search music online"});
      await search.fill("Bonobo"); await search.press("Enter");
      const download=page.locator('.online-search-song .song-download-button').first();
      await download.waitFor(); const id=await download.getAttribute('data-track-id'); await download.click();
      assert.ok((await commands()).some(command=>command.command==="DownloadTrack" && command.payload.track_id===id));
      assert.equal(await page.locator('[aria-label="Download sources"]').count(),0);
      await search.fill(""); await search.press("Enter");
    });
  }
  await check("Both presentations retain empty library/discovery/queue and failed search recovery", async () => {
    for (const name of ["Spatial","Brutalist"]) {
      await nav("Settings"); await select(name);
      await page.evaluate(() => {window.__kazeFixture.empty=true;window.__kazeFixture.playback.queue_track_ids=[];});
      await nav("Downloaded Tracks");await page.getByText("No tracks found in library",{exact:true}).waitFor();
      await nav("Queue");
      const clear=page.getByRole("button",{name:"Clear queue",exact:true});
      if(await clear.isEnabled())await clear.click();
      await page.getByText(/Your queue is open/).waitFor();
      await nav("Discover Music");
      // Returning to Discovery now keeps the previous chapter. Select the
      // Trending bin explicitly to exercise its empty/refresh behavior.
      await page.locator(".discovery-chapters").waitFor();
      if (await page.locator(".chapter-nav button").count()) await page.locator(".chapter-nav button").nth(0).click();
      else await page.getByRole("combobox", { name: "Discovery selection" }).selectOption("0");
      await page.getByRole("button",{name:/^Refresh trending songs$/i}).click();
      await page.getByText("No trending songs found",{exact:true}).waitFor();
      if (await page.locator(".chapter-nav button").count()) await page.locator(".chapter-nav button").nth(1).click();
      else await page.getByRole("combobox", { name: "Discovery selection" }).selectOption("1");
      await page.getByText("Play or add more music to your library to shape new recommendations.",{exact:true}).waitFor();
      await page.getByRole("textbox",{name:"Search music online"}).fill("unavailable");
      await page.evaluate(()=>window.__kazeFixture.failSearch=true);
      await page.getByRole("textbox",{name:"Search music online"}).press("Enter");await page.waitForTimeout(200);
      await page.getByRole("heading", {name:/No songs found for/i}).waitFor();
      await page.evaluate(()=>{window.__kazeFixture.failSearch=false;window.__kazeFixture.empty=false;});
      await nav("Home");
    }
  });
  assert.deepEqual(errors, []);
  console.log("PASS no page errors");
} finally {
  await browser.close();
}
