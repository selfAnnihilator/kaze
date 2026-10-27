import assert from "node:assert/strict";
import { installDualInterfaceFixture } from "../tests/fixtures/dualInterfaceBackend.mjs";
const { chromium } = await import(process.env.KAZE_PLAYWRIGHT_MODULE || "/home/abhi/.cache/ms-playwright-go/1.57.0/package/index.mjs");
const browser = await chromium.launch({ executablePath: "/usr/bin/chromium", headless: true, args: ["--no-sandbox"] });
try {
  for (const style of ["spatial", "brutalist"]) {
    const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
    await context.addInitScript({ content: `sessionStorage.setItem('test.interface-style','${style}');(${installDualInterfaceFixture.toString()})();` });
    await context.addInitScript(() => {
      const f = window.__kazeFixture, query = f.query.bind(f), command = f.command.bind(f);
      const hidden = new Set(["jiosaavn:recordingA"]), downloads = [];
      const songs = ["A", "B"].map(id => ({ external_track_id: `jiosaavn:recording${id}`, provider: "jiosaavn", provider_id: `recording${id}`, playback_source: { provider: "jiosaavn", track_id: `recording${id}` }, title: "Ocean Test", artist: "Fixture Artist", album: `Release ${id}`, cover_art_url: "/design-lab/artwork/migration.jpg", duration_secs: 200, match_status: "NOT_FOUND", recommendation_reason: "Based on your taste" }));
      f.playback.current_track = { ...f.playback.current_track, id: songs[0].external_track_id, title: songs[0].title, artist_name: songs[0].artist, album_title: songs[0].album, format: "online", file_path: "online://jiosaavn/recordingA", playback_source: songs[0].playback_source };
      f.playback.current_track_id = songs[0].external_track_id;
      f.query = async q => {
        if (["GetRecommendationFeedback", "GetDownloads", "GetCharts", "RefreshCharts", "RefreshSmartMixes", "SearchOnlineMusic", "GetWorldTrending", "GetDiscoveryRecommendations"].includes(q.query)) {
          f.queries.push(q);
          if (q.query === "GetRecommendationFeedback") return { type: "RecommendationFeedback", data: [...hidden] };
          if (q.query === "GetDownloads") return { type: "Downloads", data: downloads };
          if (["GetCharts", "RefreshCharts"].includes(q.query)) return { type: "Charts", data: [{ id: "123", title: "International Top Songs", image: "/design-lab/artwork/migration.jpg" }] };
          if (q.query === "RefreshSmartMixes") return { type: "SmartMixes", data: f.playlists.filter(p => p.is_smart_mix) };
          return { type: "DiscoveryRecommendations", data: q.query === "SearchOnlineMusic" ? songs : songs.filter(song => !hidden.has(song.external_track_id)) };
        }
        return query(q);
      };
      f.command = async c => {
        if (c.command === "DownloadTrack") {
          f.commands.push(c);
          const id = c.payload.track_id;
          downloads.push({ id: "exact-download", source_track_id: id.slice(9), track_ids: [id], provider: "jiosaavn", title: "Ocean Test", artist: "Fixture Artist", filename: "Ocean Test.m4a", file_size: 100, bytes_downloaded: 0, status: "DOWNLOADING", created_at: 1 });
          return { status: "DownloadStarted", data: { task_id: "exact-download" } };
        }
        if (["DislikeTrack", "RemoveTrackFeedback"].includes(c.command)) {
          f.commands.push(c); const id = c.payload.track_id, disliked = c.command === "DislikeTrack";
          if (disliked) hidden.add(id); else hidden.delete(id);
          f.emit({ event: "RecommendationFeedbackChanged", payload: { user_id: "u1", track_id: id, track_ids: [id], title: "Ocean Test", disliked, manual_like: disliked ? -1 : 0 } });
          f.emit({ event: "TasteProfileChanged", payload: { user_id: "u1" } });
          return { status: "Ok" };
        }
        return command(c);
      };
      f.finishDownload = () => { downloads[0].status = "COMPLETED"; downloads[0].bytes_downloaded = 100; f.emit({ event: "DownloadCompleted", payload: { task_id: "exact-download", file_path: "/fixture/Ocean Test.m4a" } }); };
    });
    await context.route("**/src/services/api.ts*", route => route.fulfill({ contentType: "text/javascript", body: `export const isTauri=()=>false;export const executeQuery=q=>window.__kazeFixture.query(q);export const dispatchCommand=c=>window.__kazeFixture.command(c);export const subscribeBackendEvents=async cb=>{window.__kazeFixture.listeners.add(cb);return ()=>window.__kazeFixture.listeners.delete(cb);};` }));
    const page = await context.newPage(), errors = [];
    page.on("pageerror", error => errors.push(error.message)); page.setDefaultTimeout(10000);
    await page.goto(process.env.KAZE_TEST_URL || "http://127.0.0.1:5195", { waitUntil: "domcontentloaded" });
    assert.equal(await page.getByLabel("Online music source").count(), 0);
    const search = page.getByRole("textbox", { name: "Search music online" });
    await search.fill("Ocean Test"); await search.press("Enter");
    const toggle = page.locator('.not-interested-button[data-track-id="jiosaavn:recordingA"]').first();
    await toggle.waitFor(); await page.waitForFunction(() => document.querySelector('.not-interested-button[data-track-id="jiosaavn:recordingA"]')?.getAttribute("aria-pressed") === "true");
    await toggle.click(); await page.waitForFunction(() => document.querySelector('.not-interested-button[data-track-id="jiosaavn:recordingA"]')?.getAttribute("aria-pressed") === "false");
    await toggle.click(); await page.waitForFunction(() => document.querySelector('.not-interested-button[data-track-id="jiosaavn:recordingA"]')?.getAttribute("aria-pressed") === "true");
    const download = page.locator('.song-download-button[data-track-id="jiosaavn:recordingA"]').first();
    await download.click();
    await page.waitForFunction(() => window.__kazeFixture.commands.some(c => c.command === "DownloadTrack" && c.payload.track_id === "jiosaavn:recordingA"));
    assert.equal(await page.getByRole("dialog").count(), 0);
    await page.evaluate(() => window.__kazeFixture.emit({ event: "DownloadProgressChanged", payload: { task_id: "exact-download", bytes_downloaded: 42, total_bytes: 100 } }));
    await page.waitForFunction(() => document.querySelector('.song-download-button[data-track-id="jiosaavn:recordingA"]')?.textContent.includes("42%"));
    assert.match(await download.locator('.song-download-fill').getAttribute('style'), /scaleX\(0.42\)/);
    const playerDownload = page.locator('.player-bar .song-download-button,.press-deck .song-download-button').first();
    await playerDownload.waitFor();
    assert.ok(await playerDownload.evaluate(button => {
      const bounds=button.getBoundingClientRect(); const row=button.closest('.press-options-row')?.getBoundingClientRect();
      return bounds.x>=0 && bounds.right<=innerWidth && (!row || bounds.right<=row.right+1);
    }), "download percentage must fit inside the player controls");
    assert.equal(await page.locator('.song-download-button[data-track-id="jiosaavn:recordingB"]').first().getAttribute('disabled'), null);
    await page.evaluate(() => window.__kazeFixture.finishDownload());
    await page.waitForFunction(() => document.querySelector('.song-download-button[data-track-id="jiosaavn:recordingA"]')?.getAttribute("aria-label") === "Downloaded Ocean Test");
    await search.fill(""); await search.press("Enter");
    for (let chapter = 0; chapter < 4; chapter++) {
      if (style === "brutalist") await page.getByLabel("Discovery selection", { exact: true }).selectOption(String(chapter));
      else await page.locator('.chapter-nav button').nth(chapter).click();
      const refresh = page.locator('.discovery-refresh-control button'); await refresh.waitFor(); await refresh.click();
      await page.waitForFunction(() => !document.querySelector('.discovery-refresh-control button')?.disabled);
    }
    assert.ok(await page.evaluate(() => window.__kazeFixture.queries.some(q => q.query === "RefreshCharts")));
    assert.ok(await page.evaluate(() => window.__kazeFixture.queries.some(q => q.query === "RefreshSmartMixes")));
    assert.ok(await page.locator('.chart-cover img').count());
    assert.deepEqual(errors, []);
    await page.screenshot({ path: `/tmp/kaze-song-controls-${style}.png` });
    console.log(`PASS ${style}: persisted toggle on/off, exact direct M4A download, per-recording percentage/fill/completion, all four refresh controls, real chart cover, no source badge`);
    await context.close();
  }
} finally { await browser.close(); }
