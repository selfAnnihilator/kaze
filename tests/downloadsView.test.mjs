import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const read = (p) => readFileSync(new URL(`../${p}`, import.meta.url), "utf8");

test("tracks tab is renamed Downloads and archive heading is unchanged", () => {
  const nav = read("src/presentation/BrutalistNavigation.tsx");
  assert.match(nav, /\["library", "Downloads"\]/);
  assert.match(nav, /\["albums", "Library"\]/);
  assert.match(nav, /ARCHIVE\./);
  assert.match(read("src/presentation/SpatialUtilities.tsx"), /\["library", "Downloads"\]/);
});

test("library tracks view has no indexed/loaded text and offers play all + shuffle", () => {
  const view = read("src/components/views/LibraryView.tsx");
  assert.doesNotMatch(view, /indexed/);
  assert.doesNotMatch(view, /loaded of/);
  assert.match(view, /onPlayAll/);
  assert.match(view, /onShuffleAll/);
  // Header and rows share one grid template with a column gap (no overlap).
  assert.equal((view.match(/LIBRARY_GRID_COLUMNS/g) ?? []).length >= 3, true);
  assert.match(view, /columnGap: "12px"/);
});

test("library playback dispatches ordered PlayTrackList", () => {
  const app = read("src/App.tsx");
  assert.match(app, /command: "PlayTrackList"/);
});
