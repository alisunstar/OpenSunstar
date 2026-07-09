import os from "node:os";

const GITHUB_REPO = "alisunstar/OpenSunstar";

/**
 * Map host platform to Release artifact suffix and archive type.
 * @returns {{ artifact: string, archiveExt: "zip" | "tar.gz", binaryName: string } | null}
 */
export function resolvePlatform() {
  const platform = process.platform;
  const arch = process.arch;

  if (platform === "win32" && arch === "x64") {
    return {
      artifact: "os-windows-x86_64",
      archiveExt: "zip",
      binaryName: "os.exe",
    };
  }

  if (platform === "linux" && arch === "x64") {
    return {
      artifact: "os-linux-x86_64",
      archiveExt: "tar.gz",
      binaryName: "os",
    };
  }

  if (platform === "darwin" && arch === "arm64") {
    return {
      artifact: "os-macos-aarch64",
      archiveExt: "tar.gz",
      binaryName: "os",
    };
  }

  if (platform === "darwin" && arch === "x64") {
    return {
      artifact: "os-macos-x86_64",
      archiveExt: "tar.gz",
      binaryName: "os",
    };
  }

  return null;
}

export function describeHost() {
  return `${process.platform}-${process.arch} (${os.release()})`;
}

export function releaseAssetUrl(version, platformInfo) {
  const tag = version.startsWith("v") ? version : `v${version}`;
  const file = `OpenSunstar-${tag}-${platformInfo.artifact}.${platformInfo.archiveExt}`;
  return {
    file,
    url: `https://github.com/${GITHUB_REPO}/releases/download/${tag}/${file}`,
  };
}
