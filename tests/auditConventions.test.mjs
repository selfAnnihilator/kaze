import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { mkdtempSync, mkdirSync, writeFileSync, copyFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const script = join(process.cwd(), "scripts/audit-conventions.mjs");

/** A throwaway repo with one committed legacy file; returns a runner. */
function repo() {
  const dir = mkdtempSync(join(tmpdir(), "kaze-conventions-"));
  const git = (...a) => execFileSync("git", ["-c", "user.email=t@t", "-c", "user.name=t", ...a], { cwd: dir });
  git("init", "-q");
  mkdirSync(join(dir, "src"), { recursive: true });
  mkdirSync(join(dir, "scripts"));
  mkdirSync(join(dir, "src-tauri/migrations"), { recursive: true });
  copyFileSync(script, join(dir, "scripts/audit-conventions.mjs"));
  writeFileSync(join(dir, "src/legacy.ts"), "x\n".repeat(700));
  writeFileSync(join(dir, "scripts/conventions-baseline.json"), JSON.stringify({ maxLines: 500, legacy: { "src/legacy.ts": { lines: 701, smells: 0 } } }));
  git("add", "-A");
  git("commit", "-qm", "base");
  const run = () => spawnSync("node", ["scripts/audit-conventions.mjs"], { cwd: dir, encoding: "utf8", env: { ...process.env, KAZE_AUDIT_NO_QLTY: "1" } });
  return { dir, run, cleanup: () => rmSync(dir, { recursive: true, force: true }) };
}

test("small new files and shrinking legacy files pass", () => {
  const r = repo();
  try {
    writeFileSync(join(r.dir, "src/feature.ts"), "export const a = 1;\n");
    writeFileSync(join(r.dir, "src/legacy.ts"), "x\n".repeat(600));
    const out = r.run();
    assert.equal(out.status, 0, out.stdout);
    assert.match(out.stdout, /legacy, improved/);
  } finally { r.cleanup(); }
});

test("an oversized new file, a growing legacy file and a new migration fail", () => {
  const r = repo();
  try {
    writeFileSync(join(r.dir, "src/big.ts"), "x\n".repeat(600));
    writeFileSync(join(r.dir, "src/legacy.ts"), "x\n".repeat(800));
    writeFileSync(join(r.dir, "src-tauri/migrations/20990101000000_new.sql"), "SELECT 1;");
    const out = r.run();
    assert.equal(out.status, 1);
    assert.match(out.stdout, /src\/big\.ts.*601 lines \(max 500\)/);
    assert.match(out.stdout, /src\/legacy\.ts.*grew to 801 lines/);
    assert.match(out.stdout, /new migration/);
    assert.match(out.stdout, /3 convention failure/);
  } finally { r.cleanup(); }
});
