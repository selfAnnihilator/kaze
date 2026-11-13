import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const view = readFileSync(new URL("../src/components/views/StatsView.tsx", import.meta.url), "utf8");

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
