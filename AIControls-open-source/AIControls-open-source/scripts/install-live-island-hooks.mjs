#!/usr/bin/env node

import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const PROJECT_BRIDGE = join(__dirname, "live-island-bridge.mjs");
const APP_DATA_DIR = join(
  homedir(),
  "Library/Application Support/com.aicontrols.desktop",
);
const DEST_BRIDGE = join(APP_DATA_DIR, "live-island-bridge.mjs");
const SETTINGS_PATH = join(homedir(), ".claude/settings.json");
const REQUIRED_EVENTS = [
  "UserPromptSubmit",
  "PreToolUse",
  "PostToolUse",
  "PostToolUseFailure",
  "PermissionRequest",
  "Stop",
  "StopFailure",
];
const HOOK_COMMAND = `node "${DEST_BRIDGE.replace(/\\/g, "/")}" hook`;

function hasManagedHook(entry) {
  return (entry.hooks ?? []).some((hook) =>
    String(hook.command ?? "").includes("live-island-bridge.mjs"),
  );
}

function upsertHooks(settings) {
  if (!settings.hooks || typeof settings.hooks !== "object") {
    settings.hooks = {};
  }

  for (const event of REQUIRED_EVENTS) {
    if (!Array.isArray(settings.hooks[event])) {
      settings.hooks[event] = [];
    }
    const entries = settings.hooks[event];
    let found = false;
    for (const entry of entries) {
      if (!hasManagedHook(entry)) continue;
      found = true;
      for (const hook of entry.hooks ?? []) {
        if (String(hook.command ?? "").includes("live-island-bridge.mjs")) {
          hook.type = "command";
          hook.command = HOOK_COMMAND;
        }
      }
    }
    if (!found) {
      entries.push({
        matcher: "",
        hooks: [{ type: "command", command: HOOK_COMMAND }],
      });
    }
  }
}

mkdirSync(APP_DATA_DIR, { recursive: true });
copyFileSync(PROJECT_BRIDGE, DEST_BRIDGE);

const settings = existsSync(SETTINGS_PATH)
  ? JSON.parse(readFileSync(SETTINGS_PATH, "utf8"))
  : {};
upsertHooks(settings);
mkdirSync(dirname(SETTINGS_PATH), { recursive: true });
writeFileSync(SETTINGS_PATH, `${JSON.stringify(settings, null, 2)}\n`);

console.log(`[float-ball] bridge → ${DEST_BRIDGE}`);
console.log(`[float-ball] hooks updated in ${SETTINGS_PATH}`);
for (const event of REQUIRED_EVENTS) {
  console.log(`  - ${event}`);
}
