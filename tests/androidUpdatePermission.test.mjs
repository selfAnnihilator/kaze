import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const read = (path) => readFileSync(new URL(`../${path}`, import.meta.url), "utf8");
const kotlin = "plugins/kaze-media/android/src/main/java/org/intelligentmusicplayer/kazemedia/";

test("restart permission is offered before downloading the update", () => {
  const rust = read("src-tauri/src/platform/android_update.rs");
  assert.match(rust, /prepare_update\(\)[\s\S]*let sha = download\(/);
  const bridge = read("plugins/kaze-media/src/mobile.rs");
  assert.match(bridge, /pub fn prepare_update[\s\S]*self\.call::<\(\)>\("prepareUpdate", \(\)\)/);
  const plugin = read(kotlin + "KazeMediaPlugin.kt");
  assert.match(plugin, /fun prepareUpdate[\s\S]*UpdateRestartPrompt\.show/);
  assert.match(plugin, /fun overlayAnswered[^}]*invoke\.resolve\(\)/);
});

test("the popup explains restart access and routes OK, decline and Back through the saved choice", () => {
  const prompt = read(kotlin + "UpdateRestartPrompt.kt");
  assert.match(prompt, /if \(!UpdateRelaunch\.shouldAsk\(context\)\) \{\s*continueUpdate\(\)\s*return/);
  assert.match(prompt, /setTitle\("Restart after updating\?"\)/);
  assert.match(prompt, /setMessage\([\s\S]*Display over other apps[\s\S]*open Kaze yourself/);
  assert.match(prompt, /setPositiveButton\("OK"\)[^}]*choice\.answer\(true\)/);
  assert.match(prompt, /setNegativeButton\("UPDATE WITHOUT RESTART"\)[^}]*choice\.answer\(false\)/);
  assert.match(prompt, /setOnCancelListener[^}]*choice\.answer\(false\)/);
  assert.match(prompt, /rememberAnswer = \{ UpdateRelaunch\.markAsked\(context\) \}/);
});

test("granted and remembered answers skip the popup across app updates", () => {
  const relaunch = read(kotlin + "UpdateRelaunch.kt");
  assert.match(relaunch, /UpdateRestartChoice\.shouldAsk\(canRelaunch\(context\), prefs\(context\)\.getBoolean\(ASKED, false\)\)/);
  assert.match(relaunch, /private const val ASKED = "overlay_asked"/);
  assert.match(relaunch, /putBoolean\(ASKED, true\)/);
  assert.doesNotMatch(relaunch, /remove\(ASKED\)/);
  assert.match(relaunch, /if \(!canRelaunch\(context\)\) \{[\s\S]*return\s*\}/);
});
