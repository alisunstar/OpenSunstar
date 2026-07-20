#!/usr/bin/env node
/**
 * Validate i18next locale JSON files for key alignment against a source locale.
 *
 * Usage:
 *   node scripts/i18n-check.mjs                     # report + baseline regression check
 *   node scripts/i18n-check.mjs --strict            # fail on any missing/extra keys
 *   node scripts/i18n-check.mjs --report-only       # always exit 0 (print summary)
 *   node scripts/i18n-check.mjs --write-baseline    # refresh docs/i18n/baseline.json
 *   node scripts/i18n-check.mjs --source en         # source locale (default: en)
 *   node scripts/i18n-check.mjs --locale ja,zh-TW   # limit compared locales
 *
 * See docs/i18n/README.md for workflow and conventions.
 */

import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const repoRoot = process.cwd();
const localesDir = path.join(repoRoot, "src/i18n/locales");
const defaultBaselinePath = path.join(repoRoot, "docs/i18n/baseline.json");

const args = process.argv.slice(2);
const flags = new Set(args.filter((a) => a.startsWith("--")));
const getFlagValue = (name) => {
  const eq = args.find((a) => a.startsWith(`${name}=`));
  if (eq) return eq.slice(name.length + 1);
  const idx = args.indexOf(name);
  if (idx >= 0) {
    const next = args[idx + 1];
    if (next && !next.startsWith("--")) return next;
  }
  return undefined;
};

const sourceLocale = getFlagValue("--source") ?? "en";
const localeFilter = getFlagValue("--locale")
  ?.split(",")
  .map((s) => s.trim())
  .filter(Boolean);
const strict = flags.has("--strict");
const reportOnly = flags.has("--report-only");
const writeBaseline = flags.has("--write-baseline");
const baselinePath = getFlagValue("--baseline") ?? defaultBaselinePath;
const listMissing = flags.has("--list-missing");
const maxList = Number(getFlagValue("--max-list") ?? "20");

/** @param {unknown} value */
function isPlainObject(value) {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/** @param {Record<string, unknown>} obj @param {string} [prefix] */
function flattenKeys(obj, prefix = "") {
  /** @type {string[]} */
  const keys = [];
  for (const [key, value] of Object.entries(obj)) {
    const fullKey = prefix ? `${prefix}.${key}` : key;
    if (isPlainObject(value)) {
      keys.push(...flattenKeys(value, fullKey));
    } else {
      keys.push(fullKey);
    }
  }
  return keys.sort();
}

/** @param {string} locale */
function loadLocale(locale) {
  const filePath = path.join(localesDir, `${locale}.json`);
  if (!fs.existsSync(filePath)) {
    throw new Error(`Locale file not found: ${filePath}`);
  }
  const raw = fs.readFileSync(filePath, "utf8");
  let parsed;
  try {
    parsed = JSON.parse(raw);
  } catch (error) {
    throw new Error(`Invalid JSON in ${filePath}: ${error.message}`);
  }
  if (!isPlainObject(parsed)) {
    throw new Error(`Locale root must be an object: ${filePath}`);
  }
  return parsed;
}

/** @param {string} dir */
function discoverLocales(dir) {
  return fs
    .readdirSync(dir)
    .filter((name) => name.endsWith(".json"))
    .map((name) => name.replace(/\.json$/, ""))
    .sort();
}

/** @param {Set<string>} sourceKeys @param {Set<string>} targetKeys */
function diffKeys(sourceKeys, targetKeys) {
  const missing = [...sourceKeys].filter((key) => !targetKeys.has(key));
  const extra = [...targetKeys].filter((key) => !sourceKeys.has(key));
  return { missing, extra };
}

/** @param {string} label @param {string[]} keys */
function printKeySample(label, keys) {
  if (keys.length === 0) return;
  console.log(`    ${label} (${keys.length}):`);
  for (const key of keys.slice(0, maxList)) {
    console.log(`      - ${key}`);
  }
  if (keys.length > maxList) {
    console.log(`      ... and ${keys.length - maxList} more`);
  }
}

function main() {
  const allLocales = discoverLocales(localesDir);
  if (!allLocales.includes(sourceLocale)) {
    console.error(`Source locale "${sourceLocale}" not found in ${localesDir}`);
    process.exit(1);
  }

  const sourceKeys = new Set(flattenKeys(loadLocale(sourceLocale)));
  const targetLocales = (localeFilter ?? allLocales.filter((l) => l !== sourceLocale))
    .filter((l) => l !== sourceLocale);

  /** @type {Record<string, { missing: number, extra: number, missingKeys: string[], extraKeys: string[] }>} */
  const report = {};

  console.log(`i18n key alignment check (source: ${sourceLocale}, ${sourceKeys.size} keys)`);
  console.log("");

  for (const locale of targetLocales) {
    const targetKeys = new Set(flattenKeys(loadLocale(locale)));
    const { missing, extra } = diffKeys(sourceKeys, targetKeys);
    report[locale] = {
      missing: missing.length,
      extra: extra.length,
      missingKeys: missing,
      extraKeys: extra,
    };

    const status =
      missing.length === 0 && extra.length === 0
        ? "OK"
        : missing.length > 0
          ? "MISSING"
          : "EXTRA";
    console.log(
      `[${status}] ${locale}: missing=${missing.length}, extra=${extra.length}, total=${targetKeys.size}`,
    );
    if (listMissing || strict) {
      printKeySample("missing", missing);
      printKeySample("extra", extra);
    }
  }

  if (writeBaseline) {
    const baseline = {
      source: sourceLocale,
      generatedAt: new Date().toISOString().slice(0, 10),
      sourceKeyCount: sourceKeys.size,
      locales: Object.fromEntries(
        Object.entries(report).map(([locale, data]) => [
          locale,
          { missing: data.missing, extra: data.extra },
        ]),
      ),
    };
    fs.mkdirSync(path.dirname(baselinePath), { recursive: true });
    fs.writeFileSync(baselinePath, `${JSON.stringify(baseline, null, 2)}\n`, "utf8");
    console.log("");
    console.log(`Baseline written to ${path.relative(repoRoot, baselinePath)}`);
    process.exit(0);
  }

  let failed = false;

  if (strict) {
    for (const [locale, data] of Object.entries(report)) {
      if (data.missing > 0 || data.extra > 0) {
        console.error("");
        console.error(
          `Strict check failed for ${locale}: missing=${data.missing}, extra=${data.extra}`,
        );
        failed = true;
      }
    }
  } else if (fs.existsSync(baselinePath)) {
    const baseline = JSON.parse(fs.readFileSync(baselinePath, "utf8"));
    console.log("");
    console.log(`Baseline regression check (${path.relative(repoRoot, baselinePath)}):`);
    for (const [locale, data] of Object.entries(report)) {
      const expected = baseline.locales?.[locale];
      if (!expected) {
        console.log(`  [NEW] ${locale}: no baseline entry (missing=${data.missing}, extra=${data.extra})`);
        continue;
      }
      if (data.missing > expected.missing) {
        console.error(
          `  [REGRESSION] ${locale}: missing keys increased ${expected.missing} -> ${data.missing}`,
        );
        failed = true;
      } else if (data.missing < expected.missing) {
        console.log(
          `  [IMPROVED] ${locale}: missing keys decreased ${expected.missing} -> ${data.missing} (update baseline with --write-baseline)`,
        );
      } else {
        console.log(`  [OK] ${locale}: missing=${data.missing}, extra=${data.extra}`);
      }
    }
  } else {
    console.log("");
    console.log("No baseline file found; skipping regression check.");
    console.log("Run: pnpm i18n:baseline");
  }

  if (reportOnly) {
    process.exit(0);
  }

  process.exit(failed ? 1 : 0);
}

try {
  main();
} catch (error) {
  console.error(error instanceof Error ? error.message : error);
  process.exit(1);
}
