import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const read = (path) => readFileSync(new URL(`../${path}`, import.meta.url), "utf8");

test("Android Top Charts can request a language and refresh the same feed", () => {
  const hook = read("src/presentation/mobile/discover/useCharts.ts");
  assert.match(hook, /TASTE_LANGUAGES\.map/);
  assert.match(hook, /query: "GetLanguageCharts", payload: \{ language: feed, force_refresh: force \}/);
  assert.match(read("src/presentation/mobile/discover/DiscoverChapters.tsx"), /<TopCharts c=\{c\} revision=\{chartsKey\}/);
});

test("mobile search stores provider playlists with the current query and opens them by playlist ID", () => {
  const search = read("src/hooks/app/useGlobalSearch.ts");
  assert.match(search, /run\.mobile \? await executeQuery\(\{ query: "SearchMobileCatalog"/);
  assert.match(search, /current\.searchQuery === q \? \{ \.\.\.current, searchResults: results, catalogPlaylists \}/);
  const results = read("src/presentation/mobile/discover/SearchResults.tsx");
  assert.match(results, /WindowedList items=\{lists\}/);
  assert.match(results, /if \(!results\.length && !playlists\.length\)/);
  assert.match(read("src/presentation/mobile/discover/CatalogPlaylists.tsx"), /id: `jiosaavn:playlist:\$\{p\.id\}`, type: "chart"/);
});
