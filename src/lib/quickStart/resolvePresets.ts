import { providerPresets } from "@/config/claudeProviderPresets";
import { claudeDesktopProviderPresets } from "@/config/claudeDesktopProviderPresets";
import { codexProviderPresets } from "@/config/codexProviderPresets";
import { geminiProviderPresets } from "@/config/geminiProviderPresets";
import { opencodeProviderPresets } from "@/config/opencodeProviderPresets";
import { openclawProviderPresets } from "@/config/openclawProviderPresets";
import { hermesProviderPresets } from "@/config/hermesProviderPresets";
import {
  grokBuildProviderPresets,
  grokBuildOfficialPreset,
  type GrokBuildProviderPreset,
} from "@/config/grokBuildProviderPresets";
import {
  quickStartClaudePresets,
  quickStartCodexPresets,
  quickStartDesktopPresets,
  quickStartGeminiPresets,
} from "@/config/quickStartProviderPresets";
import {
  QUICKSTART_CURATED,
  QUICKSTART_CUSTOM_PRESET_ID,
  type QuickStartAppId,
  type QuickStartCategoryId,
} from "@/config/quickStartCurated";
import type { ResolvedQuickStartPreset } from "./types";

function findClaudePreset(name: string) {
  return [...providerPresets, ...quickStartClaudePresets].find(
    (p) => p.name === name,
  );
}

function findDesktopPreset(name: string) {
  return [...claudeDesktopProviderPresets, ...quickStartDesktopPresets].find(
    (p) => p.name === name,
  );
}

function findCodexPreset(name: string) {
  return [...codexProviderPresets, ...quickStartCodexPresets].find(
    (p) => p.name === name,
  );
}

function findGeminiPreset(name: string) {
  return [...geminiProviderPresets, ...quickStartGeminiPresets].find(
    (p) => p.name === name,
  );
}

function findOpenCodePreset(name: string) {
  return opencodeProviderPresets.find((preset) => preset.name === name);
}

function findOpenClawPreset(name: string) {
  return openclawProviderPresets.find((preset) => preset.name === name);
}

function findHermesPreset(name: string) {
  return hermesProviderPresets.find((preset) => preset.name === name);
}

function findGrokBuildPreset(
  name: string,
): GrokBuildProviderPreset | undefined {
  return [grokBuildOfficialPreset, ...grokBuildProviderPresets].find(
    (preset) => preset.name === name,
  );
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
        authMode: p.requiresOAuth || p.isOfficial ? "oauth" : "api_key",
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
        authMode:
          p.requiresOAuth || p.category === "official" ? "oauth" : "api_key",
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
        authMode: p.isOfficial ? "oauth" : "api_key",
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
        authMode: p.category === "official" ? "oauth" : "api_key",
        raw: p,
      };
    }
    case "grokbuild": {
      const p = findGrokBuildPreset(name);
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
        authMode: p.isOfficial ? "oauth" : "api_key",
        raw: p,
      };
    }
    case "opencode": {
      const p = findOpenCodePreset(name);
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
        authMode: "api_key",
        raw: p,
      };
    }
    case "openclaw": {
      const p = findOpenClawPreset(name);
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
        authMode: "api_key",
        raw: p,
      };
    }
    case "hermes": {
      const p = findHermesPreset(name);
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
        authMode: "api_key",
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
  isCustomGroup?: boolean;
}

function unavailablePreset(
  displayName: string,
  unavailableReason?: string,
  unavailableReasonKey?: string,
): ResolvedQuickStartPreset {
  return {
    name: `__unavailable__:${displayName}`,
    displayName,
    websiteUrl: "",
    unavailable: true,
    unavailableReason,
    unavailableReasonKey,
    raw: null,
  };
}

/** 自定义配置卡片（虚拟 preset） */
export function customPresetCard(
  appId: QuickStartAppId,
): ResolvedQuickStartPreset {
  return {
    name: QUICKSTART_CUSTOM_PRESET_ID,
    websiteUrl: "",
    category: "custom",
    icon: appId === "codex" ? "openai" : appId === "gemini" ? "gemini" : appId,
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

    const presets = spec.presets
      .map((presetRef) => {
        if (presetRef.unavailable) {
          return unavailablePreset(
            presetRef.displayName,
            presetRef.unavailableReason,
            presetRef.unavailableReasonKey,
          );
        }
        const preset = resolvePresetByName(appId, presetRef.presetName ?? "");
        return preset
          ? { ...preset, displayName: presetRef.displayName }
          : null;
      })
      .filter((p): p is ResolvedQuickStartPreset => p !== null)
      .filter((p) => {
        if (!q) return true;
        const hay =
          `${p.displayName ?? ""} ${p.name} ${p.websiteUrl}`.toLowerCase();
        return hay.includes(q);
      });

    if (presets.length === 0) {
      continue;
    }

    groups.push({
      category: spec.category,
      presets,
    });
  }

  return groups;
}

/** CI / 单测：校验 curated 名称在对应 preset 库中存在 */
export function validateCuratedPresetNames(): string[] {
  const errors: string[] = [];
  for (const appId of Object.keys(QUICKSTART_CURATED) as QuickStartAppId[]) {
    for (const spec of QUICKSTART_CURATED[appId]) {
      for (const presetRef of spec.presets) {
        if (
          !presetRef.unavailable &&
          !resolvePresetByName(appId, presetRef.presetName ?? "")
        ) {
          errors.push(
            `${appId}: missing preset "${presetRef.presetName ?? presetRef.displayName}"`,
          );
        }
      }
    }
  }
  return errors;
}
