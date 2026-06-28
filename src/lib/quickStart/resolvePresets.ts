import { providerPresets } from "@/config/claudeProviderPresets";
import { claudeDesktopProviderPresets } from "@/config/claudeDesktopProviderPresets";
import { codexProviderPresets } from "@/config/codexProviderPresets";
import { geminiProviderPresets } from "@/config/geminiProviderPresets";
import {
  QUICKSTART_CURATED,
  QUICKSTART_CUSTOM_PRESET_ID,
  type QuickStartAppId,
  type QuickStartCategoryId,
} from "@/config/quickStartCurated";
import type { ResolvedQuickStartPreset } from "./types";

function findClaudePreset(name: string) {
  return providerPresets.find((p) => p.name === name);
}

function findDesktopPreset(name: string) {
  return claudeDesktopProviderPresets.find((p) => p.name === name);
}

function findCodexPreset(name: string) {
  return codexProviderPresets.find((p) => p.name === name);
}

function findGeminiPreset(name: string) {
  return geminiProviderPresets.find((p) => p.name === name);
}

export function resolvePresetByName(
  appId: QuickStartAppId,
  name: string,
): ResolvedQuickStartPreset | null {
  switch (appId) {
    case "claude": {
      const p = findClaudePreset(name);
      if (!p) return null;
      return {
        name: p.name,
        nameKey: p.nameKey,
        websiteUrl: p.websiteUrl,
        apiKeyUrl: p.apiKeyUrl,
        category: p.category,
        icon: p.icon,
        iconColor: p.iconColor,
        isOfficial: p.isOfficial,
        isPartner: p.isPartner,
        raw: p,
      };
    }
    case "claude-desktop": {
      const p = findDesktopPreset(name);
      if (!p) return null;
      return {
        name: p.name,
        nameKey: p.nameKey,
        websiteUrl: p.websiteUrl,
        apiKeyUrl: p.apiKeyUrl,
        category: p.category,
        icon: p.icon,
        iconColor: p.iconColor,
        isOfficial: p.category === "official",
        isPartner: p.isPartner,
        raw: p,
      };
    }
    case "codex": {
      const p = findCodexPreset(name);
      if (!p) return null;
      return {
        name: p.name,
        nameKey: p.nameKey,
        websiteUrl: p.websiteUrl,
        apiKeyUrl: p.apiKeyUrl,
        category: p.category,
        icon: p.icon,
        iconColor: p.iconColor,
        isOfficial: p.isOfficial,
        isPartner: p.isPartner,
        raw: p,
      };
    }
    case "gemini": {
      const p = findGeminiPreset(name);
      if (!p) return null;
      return {
        name: p.name,
        nameKey: p.nameKey,
        websiteUrl: p.websiteUrl,
        apiKeyUrl: p.apiKeyUrl,
        category: p.category,
        icon: p.icon,
        iconColor: p.iconColor,
        isOfficial: p.category === "official",
        isPartner: p.isPartner,
        raw: p,
      };
    }
    default:
      return null;
  }
}

export interface QuickStartPresetGroup {
  category: QuickStartCategoryId;
  presets: ResolvedQuickStartPreset[];
  emptyHintKey?: string;
  isCustomGroup?: boolean;
}

/** 自定义配置卡片（虚拟 preset） */
export function customPresetCard(
  appId: QuickStartAppId,
): ResolvedQuickStartPreset {
  return {
    name: QUICKSTART_CUSTOM_PRESET_ID,
    websiteUrl: "",
    category: "custom",
    icon:
      appId === "codex"
        ? "openai"
        : appId === "gemini"
          ? "gemini"
          : "claude",
    raw: null,
  };
}

export function getCuratedPresetGroups(
  appId: QuickStartAppId,
  searchQuery: string,
): QuickStartPresetGroup[] {
  const q = searchQuery.trim().toLowerCase();
  const specs = QUICKSTART_CURATED[appId];
  const groups: QuickStartPresetGroup[] = [];

  for (const spec of specs) {
    if (spec.category === "custom") {
      groups.push({
        category: "custom",
        presets: [customPresetCard(appId)],
        isCustomGroup: true,
      });
      continue;
    }

    const presets = spec.presetNames
      .map((name) => resolvePresetByName(appId, name))
      .filter((p): p is ResolvedQuickStartPreset => p !== null)
      .filter((p) => {
        if (!q) return true;
        const hay = `${p.name} ${p.websiteUrl}`.toLowerCase();
        return hay.includes(q);
      });

    if (presets.length === 0 && !spec.emptyHintKey) {
      continue;
    }

    groups.push({
      category: spec.category,
      presets,
      emptyHintKey: spec.emptyHintKey,
    });
  }

  return groups;
}

/** CI / 单测：校验 curated 名称在对应 preset 库中存在 */
export function validateCuratedPresetNames(): string[] {
  const errors: string[] = [];
  for (const appId of Object.keys(QUICKSTART_CURATED) as QuickStartAppId[]) {
    for (const spec of QUICKSTART_CURATED[appId]) {
      for (const name of spec.presetNames) {
        if (!resolvePresetByName(appId, name)) {
          errors.push(`${appId}: missing preset "${name}"`);
        }
      }
    }
  }
  return errors;
}
