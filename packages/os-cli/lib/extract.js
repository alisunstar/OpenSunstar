import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

export function extractArchive(archivePath, destDir, archiveExt) {
  fs.mkdirSync(destDir, { recursive: true });

  if (archiveExt === "tar.gz") {
    execFileSync("tar", ["-xzf", archivePath, "-C", destDir], {
      stdio: "inherit",
    });
    return;
  }

  if (archiveExt === "zip") {
    // Windows 10+ and modern Unix tar both support zip extraction.
    execFileSync("tar", ["-xf", archivePath, "-C", destDir], {
      stdio: "inherit",
    });
    return;
  }

  throw new Error(`Unsupported archive type: ${archiveExt}`);
}

export function assertBinaryExists(binaryPath) {
  if (!fs.existsSync(binaryPath)) {
    throw new Error(`Binary not found after extraction: ${binaryPath}`);
  }

  if (process.platform !== "win32") {
    fs.chmodSync(binaryPath, 0o755);
  }
}

export function cleanupArchive(archivePath) {
  try {
    fs.unlinkSync(archivePath);
  } catch {
    // best effort
  }
}
