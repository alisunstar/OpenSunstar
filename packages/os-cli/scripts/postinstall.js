#!/usr/bin/env node
import fs from "node:fs";
import { expectedSha256ForAsset } from "../lib/checksums.js";
import { downloadToFile } from "../lib/download.js";
import {
  assertBinaryExists,
  cleanupArchive,
  extractArchive,
} from "../lib/extract.js";
import {
  downloadCachePath,
  nativeBinaryPath,
  readPackageVersion,
  vendorDir,
} from "../lib/paths.js";
import {
  describeHost,
  releaseAssetUrl,
  resolvePlatform,
} from "../lib/platform.js";

function skipInstall() {
  return (
    process.env.OPEN_SUNSTAR_OS_SKIP_DOWNLOAD === "1" ||
    process.env.npm_config_ignore_scripts === "true"
  );
}

async function main() {
  if (skipInstall()) {
    console.log(
      "[opensunstar-os] Skipping binary download (OPEN_SUNSTAR_OS_SKIP_DOWNLOAD or ignore-scripts).",
    );
    console.log(
      "[opensunstar-os] Download manually from GitHub Releases or run: npm rebuild opensunstar-os",
    );
    return;
  }

  const platformInfo = resolvePlatform();
  if (!platformInfo) {
    console.warn(`[opensunstar-os] Unsupported platform: ${describeHost()}.`);
    console.warn(
      "[opensunstar-os] Install from GitHub Releases: https://github.com/alisunstar/OpenSunstar/releases",
    );
    return;
  }

  const version =
    process.env.OPEN_SUNSTAR_OS_VERSION?.trim() || readPackageVersion();
  const binaryPath = nativeBinaryPath(platformInfo.binaryName);

  if (fs.existsSync(binaryPath)) {
    console.log(`[opensunstar-os] Using cached binary at ${binaryPath}`);
    return;
  }

  const { file, url } = releaseAssetUrl(version, platformInfo);
  const archivePath = downloadCachePath(file);
  const expectedSha256 = expectedSha256ForAsset(version, file);

  fs.mkdirSync(vendorDir(), { recursive: true });

  console.log(`[opensunstar-os] Downloading ${file} ...`);
  await downloadToFile(url, archivePath, {
    expectedSha256,
    artifactName: file,
  });
  console.log(`[opensunstar-os] Verified SHA256 for ${file}.`);

  console.log(`[opensunstar-os] Extracting to ${vendorDir()} ...`);
  extractArchive(archivePath, vendorDir(), platformInfo.archiveExt);
  assertBinaryExists(binaryPath);
  cleanupArchive(archivePath);

  console.log(`[opensunstar-os] Ready: ${binaryPath}`);
}

main().catch((err) => {
  console.error(`[opensunstar-os] Install failed: ${err.message}`);
  console.error(
    "[opensunstar-os] Fallback: download from https://github.com/alisunstar/OpenSunstar/releases",
  );
  process.exit(1);
});
