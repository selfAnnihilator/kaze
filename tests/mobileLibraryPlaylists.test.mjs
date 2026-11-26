import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { isPersonalPlaylist } from "../src/presentation/mobile/playlistCollection.ts";

const source = (file) => readFileSync(new URL(`../src/presentation/mobile/${file}`, import.meta.url), "utf8");

test("only personal playlists count toward the Android Playlists tab", () => {
  const dailyMix = { id: "mix-1", name: "Daily Mix", is_smart_mix: 1 };
  const savedCopy = { id: "pl-1", name: "Daily Mix", is_smart_mix: 0 };
  const imported = { id: "pl-2", name: "Imported", is_smart_mix: 0 };
  const liked = { id: "pl_liked_default", name: "Liked Songs", is_smart_mix: 0 };
  const playlists = [dailyMix, savedCopy, imported, liked];

  assert.deepEqual(playlists.filter(isPersonalPlaylist), [savedCopy, imported]);
});

test("the Android playlist rows and tab count use the same filtered collection", () => {
  const library = source("MobileLibrary.tsx");
  assert.match(library, /c\.playlists\.filter\(isPersonalPlaylist\)/);
  assert.match(library, /counts=\{libraryCounts\(c, playlists\.length\)\}/);
  assert.match(library, /<PlaylistsPane c=\{c\} playlists=\{playlists\}/);
  assert.match(source("LibraryPanes.tsx"), /<WindowedList items=\{playlists\}/);
});
