import test from "node:test";
import assert from "node:assert/strict";
import { registerHooks } from "node:module";
import { readFileSync, existsSync } from "node:fs";
import ts from "typescript";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";

// Smoke render of the real App shell. Server rendering runs no effects, so no
// IPC, native audio or Tauri window is touched; this guards the App.tsx
// decomposition against broken wiring (undefined members, hook order, imports).
registerHooks({
  resolve(specifier, context, next) {
    if (specifier.startsWith(".")) {
      const base = new URL(specifier, context.parentURL);
      for (const suffix of [".ts", ".tsx", "/index.ts", "/index.tsx"]) if (existsSync(new URL(base.href + suffix))) return next(base.href + suffix, context);
    }
    return next(specifier, context);
  },
  load(url, context, next) {
    if (url.endsWith(".css")) return { format: "module", source: "export default {};", shortCircuit: true };
    if (url.endsWith(".tsx") || url.endsWith(".ts")) return { format: "module", source: ts.transpileModule(readFileSync(new URL(url), "utf8"), { compilerOptions: { module: ts.ModuleKind.ESNext, jsx: ts.JsxEmit.ReactJSX, target: ts.ScriptTarget.ES2022 } }).outputText, shortCircuit: true };
    return next(url, context);
  },
});

// Define without reading: Node's built-in localStorage getter warns on access.
const store = new Map();
Object.defineProperty(globalThis, "localStorage", { configurable: true, value: { getItem: k => store.get(k) ?? null, setItem: (k, v) => store.set(k, String(v)), removeItem: k => store.delete(k), clear: () => store.clear() } });

const { App } = await import("../src/App.tsx");
const settings = {
  interface_style: "spatial",
  database_path: "/tmp/kaze.db",
  cache_dir: "/tmp/kaze-cache",
  audio: { default_volume: 0.8 },
  history: { min_meaningful_seconds: 30, min_meaningful_percentage: 0.5 },
  metadata: { provider_priority: [] },
  downloads: { auto_import: true, max_concurrent_downloads: 2 },
};

for (const style of ["spatial", "brutalist"]) {
  test(`App renders its ${style} shell without throwing`, () => {
    const markup = renderToStaticMarkup(React.createElement(App, { initialSettings: { ...settings, interface_style: style } }));
    assert.match(markup, /class="app-container kaze-interface"/);
    assert.match(markup, /class="main-content"/);
    assert.match(markup, /history-controls/, "toolbar with back/forward renders");
    assert.match(markup, /home-flow/, "default route renders Home");
    if (style === "spatial") assert.match(markup, /spatial-navigation/);
    else assert.match(markup, /press-header/);
  });
}
