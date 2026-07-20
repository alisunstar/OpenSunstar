#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  expectedFilesForVersion,
  normalizeVersion,
  readChecksums,
  validateChecksums,
} from "../packages/os-cli/lib/checksums.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "..");
const osCliRoot = path.join(repoRoot, "packages", "os-cli");
const requireChecksums = process.argv.includes("--require-checksums");

const errors = [];
const warnings = [];

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function failIf(condition, message) {
  if (condition) {
    errors.push(message);
  }
}

function cargoPackageVersion() {
  const cargoToml = fs.readFileSync(
    path.join(repoRoot, "src-tauri", "Cargo.toml"),
    "utf8",
  );
  const packageSection = cargoToml.match(/\[package\]([\s\S]*?)(?:\n\[|$)/);
  const versionMatch = packageSection?.[1]?.match(
    /^\s*version\s*=\s*"([^"]+)"/m,
  );
  return versionMatch?.[1] ?? null;
}

function parseNpmPackJson(stdout) {
  const start = stdout.indexOf("[");
  const end = stdout.lastIndexOf("]");

  if (start === -1 || end === -1 || end < start) {
    throw new Error(`npm pack did not return JSON: ${stdout}`);
  }

  return JSON.parse(stdout.slice(start, end + 1));
}

function runNpmPackDryRun() {
  const npmCommand = process.platform === "win32" ? "npm.cmd" : "npm";
  const args = ["pack", "--dry-run", "--json"];
  const spawnOptions = {
    cwd: osCliRoot,
    encoding: "utf8",
    env: {
      ...process.env,
      OPEN_SUNSTAR_OS_SKIP_DOWNLOAD: "1",
    },
  };
  const result =
    process.platform === "win32"
      ? spawnSync(`${npmCommand} ${args.join(" ")}`, {
          ...spawnOptions,
          shell: true,
        })
      : spawnSync(npmCommand, args, spawnOptions);

  if (result.error) {
    errors.push(`Failed to run npm pack --dry-run: ${result.error.message}`);
    return [];
  }

  if (result.status !== 0) {
    errors.push(
      `npm pack --dry-run failed with exit code ${result.status}: ${
        result.stderr || result.stdout
      }`,
    );
    return [];
  }

  try {
    return (
      parseNpmPackJson(result.stdout)[0]?.files?.map((file) => file.path) ?? []
    );
  } catch (err) {
    errors.push(err.message);
    return [];
  }
}

const rootPkg = readJson(path.join(repoRoot, "package.json"));
const osPkg = readJson(path.join(osCliRoot, "package.json"));
const cargoVersion = cargoPackageVersion();

failIf(
  osPkg.name !== "opensunstar-os",
  "npm package name must be opensunstar-os",
);
failIf(osPkg.type !== "module", "npm package type must be module");
failIf(
  osPkg.bin?.os !== "bin/os.js",
  "npm package bin.os must point to bin/os.js",
);
failIf(
  osPkg.scripts?.postinstall !== "node scripts/postinstall.js",
  "npm package postinstall must be node scripts/postinstall.js",
);
failIf(
  !String(osPkg.engines?.node ?? "").includes(">=18"),
  "npm package must require Node.js >=18",
);
failIf(
  normalizeVersion(rootPkg.version) !== normalizeVersion(osPkg.version),
  `root package version ${rootPkg.version} does not match packages/os-cli version ${osPkg.version}`,
);
failIf(
  cargoVersion !== normalizeVersion(osPkg.version),
  `src-tauri/Cargo.toml version ${cargoVersion ?? "unknown"} does not match packages/os-cli version ${osPkg.version}`,
);

for (const requiredFile of ["bin", "lib", "scripts", "checksums.json"]) {
  failIf(
    !osPkg.files?.includes(requiredFile),
    `packages/os-cli package.json files must include ${requiredFile}`,
  );
}
failIf(
  osPkg.files?.includes("vendor"),
  "packages/os-cli package.json files must not include vendor",
);

const binEntry = fs.readFileSync(path.join(osCliRoot, "bin", "os.js"), "utf8");
failIf(
  !binEntry.startsWith("#!/usr/bin/env node"),
  "bin/os.js must keep the Node shebang",
);

const expectedReleaseFiles = expectedFilesForVersion(osPkg.version);
let hasChecksums = false;

try {
  const checksums = readChecksums({ required: requireChecksums });
  if (checksums) {
    hasChecksums = true;
    const checksumErrors = validateChecksums(checksums, {
      version: osPkg.version,
      requiredFiles: expectedReleaseFiles,
    });
    errors.push(...checksumErrors);

    const expectedSet = new Set(expectedReleaseFiles);
    for (const file of Object.keys(checksums.artifacts ?? {})) {
      failIf(
        !expectedSet.has(file),
        `checksums.json contains unexpected artifact ${file}`,
      );
    }
  } else {
    warnings.push(
      "checksums.json is absent; local package-shape check passed, release publish must run with --require-checksums after generating it.",
    );
  }
} catch (err) {
  errors.push(err.message);
}

const packedFiles = runNpmPackDryRun();
const allowedFiles = new Set([
  "README.md",
  "bin/os.js",
  "lib/checksums.js",
  "lib/download.js",
  "lib/extract.js",
  "lib/paths.js",
  "lib/platform.js",
  "package.json",
  "scripts/postinstall.js",
]);

if (hasChecksums) {
  allowedFiles.add("checksums.json");
}

for (const requiredPackedFile of [
  "README.md",
  "bin/os.js",
  "lib/checksums.js",
  "lib/download.js",
  "lib/extract.js",
  "lib/paths.js",
  "lib/platform.js",
  "package.json",
  "scripts/postinstall.js",
]) {
  failIf(
    !packedFiles.includes(requiredPackedFile),
    `npm package is missing ${requiredPackedFile}`,
  );
}

failIf(
  requireChecksums && !packedFiles.includes("checksums.json"),
  "npm package is missing checksums.json",
);

for (const packedFile of packedFiles) {
  failIf(
    !allowedFiles.has(packedFile),
    `npm package contains unexpected file ${packedFile}`,
  );
  failIf(
    packedFile === "vendor" || packedFile.startsWith("vendor/"),
    `npm package must not include local vendor binary ${packedFile}`,
  );
}

for (const warning of warnings) {
  console.warn(`[opensunstar-os] warning: ${warning}`);
}

if (errors.length > 0) {
  for (const error of errors) {
    console.error(`[opensunstar-os] ${error}`);
  }
  process.exit(1);
}

console.log("[opensunstar-os] npm CLI package supply-chain checks passed.");
