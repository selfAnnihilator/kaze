import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import vm from "node:vm";
import ts from "typescript";

const compile = (path) => ts.transpileModule(readFileSync(new URL(`../src/${path}`, import.meta.url), "utf8"), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022, jsx: ts.JsxEmit.React, esModuleInterop: true },
}).outputText;
const settle = () => new Promise(setImmediate);

/** Execute the real entrypoint with an initially unavailable bridge and a disposable renderer. */
function start({ phone = true, failures = 1 } = {}) {
  const paints = [], timers = new Map(), errors = [];
  const saved = { interface_style: "brutalist" };
  let calls = 0, timerId = 0, remainingFailures = failures;
  const createElement = (type, props, ...children) => ({ type, props: { ...props, children } });
  const modules = {
    react: { createElement, StrictMode: "strict" },
    "react-dom/client": { createRoot: () => ({ render(view) {
      const content = view.props.children[0];
      paints.push(content);
      if (content.props.onIntroComplete) queueMicrotask(content.props.onIntroComplete);
    } }) },
    "./services/api": { executeQuery: async () => {
      calls++;
      if (remainingFailures-- > 0) throw Error("test: startup settings not ready");
      return { type: "Settings", data: saved };
    } },
    "./presentation/mobile/platform": { isMobileDevice: () => phone },
    "./components/StartupScreen": { StartupScreen: "desktop-opening" },
    "./presentation/mobile/MobileStartup": { MobileStartup: "phone-opening" },
    "./App": { App: "App" },
  };
  const globals = {
    document: { addEventListener() {}, getElementById: () => ({}), documentElement: { dataset: {} } },
    console: { error: (...values) => errors.push(values) },
    setTimeout: (callback) => { const id = ++timerId; timers.set(id, callback); return id; },
    clearTimeout: (id) => timers.delete(id),
  };
  const require = (name) => {
    if (name.endsWith(".css")) return {};
    if (name === "./services/startupSettings") {
      const exports = {};
      vm.runInNewContext(compile("services/startupSettings.ts"), { ...globals, exports, require });
      return exports;
    }
    assert.ok(name in modules, `Unexpected bootstrap import: ${name}`);
    return modules[name];
  };
  vm.runInNewContext(compile("main.tsx"), { ...globals, exports: {}, require });
  return {
    paints, saved, errors, timers,
    calls: () => calls,
    ready: () => { remainingFailures = 0; },
    current: () => paints.at(-1),
    async advance() {
      const entry = timers.entries().next().value;
      assert.ok(entry, "A bounded startup retry must be scheduled");
      timers.delete(entry[0]); entry[1](); await settle();
    },
  };
}

test("Android startup recovers a transient settings failure without asking the user to retry", async () => {
  const h = start();
  await settle();
  assert.equal(h.current().type, "phone-opening");
  await h.advance();
  assert.equal(h.current().type, "App");
  assert.equal(h.current().props.initialSettings, h.saved);
  assert.equal(h.calls(), 2);
  assert.equal(h.paints.some((paint) => paint.props.role === "alert"), false);
  assert.equal(h.timers.size, 0);
});

test("persistent Android startup failure exhausts bounded retries and keeps a working manual retry", async () => {
  const h = start({ failures: Infinity });
  await settle();
  for (let retries = 0; h.timers.size && retries < 10; retries++) await h.advance();
  assert.equal(h.current().props.role, "alert");
  assert.ok(h.calls() > 1 && h.calls() <= 5);
  assert.equal(h.timers.size, 0);
  h.ready();
  h.current().props.children.find((node) => node.type === "button").props.onClick();
  await settle();
  assert.equal(h.current().type, "App");
});

test("desktop startup retains its immediate failure and manual retry behavior", async () => {
  const h = start({ phone: false });
  await settle();
  assert.equal(h.current().props.role, "alert");
  assert.equal(h.calls(), 1);
  assert.equal(h.timers.size, 0);
  h.current().props.children.find((node) => node.type === "button").props.onClick();
  await settle();
  assert.equal(h.current().type, "App");
  assert.equal(h.current().props.initialSettings, h.saved);
});
