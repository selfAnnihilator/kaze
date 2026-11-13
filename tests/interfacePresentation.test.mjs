import test from "node:test";
import assert from "node:assert/strict";
import {
  upcomingFromEvent,
  upcomingFromPlayback,
  withKnownMetadata,
} from "../src/presentation/queuePresentation.ts";

test("queue adapter retains absolute backend indices and duplicate occurrence identity", () => {
  const items = ["past", "current", "same", "same", "tail"].map(
    (track_id, i) => ({
      queue_id: `q${i}`,
      track_id,
      title: track_id,
      artist: "Artist",
    }),
  );
  assert.deepEqual(
    upcomingFromEvent(items, 1).map((e) => [e.key, e.trackId, e.index]),
    [
      ["q2", "same", 2],
      ["q3", "same", 3],
      ["q4", "tail", 4],
    ],
  );
  assert.equal(items.length, 5);
  assert.deepEqual(
    upcomingFromEvent(items, null).map((e) => e.index),
    [0, 1, 2, 3, 4],
  );
});
test("upcoming-only snapshots never reset absolute queue positions", () => {
  const result = upcomingFromPlayback({
    current_track_id: "current",
    current_queue_index: 17,
    queue_track_ids: ["a", "b"],
  });
  assert.deepEqual(
    result.map((e) => e.index),
    [18, 19],
  );
});
test("stopped and not-yet-started queues expose the full queue from zero", () => {
  assert.deepEqual(
    upcomingFromPlayback({
      current_track_id: null,
      current_queue_index: 5,
      queue_track_ids: ["a", "b"],
    }).map((e) => e.index),
    [0, 1],
  );
  assert.deepEqual(
    upcomingFromPlayback({
      current_queue_index: null,
      queue_track_ids: ["a", "b"],
    }).map((e) => e.index),
    [0, 1],
  );
});

test("playback snapshot keeps titles learned from queue events", () => {
  const prev = upcomingFromEvent(
    [
      { queue_id: "q0", track_id: "a", title: "Now", artist: "X" },
      { queue_id: "q1", track_id: "online-1", title: "Bad Romance", artist: "Lady Gaga" },
    ],
    0,
  );
  const snapshot = upcomingFromPlayback({
    current_queue_index: 0,
    current_track_id: "a",
    queue_track_ids: ["online-1", "online-2"],
  });
  const merged = withKnownMetadata(snapshot, prev);
  assert.equal(merged[0].title, "Bad Romance");
  assert.equal(merged[0].artist, "Lady Gaga");
  assert.equal(merged[1].title, undefined);
  assert.deepEqual(withKnownMetadata(snapshot, []), snapshot);
});
