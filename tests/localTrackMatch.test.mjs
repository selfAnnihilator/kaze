import test from "node:test";
import assert from "node:assert/strict";
import { attachLocalSearchMatches, sameSongMetadata } from "../src/localTrackMatch.ts";

const recommendation = {
  external_track_id: "itunes:pasoori",
  provider: "itunes",
  provider_id: "pasoori",
  title: "Pasoori",
  artist: "Shae Gill & Ali Sethi",
  duration_secs: 224,
  match_status: "NOT_FOUND",
  recommendation_reason: "search",
  in_wishlist: false,
};
const local = {
  id: "local-flac",
  file_path: "/music/Pasoori.flac",
  title: "Pasoori",
  artist_name: "Ali Sethi, Shae Gill",
  duration_secs: 224.146,
  format: "FLAC",
  has_cover_art: 0,
};

test("existing local recording is attached despite reversed collaboration credits", () => {
  assert.equal(sameSongMetadata("Pasoori", "Shae Gill & Ali Sethi", "Pasoori", "Ali Sethi, Shae Gill"), true);
  const [matched] = attachLocalSearchMatches([recommendation], [local]);
  assert.equal(matched.matched_local_track_id, local.id);
  assert.equal(matched.match_status, "EXACT_MATCH");
});

test("library refresh attaches a newly imported download without another online search", () => {
  const [before] = attachLocalSearchMatches([recommendation], []);
  const [after] = attachLocalSearchMatches([recommendation], [local]);
  assert.equal(before.matched_local_track_id, undefined);
  assert.equal(after.matched_local_track_id, local.id);
});

test("a cover, different recording length, and online placeholder do not count as local", () => {
  assert.equal(attachLocalSearchMatches([recommendation], [{ ...local, artist_name: "Ali Sethi" }])[0].matched_local_track_id, undefined);
  assert.equal(attachLocalSearchMatches([recommendation], [{ ...local, duration_secs: 276 }])[0].matched_local_track_id, undefined);
  assert.equal(attachLocalSearchMatches([recommendation], [{ ...local, format: "online", file_path: "online://pasoori" }])[0].matched_local_track_id, undefined);
});
