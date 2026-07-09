#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import { nativeBinaryPath } from "../lib/paths.js";
import { describeHost, resolvePlatform } from "../lib/platform.js";

const platformInfo = resolvePlatform();
const binaryName = platformInfo?.binaryName ?? (process.platform === "win32" ? "os.exe" : "os");
const binaryPath = nativeBinaryPath(binaryName);

if (!fs.existsSync(binaryPath)) {
  console.error(`[opensunstar-os] Native binary missing for ${describeHost()}.`);
  console.error("[opensunstar-os] Try:");
  console.error("  npm rebuild opensunstar-os");
  console.error("  # or download from https://github.com/alisunstar/OpenSunstar/releases");
  process.exit(1);
}

const result = spawnSync(binaryPath, process.argv.slice(2), {
  stdio: "inherit",
  env: process.env,
});

if (result.error) {
  console.error(`[opensunstar-os] Failed to run ${binaryPath}: ${result.error.message}`);
  process.exit(1);
}

process.exit(result.status ?? 1);
