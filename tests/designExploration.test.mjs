import test from "node:test";
import assert from "node:assert/strict";
import {
  DESIGN_MODELS,
  MOCK_TRACKS,
  CATEGORY_STAGES,
  EDITORIAL_FEATURE,
} from "../src/components/design-exploration/mockData.ts";

test("Design Exploration: preserves the 3 earlier standalone UI studies", () => {
  assert.equal(DESIGN_MODELS.length, 3);
  const ids = DESIGN_MODELS.map((m) => m.id);
  assert.deepEqual(ids, ["drift", "broadsheet", "gallery"]);

  // Ensure every model has distinct themes and layouts
  const themes = new Set(DESIGN_MODELS.map((m) => m.theme));
  assert.equal(themes.size, 3, "Each model must have a distinct visual theme");

  const shortcuts = DESIGN_MODELS.map((m) => m.keyShortcut);
  assert.deepEqual(
    shortcuts,
    ["1", "2", "3"],
    "Studies must retain their original shortcuts",
  );
});

test("Model A (Orbit Flow): tracks contain valid tempo, key, and SVG cover art", () => {
  assert.ok(MOCK_TRACKS.length >= 8);
  for (const track of MOCK_TRACKS) {
    assert.ok(track.id, "Track must have id");
    assert.ok(track.title, "Track must have title");
    assert.ok(track.artist, "Track must have artist");
    assert.ok(
      track.coverArt.startsWith("data:image/svg+xml"),
      "Cover art must be reliable SVG data URI",
    );
    assert.ok(
      typeof track.bpm === "number" && track.bpm > 0,
      "Track must have valid BPM",
    );
    assert.ok(track.key, "Track must have musical key");
  }
});

test("Model B (Scroll Stage): contains 5 narrative stages with in-place track streams", () => {
  assert.equal(CATEGORY_STAGES.length, 5);
  for (let i = 0; i < CATEGORY_STAGES.length; i++) {
    const stage = CATEGORY_STAGES[i];
    assert.equal(stage.number, `0${i + 1}`);
    assert.ok(stage.title);
    assert.ok(stage.subtitle);
    assert.ok(stage.description);
    assert.ok(
      stage.tracks.length >= 3,
      "Each stage must have multiple tracks for horizontal carousel",
    );
  }
});

test("Model C (Editorial Atelier): contains monograph essay and vinyl sleeve compilation", () => {
  assert.ok(EDITORIAL_FEATURE.issueNumber);
  assert.ok(EDITORIAL_FEATURE.title);
  assert.ok(
    EDITORIAL_FEATURE.essay.length > 200,
    "Essay must be substantial for editorial layout",
  );
  assert.ok(EDITORIAL_FEATURE.quote);
  assert.ok(EDITORIAL_FEATURE.quoteAuthor);
  assert.ok(EDITORIAL_FEATURE.primaryTrack);
  assert.ok(
    EDITORIAL_FEATURE.secondaryTracks.length >= 4,
    "Must have compilation tracklist",
  );
});
