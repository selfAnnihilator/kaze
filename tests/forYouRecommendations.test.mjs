import test from "node:test";
import assert from "node:assert/strict";
import { splitDiscoveryRecommendations } from "../src/forYouRecommendations.ts";

const recommendation = (id, title, reason = "Trending Chart Hit") => ({
  external_track_id: id,
  provider: "itunes",
  provider_id: id,
  title,
  artist: "Artist",
  duration_secs: 180,
  match_status: "NOT_FOUND",
  recommendation_reason: reason,
  in_wishlist: false,
});

test("For You favors taste signals and avoids saved, local, and downloading songs", () => {
  const recs = [
    recommendation("generic", "Generic"),
    recommendation("personal", "Personal", "Similar to your listening habit"),
    recommendation("playlist", "Playlist"),
    recommendation("local", "Owned"),
    recommendation("completed", "Completed"),
    recommendation("active", "Active"),
    recommendation("liked", "Liked"),
    recommendation("personal-copy", "Personal", "Matches your top genre"),
    recommendation("genre", "Genre", "Matches your top genre"),
    recommendation("other1", "Other One"),
    recommendation("other2", "Other Two"),
    recommendation("other3", "Other Three"),
  ];
  const localTracks = [{
    id: "file-owned", file_path: "/music/owned.flac", title: "Owned",
    artist_name: "Artist", duration_secs: 180, format: "FLAC", has_cover_art: 0,
  }];
  const downloads = [
    { id: "finished", title: "Completed", artist: "Artist", status: "COMPLETED" },
    { id: "in-progress", title: "Different provider filename", artist: "Uploader", status: "DOWNLOADING" },
    { id: "failed", title: "Other One", artist: "Artist", status: "FAILED" },
  ];
  const { forYou, trending } = splitDiscoveryRecommendations({
    recommendations: recs,
    playlistTrackIds: new Set(["playlist"]),
    likedTrackIds: new Set(["liked"]),
    downloads,
    downloadTargets: { "in-progress": { title: "Active", artist: "Artist" } },
    localTracks,
  });

  assert.deepEqual(forYou.map((rec) => rec.external_track_id), [
    "personal", "genre", "generic", "other1", "other2", "other3",
  ]);
  assert.equal(new Set(trending.map((rec) => rec.external_track_id)).size, trending.length);
  assert.equal(forYou.every((rec) => !trending.some((other) => other.title === rec.title && other.artist === rec.artist)), true);
});

test("a small feed keeps songs in Trending and does not invent personal picks", () => {
  const one = recommendation("one", "One");
  assert.deepEqual(splitDiscoveryRecommendations({
    recommendations: [one], playlistTrackIds: new Set(), likedTrackIds: new Set(),
    downloads: [], localTracks: [],
  }), { forYou: [], trending: [one] });
});
