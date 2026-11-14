#!/usr/bin/env node
// Checks changed code against docs/CONVENTIONS.md.
//
//   npm run audit:conventions                      changes since HEAD (incl. uncommitted)
//   npm run audit:conventions -- --base prod       changes since a ref
//   npm run audit:conventions -- --all             every source file (legacy report)
//   npm run audit:conventions -- --write-baseline  re-record the legacy ratchet
//
// Rules (exit 1 on any failure):
//   - a source file over MAX_LINES fails unless it is a recorded legacy file that did not grow;
//   - qlty function smells (high complexity, many parameters/returns, deep nesting) fail
//     unless the file is recorded legacy and its smell count did not grow;
//   - a new SQL migration fails (older Kaze versions refuse unknown migrations).
// Legacy entries live in scripts/conventions-baseline.json and may only shrink.

import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

const MAX_LINES = 500;
const BASELINE = "scripts/conventions-baseline.json";
const SMELL = /Function with (high complexity|many parameters|many returns)|Deeply nested control flow/;

const args = process.argv.slice(2);
const flag = (name) => args.includes(name);
const option = (name, fallback) => {
  const at = args.indexOf(name);
  return at >= 0 && args[at + 1] ? args[at + 1] : fallback;
};
const base = option("--base", "HEAD");

const git = (...a) => execFileSync("git", a, { encoding: "utf8" }).split("\n").filter(Boolean);

function isSource(path) {
  if (!/\.(rs|ts|tsx)$/.test(path)) return false;
  if (!path.startsWith("src/") && !path.startsWith("src-tauri/src/")) return false;
  return !/(_tests\.rs|\/tests?\/|design-exploration\/|\.d\.ts$)/.test(path);
}

function changedFiles() {
  const tracked = git("diff", "--name-only", base);
  const untracked = git("ls-files", "--others", "--exclude-standard");
  return [...new Set([...tracked, ...untracked])].filter(isSource).filter((p) => existsSync(p));
}

const allFiles = () => git("ls-files").filter(isSource).filter((p) => existsSync(p));
const lineCount = (path) => readFileSync(path, "utf8").split("\n").length;

function qltyBinary() {
  const local = join(homedir(), ".qlty", "bin", "qlty");
  if (existsSync(local)) return local;
  try { execFileSync("qlty", ["--version"], { stdio: "ignore" }); return "qlty"; } catch { return null; }
}

/** Function-level smell count per file, from qlty. */
function smellCounts(files) {
  const counts = Object.fromEntries(files.map((f) => [f, 0]));
  const qlty = process.env.KAZE_AUDIT_NO_QLTY ? null : qltyBinary();
  if (!qlty || !files.length) return { counts, available: !!qlty };
  let out;
  try {
    out = execFileSync(qlty, ["smells", "--no-snippets", "--quiet", "--no-duplication", ...files], {
      encoding: "utf8", maxBuffer: 64 * 1024 * 1024, stdio: ["ignore", "pipe", "ignore"],
    }).replace(/\x1b\[[0-9;]*m/g, "");
  } catch {
    return { counts, available: false };
  }
  let current = null;
  for (const line of out.split("\n")) {
    // Smell lines start with a line number ("  42  ..." or "1234  ..."); headers are paths.
    if (/^\s*\d+\s{2}/.test(line)) { if (current in counts && SMELL.test(line)) counts[current] += 1; }
    else if (/^\S/.test(line)) current = line.trim();
  }
  return { counts, available: true };
}

function writeBaseline() {
  const files = allFiles();
  const { counts, available } = smellCounts(files);
  if (!available) throw new Error("qlty is required to record the baseline");
  const legacy = {};
  for (const file of files) {
    const lines = lineCount(file);
    if (lines > MAX_LINES || counts[file] > 0) legacy[file] = { lines, smells: counts[file] };
  }
  writeFileSync(BASELINE, JSON.stringify({ maxLines: MAX_LINES, legacy }, null, 2) + "\n");
  console.log(`Recorded ${Object.keys(legacy).length} legacy files in ${BASELINE}.`);
}

function audit() {
  const baseline = existsSync(BASELINE) ? JSON.parse(readFileSync(BASELINE, "utf8")).legacy : {};
  const files = flag("--all") ? allFiles() : changedFiles();
  const { counts, available } = smellCounts(files);
  const rows = [];
  let failures = 0;
  for (const file of files) {
    const lines = lineCount(file);
    const smells = counts[file];
    const legacy = baseline[file];
    const problems = [];
    if (lines > MAX_LINES && (!legacy || lines > legacy.lines)) {
      problems.push(legacy ? `grew to ${lines} lines (legacy limit ${legacy.lines})` : `${lines} lines (max ${MAX_LINES})`);
    }
    if (smells > (legacy?.smells ?? 0)) {
      problems.push(legacy ? `${smells} qlty smells (legacy limit ${legacy.smells})` : `${smells} qlty smells`);
    }
    const shrunk = legacy && (lines < legacy.lines || smells < legacy.smells);
    const status = problems.length ? "FAIL" : legacy ? (shrunk ? "legacy, improved" : "legacy") : "ok";
    if (problems.length) failures += 1;
    if (problems.length || legacy || flag("--verbose")) rows.push({ file, lines, smells, status, note: problems.join("; ") });
  }
  const newMigrations = flag("--all") ? [] : git("diff", "--name-only", "--diff-filter=A", base, "--", "src-tauri/migrations")
    .concat(git("ls-files", "--others", "--exclude-standard", "src-tauri/migrations"));
  for (const migration of newMigrations) {
    failures += 1;
    rows.push({ file: migration, lines: "", smells: "", status: "FAIL", note: "new migration: record a compatibility decision first" });
  }

  console.log(`Convention audit: ${files.length} source file(s) ${flag("--all") ? "(all)" : `changed since ${base}`}`);
  if (!available) console.log("warning: qlty not found, complexity not checked");
  if (rows.length) {
    console.log("\n| File | Lines | Smells | Status | Note |\n|---|---|---|---|---|");
    for (const r of rows) console.log(`| \`${r.file}\` | ${r.lines} | ${r.smells} | ${r.status} | ${r.note} |`);
  }
  console.log(failures ? `\n${failures} convention failure(s). See docs/CONVENTIONS.md.` : "\nNo convention failures.");
  process.exit(failures ? 1 : 0);
}

if (flag("--write-baseline")) writeBaseline();
else audit();
