import { execSync } from "node:child_process";
import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const script = path.join(root, "scripts", "sync-website-pages.sh");

const gitBashCandidates = [
  "C:\\Program Files\\Git\\bin\\bash.exe",
  "C:\\Program Files (x86)\\Git\\bin\\bash.exe",
];

let bash = "bash";
if (process.platform === "win32") {
  const gitBash = gitBashCandidates.find((candidate) => existsSync(candidate));
  if (gitBash) {
    bash = `"${gitBash}"`;
  }
}

execSync(`${bash} "${script.replace(/\\/g, "/")}"`, {
  cwd: root,
  stdio: "inherit",
  shell: true,
  env: process.env,
});
