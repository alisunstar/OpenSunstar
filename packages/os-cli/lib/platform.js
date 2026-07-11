import os from "node:os";

const GITHUB_REPO = "alisunstar/OpenSunstar";

export const SUPPORTED_PLATFORMS = [
  {
    platform: "win32",
    arch: "x64",
    artifact: "os-windows-x86_64",
    archiveExt: "zip",
    binaryName: "os.exe",
  },
  {
    platform: "linux",
    arch: "x64",
    artifact: "os-linux-x86_64",
    archiveExt: "tar.gz",
    binaryName: "os",
  },
  {
    platform: "darwin",
    arch: "arm64",
    artifact: "os-macos-aarch64",
    archiveExt: "tar.gz",
    binaryName: "os",
  },
  {
    platform: "darwin",
    arch: "x64",
    artifact: "os-macos-x86_64",
    archiveExt: "tar.gz",
    binaryName: "os",
  },
];

/**
 * Map host platform to Release artifact suffix and archive type.
 * @returns {{ artifact: string, archiveExt: "zip" | "tar.gz", binaryName: string } | null}
 */
export function resolvePlatform() {
  const platform = process.platform;
  const arch = process.arch;

  return (
    SUPPORTED_PLATFORMS.find(
      (entry) => entry.platform === platform && entry.arch === arch,
    ) ?? null
  );
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
