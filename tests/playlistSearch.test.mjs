import test from "node:test";
import assert from "node:assert/strict";
import { filterPlaylistEntries, matchesPlaylistSearch } from "../src/playlistSearch.ts";

const track = { title: "Pokémon: Lavender Town", artist: "Kato", album: "Red Eye Reflections" };

test("playlist search ignores case, accents, punctuation and word order", () => {
  for (const query of ["lavender town", "LAVENDER TOWN", "pokemon", "KATO lavender", "town red eye"]) {
    assert.equal(matchesPlaylistSearch(track, query), true, query);
  }
  assert.equal(matchesPlaylistSearch(track, "littleroot town"), false);
});

test("duplicate track IDs keep separate stable rows through search and clear", () => {
  const tracks = [
    { ...track, id: "repeat" },
    { ...track, id: "other", title: "Littleroot Town" },
    { ...track, id: "repeat", title: "Lavender City" },
  ];
  const matches = filterPlaylistEntries(tracks, "LAVENDER CITY");
  assert.deepEqual(matches.map(({ originalIndex }) => originalIndex), [2]);
  const cleared = filterPlaylistEntries(tracks, "");
  assert.equal(new Set(cleared.map(({ rowKey }) => rowKey)).size, 3);
  assert.equal(matches[0].rowKey, cleared[2].rowKey);
});
