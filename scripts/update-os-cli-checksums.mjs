#!/usr/bin/env node
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { normalizeVersion } from "../packages/os-cli/lib/checksums.js";
import {
  releaseAssetUrl,
  SUPPORTED_PLATFORMS,
} from "../packages/os-cli/lib/platform.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "..");
const osCliRoot = path.join(repoRoot, "packages", "os-cli");

function option(name) {
  const index = process.argv.indexOf(name);
  if (index === -1) {
    return null;
  }
  return process.argv[index + 1] ?? null;
}

function fail(message) {
  console.error(`[opensunstar-os] ${message}`);
  process.exit(1);
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function sha256File(filePath) {
  const hash = crypto.createHash("sha256");
  hash.update(fs.readFileSync(filePath));
  return hash.digest("hex");
}

const artifactsDir = path.resolve(
  repoRoot,
  option("--artifacts-dir") ?? "dist/os-cli-artifacts",
);
const pkg = readJson(path.join(osCliRoot, "package.json"));
const tag = option("--tag") ?? `v${pkg.version}`;
const version = normalizeVersion(tag);

if (!version) {
  fail("Release tag/version is empty.");
}

if (normalizeVersion(pkg.version) !== version) {
  fail(
    `packages/os-cli version ${pkg.version} does not match release tag ${tag}. Run scripts/sync-os-cli-version.mjs first.`,
  );
}

if (!fs.existsSync(artifactsDir)) {
  fail(`Artifact directory does not exist: ${artifactsDir}`);
}

const artifacts = {};
for (const platformInfo of SUPPORTED_PLATFORMS) {
  const { file } = releaseAssetUrl(tag, platformInfo);
  const artifactPath = path.join(artifactsDir, file);

  if (!fs.existsSync(artifactPath)) {
    fail(`Missing release artifact: ${artifactPath}`);
  }

  artifacts[file] = sha256File(artifactPath);
}

const manifest = {
  version,
  algorithm: "sha256",
  artifacts,
};
const outputPath = path.join(osCliRoot, "checksums.json");

fs.writeFileSync(outputPath, `${JSON.stringify(manifest, null, 2)}\n`);
console.log(`[opensunstar-os] Wrote ${path.relative(repoRoot, outputPath)}`);
