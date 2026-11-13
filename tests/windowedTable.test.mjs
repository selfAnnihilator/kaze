import test from "node:test";
import assert from "node:assert/strict";
import { registerHooks } from "node:module";
import { readFileSync } from "node:fs";
import ts from "typescript";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";

registerHooks({
  load(url, context, next) {
    if (url.endsWith(".tsx")) return { format: "module", source: ts.transpileModule(readFileSync(new URL(url), "utf8"), { compilerOptions: { module: ts.ModuleKind.ESNext, jsx: ts.JsxEmit.ReactJSX, target: ts.ScriptTarget.ES2022 } }).outputText, shortCircuit: true };
    return next(url, context);
  },
});
const { WindowedTable } = await import("../src/presentation/WindowedTable.tsx");
const h = React.createElement;
const items = Array.from({ length: 60 }, (_, i) => i);
const spans = markup => [...markup.matchAll(/colSpan="(\d+)"|colspan="(\d+)"/g)].map(m => Number(m[1] ?? m[2]));

// Under `table-layout: fixed` an oversized spacer colSpan adds phantom columns
// that squeeze the real ones (import modal regression).
test("spacer rows span exactly the rendered column count", () => {
  const markup = renderToStaticMarkup(h(WindowedTable, {
    items,
    renderItem: i => h("tr", { key: i }, h("td", null, i), h("td", null, "t"), h("td", null, "a"), h("td", null, "s"), h("td", null, "d")),
  }));
  assert.deepEqual(spans(markup), [5]);
});

test("wrapped rows fall back to the header column count", () => {
  const Wrap = ({ children }) => children;
  const markup = renderToStaticMarkup(h(WindowedTable, {
    items,
    renderItem: i => h(Wrap, { key: i }, h("tr", null, h("td", null, i), h("td", null, "x"), h("td", null, "y"))),
  }, h("thead", null, h("tr", null, h("th", null, "A"), h("th", null, "B"), h("th", null, "C")))));
  assert.deepEqual(spans(markup), [3]);
});
