// 打包 rd-protocol 为可发布 skill 包（B2）：distrib/rd-protocol-<version>.zip
// 用法: node scripts/package-rd-protocol.mjs
// 说明: zip 根目录即 SKILL.md（install_from_zip 支持 ZIP 根含 SKILL.md 的布局，
//       安装名回退为 zip 文件名干）。用户经 Skills 管理界面「从 ZIP 安装」接入，
//       再经 skills SSOT 同步到各目标 CLI。Windows 下用 PowerShell Compress-Archive。
import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(__dirname, "..");
const pkg = JSON.parse(fs.readFileSync(path.join(root, "package.json"), "utf8"));
const srcDir = path.join(root, "rd-protocol");
const outDir = path.join(root, "distrib");
fs.mkdirSync(outDir, { recursive: true });
const outFile = path.join(outDir, `rd-protocol-${pkg.version}.zip`);
if (fs.existsSync(outFile)) fs.rmSync(outFile);

if (!fs.existsSync(path.join(srcDir, "SKILL.md"))) {
  console.error("rd-protocol/SKILL.md 不存在，无法打包");
  process.exit(1);
}

// Compress-Archive -Path rd-protocol\* => SKILL.md 位于 zip 根
execFileSync(
  "powershell.exe",
  [
    "-NoProfile",
    "-Command",
    `Compress-Archive -Path '${srcDir}\\*' -DestinationPath '${outFile}'`,
  ],
  { stdio: "inherit" }
);

console.log("OK ->", path.relative(root, outFile), fs.statSync(outFile).size, "bytes");
