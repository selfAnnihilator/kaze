import test from "node:test";
import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";

// The Stats view and its section components (src/components/views/stats/).
const statsDir = new URL("../src/components/views/stats/", import.meta.url);
const view = [readFileSync(new URL("../src/components/views/StatsView.tsx", import.meta.url), "utf8")]
  .concat(readdirSync(statsDir).map((name) => readFileSync(new URL(name, statsDir), "utf8")))
  .join("\n");

test("profile stats show top 5 artists and top 5 genres beside top 10 songs", () => {
  assert.match(view, /title="Top 5 Artists"/);
  assert.match(view, /title="Top 5 Genres"/);
  assert.match(view, /Top 10 Songs/);
  assert.doesNotMatch(view, /Top 10 Artists\n/);
  assert.match(view, /rows\.slice\(0, 5\)/);
});

test("all-time lists are not labelled as per-year", () => {
  assert.doesNotMatch(view, /listened to yet this year/);
});
