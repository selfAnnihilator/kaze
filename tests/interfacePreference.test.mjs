import test from "node:test";
import assert from "node:assert/strict";
import { registerHooks } from "node:module";
import { readFileSync } from "node:fs";
import ts from "typescript";
const hooks =
  "data:text/javascript," +
  encodeURIComponent(
    `export const useState=(...args)=>globalThis.preferenceHarness.useState(...args);export const useRef=(...args)=>globalThis.preferenceHarness.useRef(...args);export const useEffect=(...args)=>globalThis.preferenceHarness.useEffect(...args);export const useLayoutEffect=useEffect;export const useCallback=fn=>fn;`,
  );
const api =
  "data:text/javascript," +
  encodeURIComponent(
    `export const dispatchCommand=c=>globalThis.preferenceHarness.dispatch(c);`,
  );
registerHooks({
  resolve(specifier, context, next) {
    if (context.parentURL?.endsWith("/useInterfacePreference.ts")) {
      if (specifier === "react") return { url: hooks, shortCircuit: true };
      if (specifier === "../services/api")
        return { url: api, shortCircuit: true };
    }
    return next(specifier, context);
  },
  load(url, context, next) {
    if (url.endsWith("/useInterfacePreference.ts"))
      return {
        format: "module",
        source: ts.transpileModule(readFileSync(new URL(url), "utf8"), {
          compilerOptions: {
            module: ts.ModuleKind.ESNext,
            target: ts.ScriptTarget.ES2022,
          },
        }).outputText,
        shortCircuit: true,
      };
    return next(url, context);
  },
});
const { useInterfacePreference } =
  await import("../src/presentation/useInterfacePreference.ts");
function setup(dispatch, style = "spatial") {
  const values = [],
    effects = [];
  let cursor = 0;
  globalThis.document = { documentElement: { dataset: {} } };
  globalThis.preferenceHarness = {
    dispatch,
    useState(initial) {
      const i = cursor++;
      if (!(i in values)) values[i] = initial;
      return [
        values[i],
        (v) => {
          values[i] = typeof v === "function" ? v(values[i]) : v;
        },
      ];
    },
    useRef(initial) {
      const i = cursor++;
      return (values[i] ??= { current: initial });
    },
    useEffect(effect, deps) {
      const i = cursor++;
      if (!values[i] || deps.some((v, j) => v !== values[i][j])) {
        values[i] = deps;
        effects.push(effect);
      }
    },
  };
  const settings = { interface_style: style };
  function render() {
    cursor = 0;
    const result = useInterfacePreference(settings);
    while (effects.length) effects.shift()();
    return result;
  }
  const initial = render();
  return { render, settings, initial };
}
test("the first render uses the hydrated preference before effects run", () => {
  for (const style of ["brutalist", "spatial"]) {
    const h = setup(() => { throw Error("No save is needed during hydration"); }, style);
    assert.equal(h.initial.style, style);
    assert.equal(document.documentElement.dataset.interface, style);
  }
});
test("selection applies immediately, serializes saves and uses only the existing IPC setting", async () => {
  let resolve;
  const calls = [];
  const h = setup((c) => {
    calls.push(c);
    return new Promise((r) => (resolve = r));
  });
  const pending = h.render().select("brutalist");
  assert.equal(h.render().style, "brutalist");
  assert.equal(h.render().saving, true);
  assert.equal(document.documentElement.dataset.interface, "brutalist");
  await h.render().select("spatial");
  assert.equal(calls.length, 1);
  assert.deepEqual(calls[0], {
    command: "SetInterfaceStyle",
    payload: { style: "brutalist" },
  });
  resolve();
  await pending;
  assert.equal(h.render().saving, false);
  assert.equal(h.render().style, "brutalist");
});
test("failed persistence restores the previous style and exposes a retryable error", async () => {
  const h = setup(async () => {
    throw Error("disk full");
  }, "brutalist");
  await h.render().select("spatial");
  const state = h.render();
  assert.equal(state.style, "brutalist");
  assert.match(state.error, /previous style has been restored/);
  assert.equal(state.saving, false);
});
