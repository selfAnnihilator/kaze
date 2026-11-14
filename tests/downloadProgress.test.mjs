import test from "node:test";
import assert from "node:assert/strict";
import { registerHooks } from "node:module";
import { readFileSync } from "node:fs";
import ts from "typescript";

registerHooks({
  load(url, context, next) {
    if (url.endsWith("/services/downloadProgress.ts"))
      return {
        format: "module",
        source: ts.transpileModule(readFileSync(new URL(url), "utf8"), {
          compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
        }).outputText,
        shortCircuit: true,
      };
    return next(url, context);
  },
});
const { downloadProgress } = await import("../src/services/downloadProgress.ts");

const task = (id, extra) => ({ id, status: "DOWNLOADING", bytes_downloaded: 0, ...extra });

test("rows find their download by track id or JioSaavn source, first task winning", () => {
  const first = task("1", { track_ids: ["a", "b"] });
  const second = task("2", { track_ids: ["b"], provider: "jiosaavn", source_track_id: "src9" });
  const third = task("3", { provider: "jiosaavn", source_track_id: "srcA" });
  downloadProgress.replace([first, second, third]);
  assert.equal(downloadProgress.task("a").id, "1");
  assert.equal(downloadProgress.task("b").id, "1", "the earlier task wins, as a scan would");
  assert.equal(downloadProgress.task("jiosaavn:src9").id, "2");
  assert.equal(downloadProgress.task("x", "srcA").id, "3");
  assert.equal(downloadProgress.task("nothing"), undefined);
  downloadProgress.replace([third]);
  assert.equal(downloadProgress.task("a"), undefined, "the index follows the latest list");
  downloadProgress.attach("a", "3");
  assert.equal(downloadProgress.task("a").id, "3");
});
