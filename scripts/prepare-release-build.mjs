#!/usr/bin/env node
/**
 * Adjust tauri.conf.json for release builds when signing keys are absent.
 * Keeps installer bundles; skips updater .sig artifacts (requires private key).
 */
import fs from "node:fs";
import path from "node:path";

const configPath = path.join(process.cwd(), "src-tauri", "tauri.conf.json");
const hasPrivateKey = Boolean(process.env.TAURI_SIGNING_PRIVATE_KEY?.trim());

const config = JSON.parse(fs.readFileSync(configPath, "utf8"));
config.bundle ??= {};

if (hasPrivateKey) {
  config.bundle.createUpdaterArtifacts = true;
  console.log("TAURI_SIGNING_PRIVATE_KEY is set — updater artifacts enabled.");
} else {
  config.bundle.createUpdaterArtifacts = false;
  console.log(
    "TAURI_SIGNING_PRIVATE_KEY is not set — building installers only (no updater .sig).",
  );
}

fs.writeFileSync(configPath, `${JSON.stringify(config, null, 2)}\n`, "utf8");
