import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export function packageRoot() {
  return path.resolve(__dirname, "..");
}

export function readPackageVersion() {
  const pkgPath = path.join(packageRoot(), "package.json");
  const pkg = JSON.parse(fs.readFileSync(pkgPath, "utf8"));
  return String(pkg.version);
}

export function nativeBinaryPath(binaryName) {
  return path.join(packageRoot(), "vendor", binaryName);
}

export function vendorDir() {
  return path.join(packageRoot(), "vendor");
}

export function downloadCachePath(fileName) {
  return path.join(vendorDir(), fileName);
}
