import test from "node:test";
import assert from "node:assert/strict";
import { removeRecommendationTracks } from "../src/recommendationFeedback.ts";

test("feedback removes aliases from recommendation arrays without mutating responses or source pointers", () => {
  const input = { for_you: [{ track_id: "saved", title: "Song" }, { track_id: "keep", title: "Other", playback_source: { track_id: "saved", provider: "jiosaavn" } }], recent_rotation: [{ id: "canonical", title: "Song" }], flow_session: { last_track_id: "saved" } };
  const result = removeRecommendationTracks(input, new Set(["saved", "canonical"]));
  assert.deepEqual(result.for_you.map(song => song.track_id), ["keep"]);
  assert.deepEqual(result.recent_rotation, []);
  assert.deepEqual(result.for_you[0].playback_source, input.for_you[1].playback_source);
  assert.equal(input.for_you.length, 2);
  assert.equal(result.flow_session.last_track_id, "saved");
});
