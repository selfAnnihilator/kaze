import test from "node:test";
import assert from "node:assert/strict";
import { registerHooks } from "node:module";
import { readFileSync, existsSync } from "node:fs";
import ts from "typescript";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";

// Render the actual Home component; no browser, native audio or IPC is needed.
registerHooks({
  resolve(specifier, context, next) {
    if (specifier.startsWith(".")) {
      const base = new URL(specifier, context.parentURL);
      for (const suffix of [".ts", ".tsx"]) if (existsSync(new URL(base.href + suffix))) return next(base.href + suffix, context);
    }
    return next(specifier, context);
  },
  load(url, context, next) {
    if (url.endsWith(".tsx") || url.endsWith(".ts")) return { format: "module", source: ts.transpileModule(readFileSync(new URL(url), "utf8"), { compilerOptions: { module: ts.ModuleKind.ESNext, jsx: ts.JsxEmit.ReactJSX, target: ts.ScriptTarget.ES2022 } }).outputText, shortCircuit: true };
    return next(url, context);
  },
});
const { HomeView } = await import("../src/components/views/HomeView.tsx");
const currentTrack = { id: "a", title: "Track A", artist_name: "Artist", file_path: "/music/a.flac", format: "flac", duration_secs: 200 };
const overview = {
  flow_session: { is_active: true, last_track_id: "a", familiarity_ratio: .5, flow_label: "Balanced" },
  recent_rotation: [], rediscover: [],
  for_you: [{ track_id: "a", title: "Track A", reasons: [] }, { track_id: "b", title: "Track B", reasons: [] }],
};
const props = { overview, isLoading: false, currentTrack, activeTrackId: "a", flowSteering: .5, onFlowSteeringChange() {}, onRefresh() {}, onStartFlow() {}, onTogglePlayback() {}, onToggleFlowPlayback() {}, onStopFlow() {}, onPlayTrack() {} };
const render = changes => renderToStaticMarkup(React.createElement(HomeView, { ...props, ...changes }));

test("Home renders Pause Flow and Pause only on the active For You row", () => {
  const markup = render({ isPlaying: true });
  assert.match(markup, /Pause Flow/);
  assert.match(markup, /aria-label="Pause Track A"/);
  assert.match(markup, /aria-label="Play Track B"/);
  assert.match(markup, /Stop Flow/);
  assert.doesNotMatch(markup, /Flow Playing/);
});
test("paused playback renders Resume Flow and play icons without restarting the session", () => {
  const markup = render({ isPlaying: false });
  assert.match(markup, /Resume Flow/);
  assert.match(markup, /aria-label="Resume Track A"/);
  assert.match(markup, /aria-label="Play Track B"/);
});
test("inactive or unrelated session cannot label another track Pause Flow", () => {
  assert.doesNotMatch(render({ currentTrack: { ...currentTrack, id: "b" }, isPlaying: true }), /Pause Flow/);
  assert.match(render({ overview: { ...overview, flow_session: { ...overview.flow_session, is_active: false } }, isPlaying: true }), /Start Flow/);
});

test("changing current playback moves the Pause action to the newly active row", () => {
  const markup = render({ currentTrack: { ...currentTrack, id: "b", title: "Track B" }, isPlaying: true });
  assert.match(markup, /aria-label="Play Track A"/);
  assert.match(markup, /aria-label="Pause Track B"/);
});
