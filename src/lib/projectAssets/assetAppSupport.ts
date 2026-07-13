import type { AppId } from "@/lib/api";
import type { ProjectAssetType } from "@/types/projectAsset";
import supportContract from "./assetAppSupport.contract.json";

export type AssetAppSupportStatus = "supported" | "partial" | "unsupported";

export interface AssetAppSupport {
  status: AssetAppSupportStatus;
  reasonKey?: string;
  reasonDefault?: string;
}

export type AssetAppSupportMatrix = Record<
  ProjectAssetType,
  Record<AppId, AssetAppSupport>
>;

/**
 * 8 类资产 × 7 应用同步能力矩阵（与开发文档 4.0 一致，以当前源码能力为准）
 * 项目级开关据此置灰 / 展示 partial 提示，避免“点了才报错”。
 */
const LEGACY_ASSET_APP_SUPPORT: AssetAppSupportMatrix = {
  mcp: {
    claude: { status: "supported" },
    "claude-desktop": {
      status: "unsupported",
      reasonKey: "projectAssets.support.claudeDesktop",
      reasonDefault: "Claude Desktop 暂不参与统一 MCP 同步",
    },
    codex: { status: "supported" },
    gemini: { status: "supported" },
    opencode: { status: "supported" },
    openclaw: {
      status: "unsupported",
      reasonKey: "projectAssets.support.openclawMcp",
      reasonDefault: "OpenClaw 当前不支持 MCP 同步",
    },
    hermes: { status: "supported" },
  },
  skill: {
    claude: { status: "supported" },
    "claude-desktop": {
      status: "unsupported",
      reasonKey: "projectAssets.support.claudeDesktop",
      reasonDefault: "Claude Desktop 暂不参与 Skills 同步",
    },
    codex: { status: "supported" },
    gemini: { status: "supported" },
    opencode: { status: "supported" },
    openclaw: {
      status: "unsupported",
      reasonKey: "projectAssets.support.openclawSkills",
      reasonDefault: "OpenClaw 当前不支持 Skills 同步",
    },
    hermes: { status: "supported" },
  },
  prompt: {
    claude: { status: "supported" },
    "claude-desktop": {
      status: "unsupported",
      reasonKey: "projectAssets.support.claudeDesktop",
      reasonDefault: "Claude Desktop 暂不参与 Prompts 同步",
    },
    codex: { status: "supported" },
    gemini: { status: "supported" },
    opencode: { status: "supported" },
    openclaw: { status: "supported" },
    hermes: { status: "supported" },
  },
  command: {
    claude: { status: "supported" },
    "claude-desktop": {
      status: "unsupported",
      reasonKey: "projectAssets.support.claudeDesktop",
      reasonDefault: "Claude Desktop 暂不支持 Commands",
    },
    codex: { status: "supported" },
    gemini: { status: "supported" },
    opencode: { status: "supported" },
    openclaw: {
      status: "unsupported",
      reasonKey: "projectAssets.support.openclawCommands",
      reasonDefault: "OpenClaw 当前不支持 Commands 同步",
    },
    hermes: { status: "supported" },
  },
  hook: {
    claude: { status: "supported" },
    "claude-desktop": {
      status: "unsupported",
      reasonKey: "projectAssets.support.hooksClaudeOnly",
      reasonDefault: "Claude Desktop 暂不支持 Hooks 同步",
    },
    codex: { status: "supported" },
    gemini: { status: "supported" },
    opencode: {
      status: "unsupported",
      reasonKey: "projectAssets.support.hooksOpencode",
      reasonDefault: "OpenCode 需 TypeScript 插件，暂不支持 Hooks 同步",
    },
    openclaw: {
      status: "unsupported",
      reasonKey: "projectAssets.support.hooksOpenclaw",
      reasonDefault: "OpenClaw 暂不支持 Hooks 同步",
    },
    hermes: { status: "supported" },
  },
  ignore: {
    claude: { status: "supported" },
    "claude-desktop": {
      status: "unsupported",
      reasonKey: "projectAssets.support.claudeDesktop",
      reasonDefault: "Claude Desktop 暂不参与 Ignore 同步",
    },
    codex: { status: "supported" },
    gemini: { status: "supported" },
    opencode: { status: "supported" },
    openclaw: {
      status: "unsupported",
      reasonKey: "projectAssets.support.openclawIgnore",
      reasonDefault: "OpenClaw 当前不支持 Ignore 同步",
    },
    hermes: { status: "supported" },
  },
  permission: {
    claude: { status: "supported" },
    "claude-desktop": {
      status: "unsupported",
      reasonKey: "projectAssets.support.permissionsClaudeOnly",
      reasonDefault: "Claude Desktop 暂不支持 Permissions 同步",
    },
    codex: { status: "supported" },
    gemini: { status: "supported" },
    opencode: { status: "supported" },
    openclaw: { status: "supported" },
    hermes: {
      status: "partial",
      reasonKey: "projectAssets.support.hermesPermissions",
      reasonDefault: "Hermes Permissions 为最佳努力写入 config.yaml",
    },
  },
  subagent: {
    claude: { status: "supported" },
    "claude-desktop": {
      status: "unsupported",
      reasonKey: "projectAssets.support.claudeDesktop",
      reasonDefault: "Claude Desktop 暂不支持 Subagents",
    },
    codex: {
      status: "partial",
      reasonKey: "projectAssets.support.codexSubagent",
      reasonDefault: "Codex 通过 TOML 转换写入 ~/.codex/agents/",
    },
    gemini: { status: "supported" },
    opencode: { status: "supported" },
    openclaw: {
      status: "unsupported",
      reasonKey: "projectAssets.support.openclawSubagents",
      reasonDefault: "OpenClaw 当前不支持 Subagents 同步",
    },
    hermes: {
      status: "unsupported",
      reasonKey: "projectAssets.support.hermesSubagents",
      reasonDefault: "Hermes 不支持 Subagent 文件同步",
    },
  },
};

/** Single source of truth for support status, shared with the Rust backend. */
export const ASSET_APP_SUPPORT: AssetAppSupportMatrix = Object.fromEntries(
  Object.entries(LEGACY_ASSET_APP_SUPPORT).map(([assetType, legacyApps]) => [
    assetType,
    Object.fromEntries(
      Object.entries(legacyApps).map(([appId, legacySupport]) => [
        appId,
        {
          ...legacySupport,
          status: supportContract[assetType as ProjectAssetType][appId as AppId] as AssetAppSupportStatus,
        },
      ]),
    ),
  ]),
) as AssetAppSupportMatrix;

/** Prompt 同步支持的应用（与 ASSET_APP_SUPPORT.prompt 一致） */
export const PROMPT_SYNC_APP_IDS: AppId[] = (
  Object.entries(ASSET_APP_SUPPORT.prompt) as [AppId, AssetAppSupport][]
)
  .filter(([, support]) => support.status === "supported")
  .map(([appId]) => appId);

/** 资产类型是否至少在一个目标应用上可启用 */
export function isAssetLinkable(assetType: ProjectAssetType): boolean {
  return Object.values(ASSET_APP_SUPPORT[assetType]).some(
    (s) => s.status !== "unsupported",
  );
}

export function getAssetAppSupport(
  assetType: ProjectAssetType,
  appId: AppId,
): AssetAppSupport {
  return ASSET_APP_SUPPORT[assetType][appId];
}

/** 汇总该资产类型的支持摘要（用于 section 标题下 helper） */
export function summarizeAssetSupport(
  assetType: ProjectAssetType,
): { hasSupported: boolean; hasPartial: boolean; allUnsupported: boolean } {
  const entries = Object.values(ASSET_APP_SUPPORT[assetType]);
  const hasSupported = entries.some((e) => e.status === "supported");
  const hasPartial = entries.some((e) => e.status === "partial");
  const allUnsupported = entries.every((e) => e.status === "unsupported");
  return { hasSupported, hasPartial, allUnsupported };
}
