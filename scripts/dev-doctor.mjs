import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const checks = [];
const commandTimeoutMs = Number.parseInt(process.env.DEV_DOCTOR_TIMEOUT_MS || "15000", 10);

function readProjectFile(relativePath) {
  return readFileSync(join(rootDir, relativePath), "utf8");
}

function record(name, ok, detail, fix) {
  checks.push({ name, ok, detail, fix });
}

function firstMatch(source, pattern) {
  const match = source.match(pattern);
  return match ? match[1] : null;
}

function runVersion(command, args, parser) {
  const result = spawnSync(command, args, {
    cwd: rootDir,
    encoding: "utf8",
    shell: process.platform === "win32",
    timeout: commandTimeoutMs,
  });

  if (result.error) {
    return { ok: false, error: result.error.message };
  }

  if (result.status !== 0) {
    const output = `${result.stderr || ""}${result.stdout || ""}`.trim();
    return { ok: false, error: output || `exit code ${result.status}` };
  }

  return { ok: true, version: parser((result.stdout || "").trim()) };
}

const nodeExpected = readProjectFile(".node-version").trim();
const packageJson = JSON.parse(readProjectFile("package.json"));
const pnpmExpected = firstMatch(packageJson.packageManager || "", /^pnpm@(.+)$/);
const rustToolchain = readProjectFile("rust-toolchain.toml");
const rustExpected = firstMatch(rustToolchain, /channel\s*=\s*"([^"]+)"/);
const cargoToml = readProjectFile("src-tauri/Cargo.toml");
const cargoRustVersion = firstMatch(cargoToml, /rust-version\s*=\s*"([^"]+)"/);

record(
  "Node.js",
  process.versions.node === nodeExpected,
  `expected ${nodeExpected}, found ${process.versions.node}`,
  "Install the version from .node-version.",
);

record(
  "packageManager",
  Boolean(pnpmExpected),
  pnpmExpected
    ? `package.json pins pnpm ${pnpmExpected}`
    : "package.json packageManager must be pnpm@<version>",
  "Set packageManager to pnpm@11.5.2.",
);

const pnpmCommand = process.env.PNPM || "pnpm";
const pnpmResult = runVersion(pnpmCommand, ["--version"], (output) => output.split(/\s+/)[0]);
record(
  "pnpm",
  pnpmResult.ok && pnpmResult.version === pnpmExpected,
  pnpmResult.ok
    ? `expected ${pnpmExpected}, found ${pnpmResult.version}`
    : `expected ${pnpmExpected}, failed to run ${pnpmCommand}: ${pnpmResult.error}`,
  "Enable Corepack or install the pinned pnpm version.",
);

record(
  "Rust toolchain",
  Boolean(rustExpected),
  rustExpected
    ? `rust-toolchain.toml pins Rust ${rustExpected}`
    : "rust-toolchain.toml must define channel",
  "Set rust-toolchain.toml channel to 1.95.0.",
);

record(
  "Cargo rust-version",
  cargoRustVersion === rustExpected,
  `rust-version ${cargoRustVersion || "missing"}, toolchain ${rustExpected || "missing"}`,
  "Keep src-tauri/Cargo.toml rust-version aligned with rust-toolchain.toml.",
);

const cargoCommand = process.env.CARGO || "cargo";
const cargoResult = runVersion(cargoCommand, ["--version"], (output) => {
  const match = output.match(/^cargo\s+([0-9]+\.[0-9]+\.[0-9]+)/);
  return match ? match[1] : output;
});
record(
  "Cargo",
  cargoResult.ok && cargoResult.version === rustExpected,
  cargoResult.ok
    ? `expected ${rustExpected}, found ${cargoResult.version}`
    : `expected ${rustExpected}, failed to run ${cargoCommand}: ${cargoResult.error}`,
  "Install the pinned Rust toolchain and ensure cargo is on PATH.",
);

for (const check of checks) {
  const status = check.ok ? "ok" : "fail";
  console.log(`[${status}] ${check.name}: ${check.detail}`);
  if (!check.ok) {
    console.log(`       fix: ${check.fix}`);
  }
}

const failed = checks.filter((check) => !check.ok);
if (failed.length > 0) {
  console.error(`\n${failed.length} toolchain check(s) failed.`);
  process.exit(1);
}

console.log("\nToolchain checks passed.");
