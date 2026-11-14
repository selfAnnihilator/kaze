import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";

const TOKENS = "src/styles/tokens.css";
const files = execFileSync("git", ["ls-files", "src"], { encoding: "utf8" })
  .split("\n").filter((f) => /\.(css|tsx?)$/.test(f) && !f.includes("design-exploration/"));
const read = (f) => readFileSync(f, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");

/** Custom properties defined anywhere: CSS `--x:`, inline `"--x":`, or setProperty("--x"). */
function definedNames() {
  const names = new Set();
  for (const f of files) {
    for (const m of read(f).matchAll(/(?:^|[\s{;"'`(])(--[\w-]+)["'`]?\s*[:,]/g)) names.add(m[1]);
  }
  return names;
}

test("every var(--name) without a fallback refers to a defined token", () => {
  const defined = definedNames();
  const missing = new Set();
  for (const f of files) {
    for (const m of read(f).matchAll(/var\(\s*(--[\w-]+)\s*\)/g)) {
      if (!defined.has(m[1])) missing.add(`${m[1]} (${f})`);
    }
  }
  assert.deepEqual([...missing], []);
});

test("tokens.css holds the global knobs and keeps the theme cascade order", () => {
  const css = read(TOKENS);
  for (const knob of ["--font-scale", "--space-scale", "--radius-scale", "--motion-scale"]) assert.match(css, new RegExp(`${knob}:`));
  const order = [":root {", ":root[data-interface] {", ':root[data-interface="brutalist"] {', ':root[data-interface="spatial"] {', '.kaze-interface[data-interface="brutalist"] {'];
  const at = order.map((sel) => css.indexOf(sel));
  assert.ok(at.every((i) => i >= 0), `missing theme block: ${order[at.indexOf(-1)]}`);
  assert.deepEqual([...at].sort((a, b) => a - b), at);
});

test("theme tokens are defined only in tokens.css", () => {
  const theme = /^\s*--(bg-main|bg-card|bg-player|bg-sidebar|text-main|text-muted|text-dim|accent|accent-light|accent-hover|border|border-light|font-sans|font-heading|press-[a-z]+):/m;
  const elsewhere = files.filter((f) => f.endsWith(".css") && f !== TOKENS && theme.test(read(f)));
  // spatial.css scopes --accent to one panel (a local override, not a definition).
  assert.deepEqual(elsewhere.filter((f) => !/spatial\.css$/.test(f)), []);
});
