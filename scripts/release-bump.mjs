#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import process from "node:process";

/**
 * Version bump + Keep a Changelog [Unreleased] promotion + release notes scaffold.
 *
 * Usage:
 *   node scripts/release-bump.mjs                 # default: patch
 *   node scripts/release-bump.mjs patch           # 1.1.2 -> 1.1.3
 *   node scripts/release-bump.mjs minor           # 1.1.2 -> 1.2.0
 *   node scripts/release-bump.mjs major           # 1.1.2 -> 2.0.0
 *   node scripts/release-bump.mjs 1.4.0           # explicit version
 *   node scripts/release-bump.mjs patch --commit  # also git add + commit
 *   node scripts/release-bump.mjs --dry-run       # preview only
 *   node scripts/release-bump.mjs patch --date 2026-07-09
 *
 * Workflow: edit CHANGELOG [Unreleased] during dev → run release:patch → tag.
 */

const args = process.argv.slice(2);
const flags = args.filter((a) => a.startsWith("--"));
const positional = args.find((a) => !a.startsWith("--"));
const shouldCommit = flags.includes("--commit");
const dryRun = flags.includes("--dry-run");
function resolveReleaseDate(argv) {
  const dateEq = argv.find((f) => f.startsWith("--date="));
  if (dateEq) return dateEq.slice("--date=".length);
  const dateIdx = argv.indexOf("--date");
  if (dateIdx >= 0) {
    const next = argv[dateIdx + 1];
    if (next && !next.startsWith("--")) return next;
  }
  return new Date().toISOString().slice(0, 10);
}
const releaseDate = resolveReleaseDate(args);

const repoRoot = process.cwd();
const packageJsonPath = path.join(repoRoot, "package.json");
const tauriConfigPath = path.join(repoRoot, "src-tauri", "tauri.conf.json");
const cargoTomlPath = path.join(repoRoot, "src-tauri", "Cargo.toml");
const cargoLockPath = path.join(repoRoot, "src-tauri", "Cargo.lock");
const changelogPath = path.join(repoRoot, "CHANGELOG.md");
const releaseNotesDir = path.join(repoRoot, "docs", "release-notes");
const releaseNotesReadme = path.join(releaseNotesDir, "README.md");
const schemaPath = path.join(repoRoot, "src-tauri", "src", "database", "mod.rs");

const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"));
const currentVersion = packageJson.version;

const semverMatch = /^(\d+)\.(\d+)\.(\d+)$/.exec(currentVersion);
if (!semverMatch) {
  console.error(
    `Error: current package.json version is not clean SemVer: ${currentVersion}`,
  );
  process.exit(1);
}
const [, curMajor, curMinor, curPatch] = semverMatch.map(Number);

function computeNextVersion(input) {
  const bump = (input ?? "patch").toLowerCase();

  if (bump === "major") return `${curMajor + 1}.0.0`;
  if (bump === "minor") return `${curMajor}.${curMinor + 1}.0`;
  if (bump === "patch") return `${curMajor}.${curMinor}.${curPatch + 1}`;

  const explicit = bump.startsWith("v") ? bump.slice(1) : bump;
  if (!/^\d+\.\d+\.\d+$/.test(explicit)) {
    console.error(
      `Error: argument must be one of patch|minor|major or a SemVer (x.y.z). Got: ${input}`,
    );
    process.exit(1);
  }
  return explicit;
}

const version = computeNextVersion(positional);

function versionTuple(v) {
  return v.split(".").map(Number);
}
const [nMaj, nMin, nPat] = versionTuple(version);
const isGreater =
  nMaj > curMajor ||
  (nMaj === curMajor && nMin > curMinor) ||
  (nMaj === curMajor && nMin === curMinor && nPat > curPatch);

if (!isGreater) {
  console.error(
    `Error: target version ${version} is not greater than current ${currentVersion}.`,
  );
  process.exit(1);
}

if (!/^\d{4}-\d{2}-\d{2}$/.test(releaseDate)) {
  console.error(`Error: --date must be YYYY-MM-DD, got: ${releaseDate}`);
  process.exit(1);
}

function readSchemaVersion() {
  if (!fs.existsSync(schemaPath)) return null;
  const m = fs.readFileSync(schemaPath, "utf8").match(/SCHEMA_VERSION:\s*i32\s*=\s*(\d+)/);
  return m ? Number(m[1]) : null;
}

/** Parse Keep a Changelog sections after the header. */
function parseChangelogSections(content) {
  const firstHeading = content.search(/^## \[/m);
  const header = firstHeading === -1 ? content : content.slice(0, firstHeading);
  const body = firstHeading === -1 ? "" : content.slice(firstHeading);

  const parts = body.split(/^## \[/m).filter(Boolean);
  const sections = parts.map((part) => {
    const titleEnd = part.indexOf("]");
    const title = part.slice(0, titleEnd);
    let rest = part.slice(titleEnd + 1);
    let date = null;
    const dateMatch = rest.match(/^\s*-\s*(\S+)\s*\n/);
    if (dateMatch) {
      date = dateMatch[1];
      rest = rest.slice(dateMatch[0].length);
    } else {
      const nl = rest.indexOf("\n");
      rest = nl >= 0 ? rest.slice(nl + 1) : "";
    }
    return {
      title,
      date,
      body: rest.replace(/<!--[\s\S]*?-->/g, "").trim(),
    };
  });
  return { header, sections };
}

function promoteChangelog(content, nextVersion, date) {
  const { header, sections } = parseChangelogSections(content);
  const unreleasedIdx = sections.findIndex((s) => s.title === "Unreleased");
  const unreleasedBody =
    unreleasedIdx >= 0 ? sections[unreleasedIdx].body : "";
  const otherSections = sections.filter((s) => s.title !== "Unreleased");

  const releaseBody =
    unreleasedBody ||
    "_No unreleased entries; version bump only._";

  const rebuilt = [
    header.trimEnd() + "\n\n",
    "## [Unreleased]\n\n",
    `## [${nextVersion}] - ${date}\n\n${releaseBody}\n`,
    ...otherSections.map(
      (s) =>
        `## [${s.title}]${s.date ? ` - ${s.date}` : ""}\n\n${s.body}\n`,
    ),
  ].join("\n");

  return { changelog: rebuilt, unreleasedBody, releaseBody };
}

function formatReleaseNotesBody(body) {
  if (!body || body === "_No unreleased entries; version bump only._") {
    return "_Version bump; see [CHANGELOG](../../CHANGELOG.md) for history._";
  }
  return body;
}

function buildReleaseNoteEn(nextVersion, date, body, schemaVersion) {
  const changes = formatReleaseNotesBody(body);
  const schemaLine = schemaVersion ? `v${schemaVersion}` : "see CHANGELOG";
  return `# OpenSunstar v${nextVersion} Release Notes

**Release date:** ${date}  
**License:** MIT  
**Database schema:** ${schemaLine}

---

## Overview

OpenSunstar **v${nextVersion}** — see [CHANGELOG](../../CHANGELOG.md) for the full history.

**Download:** [GitHub Releases](https://github.com/alisunstar/OpenSunstar/releases/latest)

---

## Changes

${changes}

---

## Documentation

- [CHANGELOG](../../CHANGELOG.md)
- [VERSIONING.md](../VERSIONING.md)
- [Portfolio module (kanban.md)](../kanban.md)

---

**MIT License** — Core is open source; team/enterprise capabilities may be offered separately.
`;
}

function buildReleaseNoteZh(nextVersion, date, body, schemaVersion) {
  const changes = formatReleaseNotesBody(body);
  const schemaLine = schemaVersion ? `v${schemaVersion}` : "见 CHANGELOG";
  return `# OpenSunstar v${nextVersion} 发布说明

**发布日期：** ${date}  
**许可证：** MIT  
**数据库 schema：** ${schemaLine}

---

## 概述

OpenSunstar **v${nextVersion}** — 完整变更见 [CHANGELOG](../../CHANGELOG.md)。

**下载：** [GitHub Releases](https://github.com/alisunstar/OpenSunstar/releases/latest)

---

## 变更内容

${changes}

---

## 文档

- [CHANGELOG](../../CHANGELOG.md)
- [VERSIONING.md](../VERSIONING.md)
- [工作区模块说明](../kanban.md)

`;
}

function updateReleaseNotesReadme(content, nextVersion, date) {
  const row = `| **v${nextVersion}** (${date}) | [v${nextVersion}-en.md](v${nextVersion}-en.md) | [v${nextVersion}-zh.md](v${nextVersion}-zh.md) | — | — | — |`;
  if (content.includes(`v${nextVersion}-en.md`)) {
    return { content, updated: false };
  }
  const lines = content.split("\n");
  const headerIdx = lines.findIndex((l) => l.startsWith("| -------"));
  if (headerIdx === -1) {
    return { content: `${content.trimEnd()}\n${row}\n`, updated: true };
  }
  lines.splice(headerIdx + 1, 0, row);
  return { content: lines.join("\n"), updated: true };
}

// ── Plan all file changes ──
const schemaVersion = readSchemaVersion();
const changelogBefore = fs.readFileSync(changelogPath, "utf8");
const { changelog: changelogAfter, unreleasedBody, releaseBody } =
  promoteChangelog(changelogBefore, version, releaseDate);

const enPath = path.join(releaseNotesDir, `v${version}-en.md`);
const zhPath = path.join(releaseNotesDir, `v${version}-zh.md`);
const enContent = buildReleaseNoteEn(version, releaseDate, releaseBody, schemaVersion);
const zhContent = buildReleaseNoteZh(version, releaseDate, releaseBody, schemaVersion);

const readmeBefore = fs.existsSync(releaseNotesReadme)
  ? fs.readFileSync(releaseNotesReadme, "utf8")
  : "";
const { content: readmeAfter, updated: readmeUpdated } = updateReleaseNotesReadme(
  readmeBefore,
  version,
  releaseDate,
);

console.log(`Current version: ${currentVersion}`);
console.log(`Next version:    ${version}`);
console.log(`Release date:    ${releaseDate}`);
console.log(`Schema version:  ${schemaVersion ?? "unknown"}`);
console.log("");
console.log(
  `[Unreleased] content: ${unreleasedBody ? `${unreleasedBody.split("\n").length} lines` : "(empty — placeholder will be used)"}`,
);
if (unreleasedBody) {
  console.log("--- Unreleased preview ---");
  console.log(unreleasedBody.slice(0, 800));
  if (unreleasedBody.length > 800) console.log("... (truncated)");
  console.log("--------------------------");
}
console.log("");

if (dryRun) {
  console.log("Dry run — no files were modified.");
  console.log(`Would write: CHANGELOG.md, ${path.relative(repoRoot, enPath)}, ${path.relative(repoRoot, zhPath)}`);
  process.exit(0);
}

// ── package.json ──
packageJson.version = version;
fs.writeFileSync(
  packageJsonPath,
  `${JSON.stringify(packageJson, null, 2)}\n`,
  "utf8",
);

// ── tauri.conf.json ──
const tauriConfig = JSON.parse(fs.readFileSync(tauriConfigPath, "utf8"));
const oldTauriVersion = tauriConfig.version;
tauriConfig.version = version;
fs.writeFileSync(
  tauriConfigPath,
  `${JSON.stringify(tauriConfig, null, 2)}\n`,
  "utf8",
);

// ── Cargo.toml ──
let cargoToml = fs.readFileSync(cargoTomlPath, "utf8");
const oldCargoVersion = cargoToml.match(/^version = "([^"]+)"/m)?.[1] ?? "unknown";
cargoToml = cargoToml.replace(/^version = "[^"]+"/m, `version = "${version}"`);
fs.writeFileSync(cargoTomlPath, cargoToml, "utf8");

// ── Cargo.lock ──
let cargoLockUpdated = false;
if (fs.existsSync(cargoLockPath)) {
  let cargoLock = fs.readFileSync(cargoLockPath, "utf8");
  const next = cargoLock.replace(
    /(name = "OpenSunstar"\nversion = ")[^"]+(")/,
    `$1${version}$2`,
  );
  if (next !== cargoLock) {
    fs.writeFileSync(cargoLockPath, next, "utf8");
    cargoLockUpdated = true;
  }
}

// ── README badges ──
const readmeFiles = ["README.md", "README_ZH.md", "README_JA.md", "README_DE.md"];
const updatedReadmes = [];
for (const rel of readmeFiles) {
  const p = path.join(repoRoot, rel);
  if (!fs.existsSync(p)) continue;
  const before = fs.readFileSync(p, "utf8");
  const after = before.replace(
    /(badge\/version-v)\d+\.\d+\.\d+(-blue\.svg)/g,
    `$1${version}$2`,
  );
  if (after !== before) {
    fs.writeFileSync(p, after, "utf8");
    updatedReadmes.push(rel);
  }
}

// ── CHANGELOG + release notes ──
fs.writeFileSync(changelogPath, changelogAfter, "utf8");
fs.mkdirSync(releaseNotesDir, { recursive: true });
fs.writeFileSync(enPath, enContent, "utf8");
fs.writeFileSync(zhPath, zhContent, "utf8");
if (readmeUpdated) {
  fs.writeFileSync(releaseNotesReadme, readmeAfter, "utf8");
}

console.log(`Updated package.json:     ${currentVersion} -> ${version}`);
console.log(`Updated tauri.conf.json:  ${oldTauriVersion} -> ${version}`);
console.log(`Updated Cargo.toml:       ${oldCargoVersion} -> ${version}`);
console.log(
  `Updated Cargo.lock:       ${cargoLockUpdated ? "yes" : "no change / not found"}`,
);
console.log(
  `Updated README badges:    ${updatedReadmes.length ? updatedReadmes.join(", ") : "no change"}`,
);
console.log(`Promoted CHANGELOG:       [Unreleased] -> [${version}] - ${releaseDate}`);
console.log(`Created release notes:    v${version}-en.md, v${version}-zh.md`);
console.log(
  `Updated release index:    ${readmeUpdated ? "docs/release-notes/README.md" : "already present"}`,
);
console.log("");
console.log("Next steps:");
console.log("  1. Review CHANGELOG + release notes (edit highlights if needed)");
console.log("  2. pnpm typecheck && pnpm test:unit && (cd src-tauri && cargo test)");
console.log("  3. git add -A && git commit -m \"chore(release): v" + version + "\"");
console.log(`  4. pnpm release:tag        # auto-reads package.json → v${version}`);

if (shouldCommit) {
  const { spawnSync } = await import("node:child_process");
  const filesToAdd = [
    "package.json",
    "src-tauri/tauri.conf.json",
    "src-tauri/Cargo.toml",
    "src-tauri/Cargo.lock",
    "CHANGELOG.md",
    `docs/release-notes/v${version}-en.md`,
    `docs/release-notes/v${version}-zh.md`,
    ...(readmeUpdated ? ["docs/release-notes/README.md"] : []),
    ...updatedReadmes,
  ];
  const add = spawnSync("git", ["add", ...filesToAdd], { stdio: "inherit" });
  if (add.status !== 0) process.exit(add.status ?? 1);

  const commit = spawnSync(
    "git",
    ["commit", "-m", `chore(release): bump version to v${version}`],
    { stdio: "inherit" },
  );
  if (commit.status !== 0) process.exit(commit.status ?? 1);

  console.log(`\nCommitted version bump for v${version}`);
}
