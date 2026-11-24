import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { canRateFile as canRate } from "../src/downloadsFolder.ts";

const read = (f) => readFileSync(new URL(`../${f}`, import.meta.url), "utf8");
const KAZE = "/home/me/Music/Kaze";
const song = (file_path, format = "flac") => ({ id: "t", file_path, format });

test("songs from the user's own folders take no like or dislike", () => {
  assert.equal(canRate(song("/home/me/Music/fav/a.flac"), KAZE), false);
  assert.equal(canRate(song("/home/me/Music/Kaze Extras/a.flac"), KAZE), false, "a sibling folder is not Kaze's");
});

test("online songs and Kaze downloads can be rated", () => {
  assert.equal(canRate(song(`${KAZE}/a.m4a`), KAZE), true);
  assert.equal(canRate(song(`${KAZE}/a.m4a`), `${KAZE}/`), true);
  assert.equal(canRate(song("online://jiosaavn:x", "online"), KAZE), true);
  assert.equal(canRate(song("online://theirs"), KAZE), true, "another device's file: the backend decides");
  assert.equal(canRate({ id: "jiosaavn:x" }, KAZE), true, "a recommendation without a file");
  assert.equal(canRate(null, KAZE), false);
});

test("the rule follows the download folder from settings", () => {
  const rule = read("src/services/ratingRule.ts");
  assert.match(rule, /useSyncExternalStore\(subscribe, snapshot, snapshot\)/, "re-renders when the folder arrives");
  assert.match(read("src/hooks/app/useDownloadsSection.ts"), /setRatingDownloadsDir\(downloadsDir\)/);
  assert.equal(canRate({ id: "t", file_path: `${KAZE}/a.m4a` }, undefined), false, "unknown folder: every file counts as local");
});

test("every like and dislike control asks the rule (PC and phone)", () => {
  for (const file of [
    "src/components/NowPlayingBar.tsx",
    "src/components/views/LibraryView.tsx",
    "src/components/views/collection/CollectionTracks.tsx",
    "src/presentation/BrutalistDeck.tsx",
    "src/presentation/mobile/MobileApp.tsx",
    "src/presentation/mobile/MobileLyrics.tsx",
    "src/presentation/mobile/DeckParts.tsx",
    "src/presentation/mobile/CollectionRow.tsx",
    "src/presentation/mobile/LibraryPanes.tsx",
    "src/presentation/mobile/likeTrack.ts",
    "src/hooks/app/useLikes.ts",
  ]) assert.match(read(file), /canRate|useCanRate/, file);
  assert.match(read("src/presentation/mobile/sheets/TrackSheets.tsx"), /disabled=\{t\.localFile\}[\s\S]*disabled=\{t\.localFile\}/);
  assert.match(read("src/components/NotInterestedButton.tsx"), /disabled=\{busy \|\| !rateable\}/);
});
