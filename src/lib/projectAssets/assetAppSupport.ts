import type { AppId } from "@/lib/api";
import type { ProjectAssetType } from "@/types/projectAsset";

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
export const ASSET_APP_SUPPORT: AssetAppSupportMatrix = {
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
    codex: {
      status: "unsupported",
      reasonKey: "projectAssets.support.codexCommands",
      reasonDefault: "Codex 不支持独立 slash command 文件",
    },
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
      reasonDefault: "Hooks 当前仅 Claude Code 支持写回",
    },
    codex: {
      status: "unsupported",
      reasonKey: "projectAssets.support.hooksClaudeOnly",
      reasonDefault: "Hooks 当前仅 Claude Code 支持写回",
    },
    gemini: {
      status: "unsupported",
      reasonKey: "projectAssets.support.hooksClaudeOnly",
      reasonDefault: "Hooks 当前仅 Claude Code 支持写回",
    },
    opencode: {
      status: "unsupported",
      reasonKey: "projectAssets.support.hooksClaudeOnly",
      reasonDefault: "Hooks 当前仅 Claude Code 支持写回",
    },
    openclaw: {
      status: "unsupported",
      reasonKey: "projectAssets.support.hooksClaudeOnly",
      reasonDefault: "Hooks 当前仅 Claude Code 支持写回",
    },
    hermes: {
      status: "unsupported",
      reasonKey: "projectAssets.support.hooksClaudeOnly",
      reasonDefault: "Hooks 当前仅 Claude Code 支持写回",
    },
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
      reasonDefault: "Permissions 当前仅 Claude Code 支持写回",
    },
    codex: {
      status: "unsupported",
      reasonKey: "projectAssets.support.permissionsClaudeOnly",
      reasonDefault: "Permissions 当前仅 Claude Code 支持写回",
    },
    gemini: {
      status: "unsupported",
      reasonKey: "projectAssets.support.permissionsClaudeOnly",
      reasonDefault: "Permissions 当前仅 Claude Code 支持写回",
    },
    opencode: {
      status: "unsupported",
      reasonKey: "projectAssets.support.permissionsClaudeOnly",
      reasonDefault: "Permissions 当前仅 Claude Code 支持写回",
    },
    openclaw: {
      status: "unsupported",
      reasonKey: "projectAssets.support.permissionsClaudeOnly",
      reasonDefault: "Permissions 当前仅 Claude Code 支持写回",
    },
    hermes: {
      status: "unsupported",
      reasonKey: "projectAssets.support.permissionsClaudeOnly",
      reasonDefault: "Permissions 当前仅 Claude Code 支持写回",
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
