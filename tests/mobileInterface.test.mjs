import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { isMobileUserAgent } from "../src/presentation/mobile/platform.ts";
import { formatDuration, joinMeta } from "../src/presentation/mobile/format.ts";
import { MOBILE_TABS, isMobileTab } from "../src/presentation/mobile/tabs.ts";

const read = (f) => readFileSync(new URL(`../${f}`, import.meta.url), "utf8");

test("Android and iOS user agents select the mobile interface; desktop does not", () => {
  assert.equal(isMobileUserAgent("Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36"), true);
  assert.equal(isMobileUserAgent("Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X)"), true);
  assert.equal(isMobileUserAgent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15"), false);
});

test("the tab bar has Home, Discover, Library and Queue in order", () => {
  assert.deepEqual(MOBILE_TABS.map((t) => t.label), ["Home", "Discover", "Library", "Queue"]);
  assert.equal(isMobileTab("queue"), true);
  assert.equal(isMobileTab("stats"), false);
});

test("formatDuration and joinMeta tolerate missing values", () => {
  assert.equal(formatDuration(65), "1:05");
  assert.equal(formatDuration(undefined), "0:00");
  assert.equal(formatDuration(-3), "0:00");
  assert.equal(joinMeta("Artist", undefined, "Album"), "Artist · Album");
  assert.equal(joinMeta(undefined, null), "");
});

test("mobile interface is never persisted and the hubs only wire it in", () => {
  const hook = read("src/presentation/useInterfacePreference.ts");
  assert.match(hook, /next === "mobile"/);
  assert.match(read("src/presentation/InterfaceContext.tsx"), /"spatial" \| "brutalist" \| "mobile"/);
  assert.match(read("src/App.tsx"), /style === "mobile"\) \{\s*return <MobileApp/);
});

test("mobile screens use no HTML audio and no unvirtualized track maps", () => {
  for (const f of ["MobileHome", "MobileDeck", "MobileLibrary", "MobileQueue", "MobileDiscover", "MobileApp"]) {
    const src = read(`src/presentation/mobile/${f}.tsx`);
    assert.doesNotMatch(src, /<audio|new Audio\(/, f);
  }
  for (const f of ["LibraryPanes", "CollectionPane", "queue/QueueBody", "discover/SearchResults", "playlists/ImportPlaylistSheet"]) {
    assert.match(read(`src/presentation/mobile/${f}.tsx`), /WindowedList/, f);
  }
});

test("mobile palette lives in tokens.css", () => {
  const tokens = read("src/styles/tokens.css");
  for (const color of ["#0a0a0b", "#1a1b1c", "#fb615a", "#e6e4df", "#c6c0b4"]) {
    assert.match(tokens, new RegExp(color), color);
  }
  assert.match(tokens, /Courier Prime/);
});
