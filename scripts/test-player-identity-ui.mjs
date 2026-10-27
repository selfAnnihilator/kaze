import assert from "node:assert/strict";
import { installDualInterfaceFixture } from "../tests/fixtures/dualInterfaceBackend.mjs";
const { chromium } = await import(process.env.KAZE_PLAYWRIGHT_MODULE || "/home/abhi/.cache/ms-playwright-go/1.57.0/package/index.mjs");
const browser = await chromium.launch({ executablePath: process.env.KAZE_CHROMIUM || "/usr/bin/chromium", headless: true, args: ["--no-sandbox"] });
const failures = [];
try {
  for (const style of ["brutalist", "spatial"]) {
    const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
    await context.addInitScript({ content: `sessionStorage.setItem('test.interface-style','${style}');(${installDualInterfaceFixture.toString()})();` });
    await context.route("**/src/services/api.ts*", route => route.fulfill({ contentType: "text/javascript", body: `export const isTauri=()=>false;export const executeQuery=q=>window.__kazeFixture.query(q);export const dispatchCommand=c=>window.__kazeFixture.command(c);export const subscribeBackendEvents=async cb=>{window.__kazeFixture.listeners.add(cb);return ()=>window.__kazeFixture.listeners.delete(cb);};` }));
    const page = await context.newPage(), keyWarnings = [], errors = [];
    page.setDefaultTimeout(10000);
    page.on("console", message => { if (message.text().includes("same key")) keyWarnings.push(message.text()); });
    page.on("pageerror", error => errors.push(error.message));
    await page.goto(process.env.KAZE_TEST_URL || "http://127.0.0.1:5196", { waitUntil: "domcontentloaded" });
    await page.waitForFunction(() => window.__kazeFixture.listeners.size === 1);
    const player = page.locator(style === "brutalist" ? ".press-deck" : ".player-bar").first();
    const counts = [];
    for (let index = 0; index < 12; index++) {
      const online = index < 9 || index === 11;
      const artist = index === 9 ? null : index === 10 ? "   " : `Artist ${index}`;
      const track = { id: `identity:${index}`, title: index === 9 ? "Driftwood Dreamweave - Kato" : `Identity song ${index}`, artist_name: artist, album_title: "Album", duration_secs: 70.75, file_path: online ? `online://jiosaavn/id${index}` : "/fixture/untagged.flac", format: online ? "online" : "flac", has_cover_art: 0, ...(online ? { playback_source: { provider: "jiosaavn", track_id: `id${index}` } } : {}) };
      await page.evaluate(track => {
        const f = window.__kazeFixture;
        Object.assign(f.playback, { current_track: track, current_track_id: track.id, is_playing: true, position_secs: 5, duration_secs: track.duration_secs });
        // Match the flat Rust start event, then its authoritative metadata snapshot.
        f.emit({ event: "PlaybackStarted", payload: { session_id: `session-${track.id}`, track_id: track.id, title: track.title, artist: track.artist_name || "", duration_secs: track.duration_secs, source: "library" } });
        f.emit({ event: "LibraryMetadataUpdated", payload: {} });
      }, track);
      await player.locator(`.not-interested-button[data-track-id="${track.id}"]`).waitFor();
      await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
      const count = await player.locator(".not-interested-button").count();
      counts.push(count);
      if (count !== 1) failures.push(`${style} step ${index}: ${count} dislike buttons`);
      const displayedArtist = style === "brutalist" ? await player.locator('.press-deck-identity p').innerText() : await player.locator(`[title="${artist?.trim() || "Unknown Artist"}"]`).count() ? artist?.trim() || "Unknown Artist" : await player.innerText();
      if (displayedArtist !== (artist?.trim() || "Unknown Artist")) failures.push(`${style} step ${index}: artist ${JSON.stringify(displayedArtist)}`);
      if (await player.locator(".song-download-button").count() !== Number(online)) failures.push(`${style} step ${index}: wrong download control count`);
      if (await page.evaluate(() => window.__kazeFixture.listeners.size) !== 1) failures.push(`${style} step ${index}: duplicated backend subscription`);
    }
    if (keyWarnings.length) failures.push(`${style}: ${keyWarnings.length} duplicate-key warnings`);
    assert.deepEqual(errors, []);
    await page.screenshot({ path: `/tmp/kaze-player-identity-${style}.png` });
    console.log(JSON.stringify({ style, counts, duplicateKeyWarnings: keyWarnings.length }));
    await context.close();
  }
  assert.deepEqual(failures, [], failures.join("\n"));
  console.log("PASS both players: repeated online/local changes keep one dislike, correct download count, real artist or Unknown Artist, one subscription and no duplicate keys");
} finally { await browser.close(); }
