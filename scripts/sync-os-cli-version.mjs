#!/usr/bin/env node
/**
 * Sync packages/os-cli/package.json version with root package.json.
 * Usage: node scripts/sync-os-cli-version.mjs
 */
import fs from "node:fs";
import path from "node:path";

const root = process.cwd();
const rootPkg = JSON.parse(
  fs.readFileSync(path.join(root, "package.json"), "utf8"),
);
const cliPkgPath = path.join(root, "packages", "os-cli", "package.json");
const cliPkg = JSON.parse(fs.readFileSync(cliPkgPath, "utf8"));

cliPkg.version = rootPkg.version;
fs.writeFileSync(cliPkgPath, `${JSON.stringify(cliPkg, null, 2)}\n`, "utf8");
console.log(`Synced opensunstar-os version → ${cliPkg.version}`);
