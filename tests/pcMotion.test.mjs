import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const read = (f) => readFileSync(new URL(`../${f}`, import.meta.url), "utf8");
const css = read("src/presentation/pc-motion.css");

test("PC motion is wired in and scoped to the Brutalist interface", () => {
  assert.match(read("src/App.tsx"), /import "\.\/presentation\/pc-motion\.css";/);
  for (const rule of css.match(/^[^@/\s}][^{]*\{/gm)) {
    assert.match(rule, /\.kaze-interface\[data-interface="brutalist"\]/, rule);
  }
});

test("PC screens, sections and popups animate in", () => {
  assert.match(css, /\.main-content > :not\(\.interface-toolbar\) \{\s*animation: pc-fade/);
  assert.match(css, /\.main-content > :not\(\.interface-toolbar\) > :nth-child\(-n \+ 8\) \{\s*animation: pc-rise/);
  assert.match(css, /:is\(\.modal-content, \.modal-card-anim\) \{ animation: pc-pop/);
  assert.match(css, /\.press-fullscreen \{ animation: pc-sheet/);
  assert.match(read("src/components/modals/AuthModal.tsx"), /className="modal-card-anim"/);
});

test("PC motion only animates opacity, translate and scale", () => {
  for (const frames of css.match(/@keyframes [^{]+\{[^}]*\}/g)) {
    const props = [...frames.matchAll(/([a-z-]+):/g)].map((m) => m[1]);
    for (const p of props) assert.ok(["opacity", "translate", "scale"].includes(p), `${p} in ${frames}`);
  }
  assert.doesNotMatch(css, /backdrop-filter|filter:/);
});
