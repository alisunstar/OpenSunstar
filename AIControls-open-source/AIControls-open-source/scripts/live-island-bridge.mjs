#!/usr/bin/env node

import { execSync } from "node:child_process";
import { connect } from "node:net";
import { existsSync } from "node:fs";
import { homedir } from "node:os";
import { basename, dirname } from "node:path";

const HOST = "127.0.0.1";
const PORT = 38971;
const AGENT_META_DIR_NAMES = new Set([
  ".claude",
  ".cursor",
  ".codex",
  ".hermes",
  ".openclaw",
  ".trae",
  ".qoder",
  ".qoderwork",
  ".kiro",
  ".config",
]);
const HOOK_EVENT_ALIASES = new Map([
  ["userpromptsubmit", "UserPromptSubmit"],
  ["pretooluse", "PreToolUse"],
  ["posttooluse", "PostToolUse"],
  ["posttoolusefailure", "PostToolUseFailure"],
  ["permissionrequest", "PermissionRequest"],
  ["stop", "Stop"],
  ["stopfailure", "StopFailure"],
]);

function normalizePath(input) {
  return String(input ?? "")
    .trim()
    .replace(/\\/g, "/")
    .replace(/\/+$/, "");
}

function isAgentMetaDirName(name) {
  return AGENT_META_DIR_NAMES.has(String(name ?? "").trim());
}

function resolveProjectRootFromPath(inputPath) {
  const normalized = normalizePath(inputPath);
  if (!normalized) return null;

  const segments = normalized.split("/").filter(Boolean);
  while (segments.length > 0) {
    const leaf = segments[segments.length - 1];
    if (!isAgentMetaDirName(leaf)) {
      return segments.length === 0 ? "/" : `/${segments.join("/")}`;
    }
    segments.pop();
  }

  return null;
}

function isHomeDirectory(path) {
  const root = normalizePath(path);
  return Boolean(root && root === normalizePath(homedir()));
}

function isUsableProjectRoot(path) {
  const root = normalizePath(path);
  if (!root) return false;
  if (root === "/" || isHomeDirectory(root)) return false;
  if (isAgentMetaDirName(basename(root))) return false;
  return existsSync(root);
}

function workspaceFromEnv() {
  for (const key of [
    "CURSOR_WORKSPACE",
    "VSCODE_CWD",
    "WORKSPACE_FOLDER_PATHS",
    "CLAUDE_PROJECT_DIR",
  ]) {
    const raw = process.env[key];
    if (!raw) continue;
    const candidates = String(raw)
      .split(",")
      .map((part) => normalizePath(part))
      .filter(Boolean);
    for (const candidate of candidates) {
      const root = resolveProjectRootFromPath(candidate);
      if (root && isUsableProjectRoot(root)) return root;
    }
  }
  return null;
}

function workspaceFromTranscriptPath(transcriptPath) {
  const normalized = normalizePath(transcriptPath);
  for (const marker of ["/.claude/projects/", "/.cursor/projects/"]) {
    const idx = normalized.indexOf(marker);
    if (idx < 0) continue;

    const encoded = normalized.slice(idx + marker.length).split("/")[0];
    if (!encoded || !encoded.startsWith("-")) continue;

    const decoded = `/${encoded.slice(1).replace(/-/g, "/")}`;
    const root = resolveProjectRootFromPath(decoded) ?? decoded;
    if (root && isUsableProjectRoot(root)) return root;
  }
  return null;
}

function workspaceFromToolInput(toolInput) {
  if (!toolInput || typeof toolInput !== "object") return null;
  const filePath =
    toolInput.file_path || toolInput.filePath || toolInput.path || toolInput.notebook_path;
  if (!filePath) return null;
  const root = resolveProjectRootFromPath(dirname(normalizePath(String(filePath))));
  return root && isUsableProjectRoot(root) ? root : null;
}

function resolveHookWorkspace(payload) {
  const payloadCandidates = [
    payload.workspace_path,
    payload.workspacePath,
    payload.workspace_root,
    payload.workspaceRoot,
    payload.project_path,
    payload.projectPath,
    payload.project_root,
    payload.projectRoot,
    payload.workspace_folder,
    payload.workspaceFolder,
  ]
    .map(normalizePath)
    .filter(Boolean);

  for (const candidate of payloadCandidates) {
    const root = resolveProjectRootFromPath(candidate);
    if (root && isUsableProjectRoot(root)) return root;
  }

  const envRoot = workspaceFromEnv();
  if (envRoot) return envRoot;

  const cwdRoot = resolveProjectRootFromPath(payload.cwd || process.cwd());
  if (cwdRoot && isUsableProjectRoot(cwdRoot)) return cwdRoot;

  const transcriptRoot = workspaceFromTranscriptPath(payload.transcript_path);
  if (transcriptRoot) return transcriptRoot;

  const toolRoot = workspaceFromToolInput(payload.tool_input);
  if (toolRoot) return toolRoot;

  return null;
}

function normalizeHookEvent(eventName) {
  const key = String(eventName ?? "").trim().toLowerCase();
  return HOOK_EVENT_ALIASES.get(key) || String(eventName ?? "").trim();
}

function readStdin() {
  return new Promise((resolve, reject) => {
    let buffer = "";
    process.stdin.setEncoding("utf8");
    process.stdin.on("data", (chunk) => {
      buffer += chunk;
    });
    process.stdin.on("end", () => resolve(buffer));
    process.stdin.on("error", reject);
  });
}

function parsePayload(raw) {
  const trimmed = String(raw ?? "").trim();
  if (!trimmed) return {};
  try {
    return JSON.parse(trimmed);
  } catch {
    return {};
  }
}

function sendMessage(message) {
  return new Promise((resolve, reject) => {
    const socket = connect(PORT, HOST, () => {
      socket.write(`${JSON.stringify(message)}\n`);
      socket.end();
    });
    socket.on("close", () => resolve());
    socket.on("error", reject);
  });
}

function fallbackGitRoot(cwd) {
  const normalized = normalizePath(cwd);
  if (!normalized) return null;
  try {
    const out = execSync("git rev-parse --show-toplevel", {
      cwd: normalized,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
    const root = normalizePath(out);
    return root && isUsableProjectRoot(root) ? root : null;
  } catch {
    return null;
  }
}

function mapHookEventToState(eventName) {
  switch (eventName) {
    case "UserPromptSubmit":
      return "running";
    case "PreToolUse":
      return "tool";
    case "PostToolUse":
    case "PostToolUseFailure":
      return "running";
    case "PermissionRequest":
      return "waiting";
    case "Stop":
      return "complete";
    case "StopFailure":
      return "error";
    default:
      return null;
  }
}

function toolNameFromPayload(payload) {
  const candidate = payload.tool_name || payload.toolName || payload.tool;
  return typeof candidate === "string" && candidate.trim() ? candidate.trim() : null;
}

async function handleHook(payload) {
  const eventName = normalizeHookEvent(payload.hook_event_name || payload.event || payload.hookEventName);
  const state = mapHookEventToState(eventName);
  if (!state) return;

  const cwd =
    resolveHookWorkspace(payload) ||
    fallbackGitRoot(payload.cwd || process.cwd()) ||
    normalizePath(payload.cwd || process.cwd());
  if (!cwd) return;

  await sendMessage({
    type: "claude-state",
    cwd,
    sessionId: payload.session_id || payload.sessionId || null,
    state,
    toolName: toolNameFromPayload(payload),
  });
}

async function main() {
  if (process.argv[2] !== "hook") return;
  const raw = await readStdin();
  const payload = parsePayload(raw);
  try {
    await handleHook(payload);
  } catch (error) {
    console.error(`[live-island-bridge] failed to send hook state: ${error instanceof Error ? error.message : String(error)}`);
    process.exitCode = 0;
  }
}

void main();
