// Finds hardcoded styling values that belong in src/styles/tokens.css
// (docs/CONVENTIONS.md section 6). Used by audit-conventions.mjs.
//
// Counted: colors (hex, rgb/hsl/oklch/lab functions), font families, and px/rem
// font sizes, numeric font weights, spacing (padding, margin, gap), radii,
// global z-index layers (10 and up) and ms/s durations.
// Allowed: 0, 1px hairlines, %, em, viewport units, keywords, var(), calc() of
// tokens, local stacking (z-index below 10), and local geometry custom
// properties (only colors count inside a `--name:` definition).

export const TOKEN_FILE = "src/styles/tokens.css";

const COLOR = /#[0-9a-fA-F]{3,8}\b|\b(?:rgba?|hsla?|oklch|oklab|lab|lch)\(/g;
const LENGTH = /(?<![\w.#-])-?\d*\.?\d+(px|rem)\b/g;
const DURATION = /(?<![\w.#-])\d*\.?\d+m?s\b/g;
const NUMBER = /(?<![\w.#-])-?\d*\.?\d+(?![\w.%])/g;

/** Lengths other than 0 and 1px hairlines. */
const lengths = (value) => (value.match(LENGTH) || []).filter((n) => !/^-?0*\.?0+(px|rem)$|^-?1px$/.test(n)).length;
const durations = (value) => (value.match(DURATION) || []).filter((n) => !/^0*\.?0+m?s$|^0?\.01ms$/.test(n)).length;
const colors = (value) => (value.replace(/url\([^)]*\)/g, "").match(COLOR) || []).length;

const SPACING = /^(font-size|padding(-[a-z-]+)?|margin(-[a-z-]+)?|gap|row-gap|column-gap)$|radius$/;

const numbers = (value) => value.match(NUMBER) || [];
const isFontFamily = (value) => !/^\s*(var\(|inherit|initial|unset)/.test(value);
const hasNumericWeight = (value) => /(?<![\w.])[1-9][05]0(?![\w.])/.test(value);

/** Literal counters by property; the first matching rule wins. */
const RULES = [
  [(p) => p.startsWith("--"), () => 0],
  [(p) => p === "font-family", (v) => (isFontFamily(v) ? 1 : 0)],
  [(p) => p === "font", (v) => lengths(v) + (hasNumericWeight(v) ? 1 : 0)],
  [(p) => p === "font-weight", (v) => numbers(v).length],
  [(p) => p === "z-index", (v) => numbers(v).filter((n) => Math.abs(Number(n)) >= 10).length],
  [(p) => SPACING.test(p), lengths],
  [(p) => /^(transition|animation)/.test(p), durations],
];

/** Non-color literals in one declaration (CSS property name, kebab-case). */
function nonColor(prop, value) {
  const rule = RULES.find(([applies]) => applies(prop));
  return rule ? rule[1](value) : 0;
}

const kebab = (key) => key.replace(/^["']|["']$/g, "").replace(/[A-Z]/g, (m) => "-" + m.toLowerCase());

function stripCss(text) {
  return text.replace(/\/\*[\s\S]*?\*\//g, "");
}

function countCss(text) {
  let total = 0;
  const decl = /(?<=[{;\s])(--[\w-]+|[a-z-]+)\s*:\s*([^;{}]+)(?=[;}])/g;
  for (const m of stripCss(text).matchAll(decl)) total += colors(m[2]) + nonColor(m[1], m[2]);
  return total;
}

function stripTs(text) {
  return text.replace(/\/\*[\s\S]*?\*\//g, "").replace(/^\s*\/\/.*$/gm, "").replace(/\{\s*\/\*[\s\S]*?\*\/\s*\}/g, "");
}

/** Inline style keys (`fontSize: 12`, `padding: "8px 12px"`) plus colors in any string. */
function countTs(text) {
  const src = stripTs(text);
  let total = 0;
  const prop = /(?<![\w$.])(["']?--[\w-]+["']?|[a-zA-Z]+)\s*:\s*(`[^`]*`|"[^"]*"|'[^']*'|-?\d*\.?\d+)/g;
  for (const m of src.matchAll(prop)) {
    const css = kebab(m[1]);
    const value = /^-?\d/.test(m[2]) ? numberValue(css, m[2]) : m[2].slice(1, -1);
    if (value !== null) total += nonColor(css, value);
  }
  for (const m of src.matchAll(/(["'`])((?:(?!\1)[^\\\n]|\\.)*)\1/g)) total += colors(m[2]);
  return total;
}

/** A bare number in a style object: React adds px to lengths. */
function numberValue(css, raw) {
  if (css === "font-weight" || css === "z-index") return raw;
  if (/^(font-size|padding|margin|gap|row-gap|column-gap)|radius$/.test(css)) return `${raw}px`;
  return null;
}

/** Hardcoded style values in one file (0 for files the rule does not cover). */
export function countStyleLiterals(path, text) {
  if (!isStyleSource(path)) return 0;
  return path.endsWith(".css") ? countCss(text) : countTs(text);
}

export function isStyleSource(path) {
  if (!path.startsWith("src/") || path === TOKEN_FILE) return false;
  if (/design-exploration\/|\.d\.ts$/.test(path)) return false;
  return /\.(css|tsx?)$/.test(path);
}
