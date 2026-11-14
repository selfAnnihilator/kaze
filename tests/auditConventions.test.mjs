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
  copyFileSync(join(process.cwd(), "scripts/style-literals.mjs"), join(dir, "scripts/style-literals.mjs"));
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

test("new hardcoded style values fail; tokens, hairlines and local stacking pass", () => {
  const r = repo();
  try {
    writeFileSync(join(r.dir, "src/clean.css"), ".a { color: var(--text-main); padding: var(--space-2) 1px; margin: 0; z-index: 2; --w: 155px; }\n");
    writeFileSync(join(r.dir, "src/Clean.tsx"), "export const s = { color: \"var(--accent)\", padding: 0, zIndex: 3 };\n");
    let out = r.run();
    assert.equal(out.status, 0, out.stdout);
    writeFileSync(join(r.dir, "src/dirty.css"), ".a { color: #fff; padding: 8px 12px; font-size: 13px; z-index: 100; transition: opacity .2s; }\n");
    writeFileSync(join(r.dir, "src/Dirty.tsx"), "export const s = { color: \"rgba(0,0,0,.4)\", fontSize: 13, borderRadius: \"8px\" };\n");
    out = r.run();
    assert.equal(out.status, 1);
    assert.match(out.stdout, /src\/dirty\.css.*6 hardcoded style values/);
    assert.match(out.stdout, /src\/Dirty\.tsx.*3 hardcoded style values/);
  } finally { r.cleanup(); }
});

test("a legacy file's style literals may shrink but not grow", () => {
  const r = repo();
  try {
    writeFileSync(join(r.dir, "src/old.css"), ".a { color: #fff; padding: 8px; }\n");
    writeFileSync(join(r.dir, "scripts/conventions-baseline.json"), JSON.stringify({ maxLines: 500, legacy: {
      "src/legacy.ts": { lines: 701, smells: 0 }, "src/old.css": { lines: 2, smells: 0, literals: 2 } } }));
    execFileSync("git", ["-c", "user.email=t@t", "-c", "user.name=t", "commit", "-qam", "baseline"], { cwd: r.dir });
    execFileSync("git", ["-c", "user.email=t@t", "-c", "user.name=t", "add", "-A"], { cwd: r.dir });
    execFileSync("git", ["-c", "user.email=t@t", "-c", "user.name=t", "commit", "-qm", "old"], { cwd: r.dir });
    writeFileSync(join(r.dir, "src/old.css"), ".a { color: var(--text-main); padding: 8px; }\n");
    let out = r.run();
    assert.equal(out.status, 0, out.stdout);
    assert.match(out.stdout, /legacy, improved/);
    writeFileSync(join(r.dir, "src/old.css"), ".a { color: #fff; padding: 8px; margin: 4px; }\n");
    out = r.run();
    assert.equal(out.status, 1);
    assert.match(out.stdout, /3 hardcoded style values \(legacy limit 2\)/);
  } finally { r.cleanup(); }
});
