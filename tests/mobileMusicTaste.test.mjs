import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { mergeArtistChoices } from "../src/presentation/mobile/settings/artistChoices.ts";

const read = (name) => readFileSync(new URL(`../src/presentation/mobile/settings/${name}`, import.meta.url), "utf8");

test("Android taste setup accepts language-only choices and keeps the selection on save", () => {
  const screen = read("MobileMusicTaste.tsx");
  const hook = read("useMusicTaste.ts");
  assert.match(screen, /<TasteChoices label="LANGUAGES".+onToggle=\{t\.toggleLanguage\}/);
  assert.match(screen, /hasChoices = t\.languages\.length \+ t\.artists\.length \+ t\.genres\.length > 0/);
  assert.match(screen, /t\.save\(!hasChoices\)/);
  assert.match(read("tasteLanguages.ts"), /TASTE_LANGUAGES = \["English", "Hindi", "Malayalam"/);
  assert.match(hook, /setLanguages\(result\.data\.languages \?\? \[\]\)/);
  assert.match(hook, /SaveRecommendationSetup".+genres, languages, skipped/);
});

test("the last step offers a language pool and related artists while retaining picks", () => {
  const screen = read("MobileMusicTaste.tsx");
  assert.match(screen, /chooseArtists \? <TasteArtists languages=\{t\.languages\}/);
  assert.match(screen, /setChooseArtists\(true\)\}>NEXT/);
  assert.match(read("TasteArtists.tsx"), /useTasteArtistPicks\(languages, selected\[selected\.length - 1\]\)/);
  assert.deepEqual(mergeArtistChoices(["A. R. Rahman"], ["a. r. rahman", "Shreya Ghoshal", "K. S. Chithra"]), [
    "A. R. Rahman", "Shreya Ghoshal", "K. S. Chithra",
  ]);
});

test("Android can select every offered genre and any number of artists", () => {
  const screen = read("MobileMusicTaste.tsx");
  const hook = read("useMusicTaste.ts");
  assert.match(screen, /label="GENRES".+disabled=\{blocked\} onToggle=\{t\.toggleGenre\}/);
  assert.match(hook, /toggleGenre:.+toggleIn\(genre, values\)/);
  assert.match(hook, /toggleArtist:.+toggleIn\(name, values\)\)/);
  assert.doesNotMatch(hook, /ARTIST_LIMIT|limit/);
  assert.match(read("TasteArtists.tsx"), /aria-pressed=\{on\}\s*disabled=\{disabled\} onClick/);
  assert.match(read("TasteChoices.tsx"), /disabled=\{disabled\} onClick=\{\(\) => onToggle\(name\)\}/);
});
