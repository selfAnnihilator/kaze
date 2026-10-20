import test from "node:test";
import assert from "node:assert/strict";
import { registerHooks } from "node:module";
import { readFileSync } from "node:fs";
import ts from "typescript";

const realReact = import.meta.resolve("react");
const hooksUrl = "data:text/javascript," + encodeURIComponent(`
import React from ${JSON.stringify(realReact)};
export const memo = fn => fn;
export const useState = (...args) => globalThis.startupHarness.useState(...args);
export const useRef = (...args) => globalThis.startupHarness.useRef(...args);
export const useEffect = (...args) => globalThis.startupHarness.useEffect(...args);
`);
const apiUrl = "data:text/javascript," + encodeURIComponent(`
export const getAutostart = () => globalThis.startupHarness.get();
export const setAutostart = value => globalThis.startupHarness.set(value);
`);
registerHooks({
  resolve(specifier, context, next) {
    if (context.parentURL?.includes("/StartupSettings.tsx")) {
      if (specifier === "react") return { url: hooksUrl, shortCircuit: true };
      if (specifier === "../services/desktopStartup") return { url: apiUrl, shortCircuit: true };
    }
    return next(specifier, context);
  },
  load(url, context, next) {
    if (url.includes("/StartupSettings.tsx")) return {
      format: "module",
      source: ts.transpileModule(readFileSync(new URL(url), "utf8"), {
        compilerOptions: { module: ts.ModuleKind.ESNext, jsx: ts.JsxEmit.ReactJSX, target: ts.ScriptTarget.ES2022 },
      }).outputText,
      shortCircuit: true,
    };
    return next(url, context);
  },
});
const { StartupSettings } = await import("../src/components/StartupSettings.tsx?startup-test");
function find(node, predicate) {
  if (Array.isArray(node)) {
    for (const child of node) { const found = find(child, predicate); if (found) return found; }
  } else if (node?.props) {
    if (predicate(node)) return node;
    return find(node.props.children, predicate);
  }
}
function setup(get = async () => false, set = async value => value) {
  const values = [], effects = [], cleanups = [];
  let cursor = 0;
  globalThis.startupHarness = {
    get, set,
    useState(initial) {
      const i = cursor++;
      if (!(i in values)) values[i] = initial;
      return [values[i], value => { values[i] = typeof value === "function" ? value(values[i]) : value; }];
    },
    useRef(initial) { const i = cursor++; return (values[i] ??= { current: initial }); },
    useEffect(effect, deps) {
      const i = cursor++;
      if (!values[i] || deps.some((v, j) => !Object.is(v, values[i][j]))) {
        values[i] = deps;
        effects.push(() => { cleanups[i]?.(); cleanups[i] = effect(); });
      }
    },
  };
  const render = () => { cursor = 0; const tree = StartupSettings(); while (effects.length) effects.shift()(); return tree; };
  const control = tree => find(tree, node => node.props.role === "switch");
  return { render, control, choose: value => control(render()).props.onChange({ target: { checked: value } }) };
}
const settle = () => new Promise(resolve => setImmediate(resolve));

test("login switch waits for the OS value and loads enabled registration", async () => {
  let resolve;
  const h = setup(() => new Promise(r => { resolve = r; }));
  assert.equal(h.control(h.render()).props.disabled, true);
  resolve(true); await settle();
  assert.equal(h.control(h.render()).props.checked, true);
  assert.equal(h.control(h.render()).props.disabled, false);
});

test("login switch confirms save before changing and prevents overlapping writes", async () => {
  let resolve, calls = 0;
  const h = setup(undefined, value => { calls++; assert.equal(value, true); return new Promise(r => { resolve = r; }); });
  h.render(); await settle();
  h.choose(true); h.choose(true);
  assert.equal(calls, 1);
  assert.equal(h.control(h.render()).props.checked, false);
  assert.equal(h.control(h.render()).props.disabled, true);
  resolve(true); await settle();
  assert.equal(h.control(h.render()).props.checked, true);
  assert.equal(h.control(h.render()).props.disabled, false);
});

test("failed login write keeps the confirmed choice and offers retry by switching again", async () => {
  let calls = 0;
  const h = setup(undefined, async value => { if (++calls === 1) throw Error("permission denied"); return value; });
  h.render(); await settle(); h.choose(true); await settle();
  assert.equal(h.control(h.render()).props.checked, false);
  assert.ok(find(h.render(), node => node.props.role === "alert"));
  h.choose(true); await settle();
  assert.equal(h.control(h.render()).props.checked, true);
  h.choose(false); await settle();
  assert.equal(h.control(h.render()).props.checked, false);
});

test("a failed startup read cannot be overwritten and can be retried", async () => {
  let reads = 0;
  const h = setup(async () => { if (++reads === 1) throw Error("unavailable"); return true; });
  h.render(); await settle();
  assert.equal(h.control(h.render()).props.disabled, true);
  find(h.render(), node => node.type === "button").props.onClick();
  h.render(); await settle();
  assert.equal(h.control(h.render()).props.checked, true);
  assert.equal(h.control(h.render()).props.disabled, false);
});

test("a mismatched OS confirmation is displayed truthfully", async () => {
  const h = setup(undefined, async () => false);
  h.render(); await settle(); h.choose(true); await settle();
  assert.equal(h.control(h.render()).props.checked, false);
  assert.ok(find(h.render(), node => node.props.role === "alert"));
});
