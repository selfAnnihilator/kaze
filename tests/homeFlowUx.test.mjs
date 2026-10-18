import test from "node:test";
import assert from "node:assert/strict";
import { visitDestination, replaceDestination, stepHistory, viewDestination, navigationShortcut } from "../src/navigationHistory.ts";
import { presentFlowExplanation } from "../src/flowExplanation.ts";
import { isCurrentPlaybackTrack } from "../src/localTrackMatch.ts";

const evidence = (values = {}) => ({ primary: "library", is_liked: false, is_unplayed: false, from_existing_queue: false, ...values });

test("history restores album and collection destinations, truncating abandoned forward branches", () => {
  const home = viewDestination("home");
  const album = { ...viewDestination("library"), libraryQuery: "Album A", libraryAlbumId: "album-a" };
  const collection = { ...viewDestination("playlists"), collection: { id: "p", type: "playlist", title: "Playlist", subtitle: "", tag: "", tracks: [{ id: "song" }] } };
  let h = { entries: [home], index: 0 };
  for (const d of [album, collection, viewDestination("downloads")]) h = visitDestination(h, d);
  h = stepHistory(h, -1); assert.equal(h.entries[h.index], collection);
  h = stepHistory(h, -1); assert.equal(h.entries[h.index], album);
  h = stepHistory(h, 1); assert.equal(h.entries[h.index].collection.tracks[0].id, "song");
  h = stepHistory(h, -1);
  h = visitDestination(h, viewDestination("settings"));
  assert.equal(h.entries.length, 3);
  assert.equal(stepHistory(h, 1), h);
  h = stepHistory(stepHistory(h, -1), -1);
  assert.equal(stepHistory(h, -1), h);
});

test("hydration and repeated visits don't append entries; collection and search identities remain distinct", () => {
  let h = { entries: [viewDestination("home")], index: 0 };
  const detail = { ...viewDestination("discovery"), collection: { id: "mix1", type: "mix", title: "Mix", subtitle: "", tag: "" } };
  h = visitDestination(h, detail);
  h = replaceDestination(h, { ...detail, collection: { ...detail.collection, tracks: [] } });
  assert.equal(h.entries.length, 2);
  h = visitDestination(h, h.entries[h.index]); assert.equal(h.entries.length, 2);
  h = visitDestination(h, { ...viewDestination("discovery"), searchQuery: "Queen", searchResults: [{ title: "Track" }] });
  h = visitDestination(h, { ...viewDestination("discovery"), searchQuery: "Prince" });
  h = stepHistory(h, -1); assert.equal(h.entries[h.index].searchResults[0].title, "Track");
  h = stepHistory(h, -1); assert.equal(h.entries[h.index].collection.id, "mix1");
});

test("Alt navigation leaves editable, handled, conflicting and normal arrow events alone", () => {
  const e = { key: "ArrowLeft", altKey: true, ctrlKey: false, metaKey: false, shiftKey: false, defaultPrevented: false };
  assert.equal(navigationShortcut(e, false), -1);
  assert.equal(navigationShortcut({ ...e, key: "ArrowRight" }, false), 1);
  for (const field of ["ctrlKey", "metaKey", "shiftKey", "defaultPrevented"]) assert.equal(navigationShortcut({ ...e, [field]: true }, false), null);
  assert.equal(navigationShortcut(e, true), null);
  assert.equal(navigationShortcut({ ...e, altKey: false }, false), null);
});

test("playback identity covers local IDs, explicit matches and catalog aliases without confusing local duplicates or covers", () => {
  const current = { id: "local", title: "Song", artist_name: "Artist", duration_secs: 200, format: "flac", file_path: "/music/song.flac" };
  assert.equal(isCurrentPlaybackTrack({ track_id: "local", title: "Song" }, current), true);
  assert.equal(isCurrentPlaybackTrack({ external_track_id: "itunes:1", matched_local_track_id: "local", title: "Song" }, current), true);
  assert.equal(isCurrentPlaybackTrack({ external_track_id: "itunes:1", title: "Song", artist: "Artist", duration_secs: 201 }, current), true);
  assert.equal(isCurrentPlaybackTrack({ track_id: "other-local", title: "Song", artist_name: "Artist" }, current), false);
  assert.equal(isCurrentPlaybackTrack({ external_track_id: "itunes:2", title: "Song", artist: "Cover artist" }, current), false);
  assert.equal(isCurrentPlaybackTrack({ external_track_id: "itunes:2", title: "Song", artist: "Artist", duration_secs: 300 }, current), false);
  assert.equal(isCurrentPlaybackTrack({ track_id: "local", title: "Song" }, null), false);
});

test("explanations use verified evidence for all five cases and avoid unsupported frequency claims", () => {
  assert.equal(presentFlowExplanation(evidence({ primary: "library_exploration", is_unplayed: true })).title, "A fresh pick from your library");
  const liked = presentFlowExplanation(evidence({ primary: "liked", is_liked: true, days_since_played: 123, artist_affinity: .7, artist_name: "Artist", familiarity_ratio: .2, selected_at: 100 }));
  assert.equal(liked.title, "One of your favorites"); assert.equal(liked.secondary.length, 2); assert.match(liked.secondary[0], /about 4 months/);
  assert.match(presentFlowExplanation(evidence({ primary: "rediscovery", days_since_played: 45 })).body, /45 days/);
  const affinity = presentFlowExplanation(evidence({ primary: "artist_affinity", artist_affinity: .5, artist_name: "Charlie Puth" }));
  assert.match(affinity.title, /Charlie Puth/); assert.doesNotMatch(affinity.body, /often|frequent/);
  assert.equal(presentFlowExplanation().title, "From your library");
  assert.equal(presentFlowExplanation(evidence({ primary: "library_exploration", is_unplayed: false })).title, "From your library");
  assert.equal(presentFlowExplanation(evidence({ primary: "rediscovery" })).title, "From your library");
  assert.match(presentFlowExplanation(evidence({ primary: "library_exploration", is_unplayed: true, selected_at: 100 })).body, /hadn't/);
  assert.equal(presentFlowExplanation(evidence({ from_existing_queue: true })).title, "Your queue, your choice");
});

test("secondary evidence preserves recorded counts and steering without inventing low-play rankings", () => {
  const copy = presentFlowExplanation(evidence({ play_count: 2, familiarity_ratio: .1, selected_at: 100 }));
  assert.match(copy.secondary[0], /2 recorded plays/);
  assert.match(copy.secondary[1], /rediscovery/);
  assert.doesNotMatch(JSON.stringify(copy), /less than most|low.play|batch|database|generation|scorer/i);
});

test("preserved pre-Flow current song has continuation copy without invented queue-choice or taste claims", () => {
  const copy = presentFlowExplanation(evidence({ primary: "continuing_track" }));
  assert.equal(copy.title, "Your current track continues");
  assert.match(copy.body, /next track/);
  assert.equal(copy.secondary.length, 0);
  assert.doesNotMatch(JSON.stringify(copy), /your choice|favorite|affinity/i);
});

test("online Flow discoveries keep their source visible without claiming local-library ownership", () => {
  const fresh = presentFlowExplanation(evidence({ primary: "online_discovery", is_unplayed: true, familiarity_ratio: 0, selected_at: 1 }));
  assert.equal(fresh.title, "An online discovery");
  assert.match(fresh.body, /online song/);
  assert.doesNotMatch(JSON.stringify(fresh), /your library|own music collection/);
  const known = presentFlowExplanation(evidence({ primary: "online", play_count: 3 }));
  assert.equal(known.title, "From your online listening");
  assert.doesNotMatch(known.body, /library/);
});
