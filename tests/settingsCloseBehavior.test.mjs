import test from "node:test";
import assert from "node:assert/strict";
import { registerHooks } from "node:module";
import { readFileSync, existsSync } from "node:fs";
import ts from "typescript";

// Execute the actual Settings component and its handlers with controlled hook
// scheduling and IPC responses. No copied production handler or native writes.
const realReact = import.meta.resolve("react");
const hooksUrl =
  "data:text/javascript," +
  encodeURIComponent(`
import React from ${JSON.stringify(realReact)};
export default React;
export const memo = fn => fn;
export const useLayoutEffect = (...a) => globalThis.closeSettingsHarness.useEffect(...a);
export const useState = (...a) => globalThis.closeSettingsHarness.useState(...a);
export const useRef = (...a) => globalThis.closeSettingsHarness.useRef(...a);
export const useEffect = (...a) => globalThis.closeSettingsHarness.useEffect(...a);
`);
const memoryUrl =
  "data:text/javascript," +
  encodeURIComponent(
    `export const useViewMemory=(name,create)=>globalThis.closeSettingsHarness.useRef(create()).current;`,
  );
const apiUrl =
  "data:text/javascript," +
  encodeURIComponent(`
export const dispatchCommand = (...a) => globalThis.closeSettingsHarness.dispatch(...a);
export const executeQuery = async () => ({ total_size_bytes: 0 });
`);
registerHooks({
  resolve(specifier, context, next) {
    if (context.parentURL?.endsWith("/SettingsView.tsx")) {
      if (specifier === "../../presentation/ViewMemory")
        return { url: memoryUrl, shortCircuit: true };
      if (specifier === "react") return { url: hooksUrl, shortCircuit: true };
      if (specifier === "../../services/api")
        return { url: apiUrl, shortCircuit: true };
    }
    if (specifier.startsWith(".")) {
      const base = new URL(specifier, context.parentURL);
      for (const suffix of [".ts", ".tsx"])
        if (existsSync(new URL(base.href + suffix)))
          return next(base.href + suffix, context);
    }
    return next(specifier, context);
  },
  load(url, context, next) {
    if (url.endsWith(".tsx") || url.endsWith(".ts"))
      return {
        format: "module",
        source: ts.transpileModule(readFileSync(new URL(url), "utf8"), {
          compilerOptions: {
            module: ts.ModuleKind.ESNext,
            jsx: ts.JsxEmit.ReactJSX,
            target: ts.ScriptTarget.ES2022,
          },
        }).outputText,
        shortCircuit: true,
      };
    return next(url, context);
  },
});
const { SettingsView } =
  await import("../src/components/views/SettingsView.tsx");
const text = (node) =>
  Array.isArray(node)
    ? node.map(text).join("")
    : typeof node === "string"
      ? node
      : node?.props
        ? text(node.props.children)
        : "";
function find(node, predicate) {
  if (Array.isArray(node)) {
    for (const child of node) {
      const found = find(child, predicate);
      if (found) return found;
    }
  } else if (node?.props) {
    if (predicate(node)) return node;
    return find(node.props.children, predicate);
  }
}
function setup(dispatch, reload = async () => {}) {
  const values = [],
    effects = [];
  let cursor = 0;
  globalThis.closeSettingsHarness = {
    dispatch,
    useState(initial) {
      const i = cursor++;
      if (!(i in values))
        values[i] = typeof initial === "function" ? initial() : initial;
      return [
        values[i],
        (value) => {
          values[i] = typeof value === "function" ? value(values[i]) : value;
        },
      ];
    },
    useRef(initial) {
      const i = cursor++;
      return (values[i] ??= { current: initial });
    },
    useEffect(effect, deps) {
      const i = cursor++;
      if (!values[i] || deps.some((v, j) => !Object.is(v, values[i][j]))) {
        values[i] = deps;
        effects.push(effect);
      }
    },
  };
  const props = {
    interfaceStyle: "spatial",
    onSelectInterfaceStyle: async () => {},
    settings: { close_button_behavior: "minimize_to_tray" },
    configuredFolders: [],
    currentUser: null,
    onAddFolder: async () => {},
    onRemoveFolder: async () => {},
    onRescanLibrary: async () => {},
    onLogout: async () => {},
    onOpenAuthModal() {},
    onReloadSettings: reload,
  };
  function render() {
    cursor = 0;
    const tree = SettingsView(props);
    while (effects.length) effects.shift()();
    return tree;
  }
  const select = (tree) =>
    find(
      tree,
      (n) => n.type === "select" && n.props.id === "settings-close-behavior",
    );
  const choose = (behavior) =>
    select(render()).props.onChange({ target: { value: behavior } });
  return { render, select, choose };
}
const settle = () => new Promise((resolve) => setImmediate(resolve));

test("failed persistence leaves the previous close choice active and shows an error", async (t) => {
  t.mock.method(console, "error", () => {});
  const h = setup(async () => {
    throw Error("disk full");
  });
  h.choose("quit_app");
  await settle();
  const tree = h.render();
  assert.equal(h.select(tree).props.value, "minimize_to_tray");
  assert.match(
    text(find(tree, (n) => n.props.role === "alert")),
    /Couldn't save/,
  );
});
test("a pending save does not claim success and blocks duplicate clicks", async () => {
  let resolve,
    count = 0,
    reloads = 0;
  const h = setup(
    (command) => {
      count++;
      assert.deepEqual(command, {
        command: "SetCloseButtonBehavior",
        payload: { behavior: "quit_app" },
      });
      return new Promise((r) => {
        resolve = r;
      });
    },
    async () => {
      reloads++;
    },
  );
  h.choose("quit_app");
  h.choose("quit_app");
  assert.equal(count, 1);
  assert.equal(h.select(h.render()).props.disabled, true);
  assert.equal(h.select(h.render()).props.value, "minimize_to_tray");
  resolve({});
  await settle();
  assert.equal(reloads, 1);
  assert.equal(h.select(h.render()).props.value, "quit_app");
  assert.equal(h.select(h.render()).props.disabled, false);
});
test("a refresh failure reports a saved choice accurately", async () => {
  const h = setup(
    async () => ({}),
    async () => {
      throw Error("refresh failed");
    },
  );
  h.choose("quit_app");
  await settle();
  const tree = h.render();
  assert.equal(h.select(tree).props.value, "quit_app");
  assert.match(
    text(find(tree, (n) => n.props.role === "alert")),
    /choice was saved/,
  );
});
