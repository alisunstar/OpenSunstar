import fs from "node:fs";
import path from "node:path";
import { packageRoot } from "./paths.js";
import { releaseAssetUrl, SUPPORTED_PLATFORMS } from "./platform.js";

const SHA256_RE = /^[a-f0-9]{64}$/;

export function normalizeVersion(version) {
  const normalized = String(version ?? "").trim();
  return normalized.startsWith("v") ? normalized.slice(1) : normalized;
}

export function checksumsPath() {
  return path.join(packageRoot(), "checksums.json");
}

export function readChecksums({ required = true } = {}) {
  const manifestPath = checksumsPath();

  if (!fs.existsSync(manifestPath)) {
    if (!required) {
      return null;
    }
    throw new Error(
      `Missing integrity manifest: ${manifestPath}. The npm package must include checksums.json generated from release artifacts.`,
    );
  }

  return JSON.parse(fs.readFileSync(manifestPath, "utf8"));
}

export function expectedFilesForVersion(version) {
  return SUPPORTED_PLATFORMS.map(
    (platformInfo) => releaseAssetUrl(version, platformInfo).file,
  );
}

export function validateChecksums(
  manifest,
  { version, requiredFiles = [] } = {},
) {
  const errors = [];

  if (!manifest || typeof manifest !== "object" || Array.isArray(manifest)) {
    return ["checksums.json must be a JSON object"];
  }

  if (manifest.algorithm !== "sha256") {
    errors.push('checksums.json "algorithm" must be "sha256"');
  }

  const manifestVersion = normalizeVersion(manifest.version);
  if (!manifestVersion) {
    errors.push('checksums.json must include a non-empty "version"');
  }

  if (version && manifestVersion !== normalizeVersion(version)) {
    errors.push(
      `checksums.json version ${manifest.version} does not match package version ${normalizeVersion(
        version,
      )}`,
    );
  }

  if (
    !manifest.artifacts ||
    typeof manifest.artifacts !== "object" ||
    Array.isArray(manifest.artifacts)
  ) {
    errors.push('checksums.json must include an "artifacts" object');
    return errors;
  }

  for (const file of requiredFiles) {
    if (!manifest.artifacts[file]) {
      errors.push(`checksums.json is missing SHA256 for ${file}`);
    }
  }

  for (const [file, checksum] of Object.entries(manifest.artifacts)) {
    if (typeof file !== "string" || !file.trim()) {
      errors.push("checksums.json contains an empty artifact filename");
    }

    if (typeof checksum !== "string" || !SHA256_RE.test(checksum)) {
      errors.push(`checksums.json has invalid SHA256 for ${file}`);
    }
  }

  return errors;
}

export function expectedSha256ForAsset(version, file) {
  const manifest = readChecksums();
  const errors = validateChecksums(manifest, {
    version,
    requiredFiles: [file],
  });

  if (errors.length > 0) {
    throw new Error(errors.join("; "));
  }

  return manifest.artifacts[file];
}
