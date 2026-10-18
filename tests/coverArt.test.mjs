import test from "node:test";
import assert from "node:assert/strict";
import { registerHooks } from "node:module";
import { readFileSync } from "node:fs";
import ts from "typescript";
const api =
  "data:text/javascript," +
  encodeURIComponent(
    "export const executeQuery=q=>globalThis.coverArtHarness.query(q);",
  );
registerHooks({
  resolve(specifier, context, next) {
    if (
      context.parentURL?.endsWith("/services/coverArt.ts") &&
      specifier === "./api"
    )
      return { url: api, shortCircuit: true };
    return next(specifier, context);
  },
  load(url, context, next) {
    if (url.endsWith("/services/coverArt.ts"))
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
const { loadCoverArt, peekCoverArt, clearCoverArtCache } =
  await import("../src/services/coverArt.ts");
const deferred = () => {
  let resolve;
  const promise = new Promise((r) => (resolve = r));
  return { promise, resolve };
};
const flush = () => new Promise((resolve) => setImmediate(resolve));

test("concurrent artwork consumers share a request; leaving a view cancels its queued work", async () => {
  clearCoverArtCache();
  const calls = [];
  const jobs = [];
  globalThis.coverArtHarness = {
    query(q) {
      calls.push(q.payload.track_id);
      const job = deferred();
      jobs.push(job);
      return job.promise;
    },
  };
  const first = loadCoverArt("same");
  const second = loadCoverArt("same");
  const next = ["b", "c", "d"].map((id) => loadCoverArt(id));
  const abandoned = new AbortController();
  const skipped = loadCoverArt("unmounted", abandoned.signal);
  abandoned.abort();
  assert.equal(await skipped, null);
  assert.deepEqual(calls, ["same", "b", "c", "d"]);
  jobs.forEach((job, i) => job.resolve({ data: `cover-${i}` }));
  assert.equal(await first, "cover-0");
  assert.equal(await second, "cover-0");
  await Promise.all(next);
  await flush();
  assert.equal(await loadCoverArt("same"), "cover-0");
  assert.equal(calls.length, 4);
});

test("one consumer leaving does not cancel another; a failed read can retry", async () => {
  clearCoverArtCache();
  const job = deferred();
  let calls = 0;
  globalThis.coverArtHarness = {
    query() {
      calls++;
      return calls === 1 ? job.promise : Promise.resolve({ data: "retried" });
    },
  };
  const controller = new AbortController();
  const gone = loadCoverArt("shared", controller.signal);
  const stays = loadCoverArt("shared");
  controller.abort();
  job.resolve({ data: "available" });
  assert.equal(await gone, null);
  assert.equal(await stays, "available");
  await flush();
  clearCoverArtCache();
  globalThis.coverArtHarness.query = () => Promise.reject(Error("unavailable"));
  assert.equal(await loadCoverArt("retry"), null);
  await flush();
  globalThis.coverArtHarness.query = () => Promise.resolve({ data: "retried" });
  assert.equal(await loadCoverArt("retry"), "retried");
  await flush();
});

test("artwork cache is bounded by entry count and retained string bytes", async () => {
  clearCoverArtCache();
  globalThis.coverArtHarness = {
    query: (q) => Promise.resolve({ data: q.payload.track_id }),
  };
  for (let i = 0; i < 70; i++) {
    await loadCoverArt(`track-${i}`);
    await flush();
  }
  assert.equal(peekCoverArt("track-0"), undefined);
  assert.equal(peekCoverArt("track-69"), "track-69");
  clearCoverArtCache();
  globalThis.coverArtHarness.query = () =>
    Promise.resolve({ data: "x".repeat(1024 * 1024) });
  for (let i = 0; i < 6; i++) {
    await loadCoverArt(`large-${i}`);
    await flush();
  }
  assert.equal(peekCoverArt("large-0"), undefined);
  assert.equal(peekCoverArt("large-5").length, 1024 * 1024);
});

test("scan invalidation prevents a late old read from repopulating the cache", async () => {
  clearCoverArtCache();
  const old = deferred();
  globalThis.coverArtHarness = { query: () => old.promise };
  const reading = loadCoverArt("changed");
  clearCoverArtCache();
  old.resolve({ data: "stale" });
  await reading;
  await flush();
  assert.equal(peekCoverArt("changed"), undefined);
});
