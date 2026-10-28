import test from "node:test";
import assert from "node:assert/strict";
import { build } from "esbuild";
import { mkdtemp, writeFile, symlink, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";
const directory = await mkdtemp(join(tmpdir(), "kaze-song-controls-"));
let controls;
try {
  const result = await build({ stdin: { contents: `export { DownloadButton } from './src/components/DownloadButton'; export { downloadProgress } from './src/services/downloadProgress'; export { NotInterestedButton } from './src/components/NotInterestedButton'; export { recommendationPreferences } from './src/services/recommendationPreferences';`, resolveDir: process.cwd(), loader: "ts" }, bundle: true, format: "esm", write: false, packages: "external", jsx: "automatic" });
  await symlink(join(process.cwd(), "node_modules"), join(directory, "node_modules"));
  await writeFile(join(directory, "controls.mjs"), result.outputFiles[0].text);
  controls = await import(pathToFileURL(join(directory, "controls.mjs")));
} finally { await rm(directory, { recursive: true, force: true }); }
const song = { id: "transfer-one", provider: "jiosaavn", source_track_id: "recording-a", track_ids: ["saved-alias"], title: "Song", artist: "Artist", filename: "Artist - Song (recording-a).m4a", status: "DOWNLOADING", bytes_downloaded: 0, file_size: 100, created_at: 0 };
const button = (id, props = {}) => renderToStaticMarkup(React.createElement(controls.DownloadButton, { trackId: id, title: "Song", ...props }));
test("download fill follows the exact recording, survives saved aliases, and becomes a plain completed icon", () => {
  controls.downloadProgress.replace([song]);
  controls.downloadProgress.update(song.id, 41, 100);
  assert.match(button("saved-alias"), /scaleX\(0.41\)/);
  assert.match(button("saved-alias"), /41%/);
  assert.match(button("jiosaavn:recording-a"), /41%/);
  controls.downloadProgress.replace([{ ...song, bytes_downloaded: 0, file_size: 0 }]);
  assert.match(button("saved-alias"), /41%/, "a delayed task snapshot cannot rewind live progress");
  assert.doesNotMatch(button("jiosaavn:recording-b"), /41%|Downloaded Song/);
  controls.downloadProgress.replace([{ ...song, status: "COMPLETED", bytes_downloaded: 100 }]);
  assert.match(button("saved-alias"), /Downloaded Song/);
  assert.match(button("saved-alias"), /is-complete/);
  assert.doesNotMatch(button("saved-alias"), /song-download-fill|<span>Downloaded<\/span>/);
  assert.doesNotMatch(button("saved-alias"), /Downloading Song/);
  controls.downloadProgress.replace([song]);
  assert.match(button("saved-alias"), /Downloaded Song/, "a stale active snapshot cannot undo completion");
});
test("unknown size is honest 0% until bytes/total arrive, retries and unrelated songs stay independent", () => {
  controls.downloadProgress.replace([]);
  controls.downloadProgress.replace([{ ...song, file_size: 0 }]);
  assert.match(button("saved-alias"), /0%/);
  controls.downloadProgress.update(song.id, 100, 100);
  assert.match(button("saved-alias"), /99%/); // completion means the file has actually been committed
  controls.downloadProgress.replace([{ ...song, status: "FAILED" }]);
  assert.match(button("saved-alias"), /aria-label="Download Song"/);
  assert.doesNotMatch(button("saved-alias"), /disabled/);
});
test("not interested is a shared reversible pressed icon, including persisted hydration state", () => {
  const render = id => renderToStaticMarkup(React.createElement(controls.NotInterestedButton, { trackId: id, title: "Song" }));
  controls.recommendationPreferences.update(["song", "alias"], true);
  assert.match(render("song"), /aria-pressed="true"/);
  assert.match(render("alias"), /Allow Song in recommendations again/);
  controls.recommendationPreferences.update(["song", "alias"], false);
  assert.match(render("song"), /aria-pressed="false"/);
  assert.match(render("alias"), /aria-pressed="false"/);
  controls.recommendationPreferences.reset();
  assert.match(render("song"), /aria-pressed="false"/);
});
