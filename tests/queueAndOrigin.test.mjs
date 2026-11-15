import test from "node:test";
import assert from "node:assert/strict";
import { build } from "esbuild";
import { mkdtemp, writeFile, symlink, rm } from "node:fs/promises";
import { readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";

const directory = await mkdtemp(join(tmpdir(), "kaze-queue-origin-"));
let m;
try {
  const result = await build({ stdin: { contents: `export { linkableOrigin } from './src/playingOrigin'; export { queueIdOf } from './src/trackKind'; export { QueueButton } from './src/components/QueueButton'; export { DiscoveryTrackCard } from './src/components/views/discovery/DiscoveryTrackCard';`, resolveDir: process.cwd(), loader: "ts" }, bundle: true, format: "esm", write: false, packages: "external", jsx: "automatic" });
  await symlink(join(process.cwd(), "node_modules"), join(directory, "node_modules"));
  await writeFile(join(directory, "module.mjs"), result.outputFiles[0].text);
  m = await import(pathToFileURL(join(directory, "module.mjs")));
} finally { await rm(directory, { recursive: true, force: true }); }

const read = (p) => readFileSync(new URL(`../${p}`, import.meta.url), "utf8");
const DOWNLOADS = "/music/Kaze";
const local = (id, file_path = `/music/Library/${id}.flac`) => ({ id, file_path, format: "flac" });
const online = (id) => ({ id, file_path: `online://${id}`, format: "online" });

test("the player links back only to the collection the playing song came from", () => {
  const liked = { type: "playlist", id: "p1", name: "Liked Songs", collectionData: { id: "p1", type: "playlist", title: "Liked Songs", subtitle: "", tag: "", tracks: [{ id: "jiosaavn:a", matched_local_track_id: "song-a" }, { id: "jiosaavn:b" }] } };
  assert.equal(m.linkableOrigin(liked, local("song-a"), DOWNLOADS), liked, "by local copy");
  assert.equal(m.linkableOrigin(liked, online("jiosaavn:b"), DOWNLOADS), liked, "by online id");
  assert.equal(m.linkableOrigin(liked, online("jiosaavn:queued-from-search"), DOWNLOADS), null, "a queued search song is not from the playlist");

  const sidebarPlaylist = { type: "playlist", id: "p2", name: "Road", trackIds: ["song-x"] };
  assert.equal(m.linkableOrigin(sidebarPlaylist, local("song-x"), DOWNLOADS), sidebarPlaylist);
  assert.equal(m.linkableOrigin(sidebarPlaylist, local("song-y"), DOWNLOADS), null);
});

test("Local and Downloads link by where the file lives; Flow and Discover never link", () => {
  const localList = { type: "library", scope: "local", name: "Local" };
  const downloadsList = { type: "library", scope: "downloads", name: "Downloads" };
  const mine = local("mine");
  const kaze = local("kaze", `${DOWNLOADS}/Artist - Song.m4a`);
  assert.equal(m.linkableOrigin(localList, mine, DOWNLOADS), localList);
  assert.equal(m.linkableOrigin(localList, kaze, DOWNLOADS), null);
  assert.equal(m.linkableOrigin(downloadsList, kaze, DOWNLOADS), downloadsList);
  assert.equal(m.linkableOrigin(downloadsList, mine, DOWNLOADS), null);
  assert.equal(m.linkableOrigin(localList, online("jiosaavn:c"), DOWNLOADS), null, "an online song is never from Local");
  assert.equal(m.linkableOrigin({ type: "flow", name: "Flow" }, mine, DOWNLOADS), null);
  assert.equal(m.linkableOrigin({ type: "discovery" }, mine, DOWNLOADS), null);
  assert.equal(m.linkableOrigin(null, mine, DOWNLOADS), null);
  assert.equal(m.linkableOrigin(localList, undefined, DOWNLOADS), null);
});

test("a recommendation is queued under its local copy when it has one", () => {
  assert.equal(m.queueIdOf({ external_track_id: "jiosaavn:a", matched_local_track_id: "song-a" }), "song-a");
  assert.equal(m.queueIdOf({ external_track_id: "jiosaavn:a", matched_local_track_id: "online:x" }), "jiosaavn:a");
  assert.equal(m.queueIdOf({ external_track_id: "jiosaavn:a" }), "jiosaavn:a");
});

test("Discover cards show one visible queue button, in the footer", () => {
  const rec = { external_track_id: "jiosaavn:a", provider: "jiosaavn", provider_id: "a", title: "Song", artist: "Artist", duration_secs: 120 };
  const card = (props) => renderToStaticMarkup(React.createElement(m.DiscoveryTrackCard, { rec, isPlaying: false, isLoading: false, isSelected: false, onPlay() {}, onStop() {}, onArtistClick() {}, onAddToWishlist() {}, onDownload() {}, onEnqueue() {}, ...props }));
  const html = card({ isQueued: false });
  assert.equal(html.match(/queue-button/g)?.length, 1);
  assert.match(html, /aria-label="Add Song to queue"/);
  assert.ok(html.indexOf("queue-button") > html.indexOf("discovery-card-title"), "below the cover, with the other actions");
  assert.match(card({ isQueued: true }), /aria-pressed="true"[^>]*title="Already in queue"|title="Already in queue"[^>]*aria-pressed="true"/);
});

test("search rows and the top result offer the queue; the players have no shuffle", () => {
  assert.match(read("src/components/views/OnlineSearchResultsSection.tsx"), /<QueueButton /);
  assert.match(read("src/components/views/searchResults/AllResultsTab.tsx"), /<QueueButton /);
  assert.match(read("src/components/views/DiscoveryView.tsx"), /onEnqueueTrack=\{onEnqueueTrack\}/);
  for (const player of ["src/presentation/BrutalistDeck.tsx", "src/components/NowPlayingBar.tsx"]) {
    assert.doesNotMatch(read(player), /Shuffle/, `${player} has no shuffle`);
  }
  assert.match(read("src/presentation/BrutalistDeck.tsx"), /title="Open Queue"[\s\S]{0,120}onClick=\{p\.onOpenQueue\}/);
  assert.match(read("src/components/views/collection/CollectionToolbar.tsx"), /if \(c\.isLikedSongs\) return null;/);
});

test("songs the user queues are marked manual, so they play before the app's queue", () => {
  const queue = read("src/hooks/app/useQueue.ts");
  assert.equal(queue.match(/manual: true/g)?.length, 2, "local and online enqueue");
  for (const app of ["src/hooks/app/useCollections.ts", "src/hooks/app/usePlaybackControls.ts", "src/hooks/app/usePlayDiscoveredCollection.ts"]) {
    assert.doesNotMatch(read(app), /manual: true/, `${app} queues for the app`);
  }
});
