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
  assert.match(view, /onToggleShuffle/);
  // Header and rows share one grid template with a column gap (no overlap).
  assert.equal((view.match(/LIBRARY_GRID_COLUMNS/g) ?? []).length >= 3, true);
  assert.match(view, /columnGap: "12px"/);
});

test("library playback dispatches ordered PlayTrackList", () => {
  const app = read("src/App.tsx");
  assert.match(app, /command: "PlayTrackList"/);
  // Search filters the view only: the list always plays the full library.
  const start = app.slice(app.indexOf("const startLibraryList"), app.indexOf("const handleLibraryRowPlay"));
  assert.match(start, /track_ids: null/);
  assert.doesNotMatch(start, /tracks\.map/);
});

test("Play all toggles pause/resume and shuffle is a per-section toggle", () => {
  const view = read("src/components/views/LibraryView.tsx");
  assert.match(view, /listState === "playing" \? "Pause"/);
  assert.match(view, /aria-pressed=\{shuffleEnabled\}/);
  const app = read("src/App.tsx");
  // Section-local shuffle state: never derived from the queue-wide is_shuffled flag.
  assert.match(app, /const \[libraryShuffle, setLibraryShuffle\] = useState<boolean>\(/);
  // Persisted across restarts under a section-specific key.
  assert.match(app, /localStorage\.getItem\(DOWNLOADS_SHUFFLE_KEY\) === "1"/);
  assert.match(app, /localStorage\.setItem\(DOWNLOADS_SHUFFLE_KEY/);
  assert.match(app, /command: "SetArchiveShuffle"/);
  assert.match(app, /playbackState\.source === "archive"/);
  assert.match(read("src/hooks/useScopedTracks.ts"), /online:\/\//);
  assert.match(view, /key=\{searchQuery\}/);
});
