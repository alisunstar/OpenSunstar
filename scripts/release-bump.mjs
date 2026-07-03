#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import process from "node:process";

const args = process.argv.slice(2);
const versionArg = args.find((arg) => !arg.startsWith("--"));
const shouldCommit = args.includes("--commit");

if (!versionArg) {
  console.error("Usage: node scripts/release-bump.mjs <version> [--commit]");
  console.error("Example: node scripts/release-bump.mjs 0.1.1 --commit");
  process.exit(1);
}

const version = versionArg.startsWith("v") ? versionArg.slice(1) : versionArg;
if (!/^\d+\.\d+\.\d+$/.test(version)) {
  console.error(`Error: version must be SemVer (x.y.z), got: ${versionArg}`);
  process.exit(1);
}

const repoRoot = process.cwd();
const packageJsonPath = path.join(repoRoot, "package.json");
const tauriConfigPath = path.join(repoRoot, "src-tauri", "tauri.conf.json");

const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, "utf8"));
const tauriConfig = JSON.parse(fs.readFileSync(tauriConfigPath, "utf8"));

const oldPackageVersion = packageJson.version;
const oldTauriVersion = tauriConfig.version;

packageJson.version = version;
tauriConfig.version = version;

fs.writeFileSync(packageJsonPath, `${JSON.stringify(packageJson, null, 2)}\n`, "utf8");
fs.writeFileSync(tauriConfigPath, `${JSON.stringify(tauriConfig, null, 2)}\n`, "utf8");

console.log(`Updated package.json: ${oldPackageVersion} -> ${version}`);
console.log(`Updated tauri.conf.json: ${oldTauriVersion} -> ${version}`);

if (shouldCommit) {
  const { spawnSync } = await import("node:child_process");
  const add = spawnSync("git", ["add", "package.json", "src-tauri/tauri.conf.json"], {
    stdio: "inherit",
  });
  if (add.status !== 0) process.exit(add.status ?? 1);

  const commit = spawnSync("git", ["commit", "-m", `chore(release): bump version to v${version}`], {
    stdio: "inherit",
  });
  if (commit.status !== 0) process.exit(commit.status ?? 1);

  console.log(`Committed version bump for v${version}`);
}
