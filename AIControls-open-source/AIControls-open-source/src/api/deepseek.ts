import { invoke } from "@tauri-apps/api/core";
import type { AgentInventory } from "./agents";

export type DeepseekSettings = {
  apiKeyConfigured: boolean;
};

export async function getDeepseekSettings(): Promise<DeepseekSettings | null> {
  try {
    return await invoke<DeepseekSettings>("get_deepseek_settings");
  } catch {
    return null;
  }
}

export async function saveDeepseekSettings(apiKey: string): Promise<boolean> {
  try {
    await invoke("save_deepseek_settings", { apiKey });
    return true;
  } catch {
    return false;
  }
}

export async function testDeepseekConnection(): Promise<{
  ok: boolean;
  message: string;
}> {
  try {
    const msg = await invoke<string>("test_deepseek_connection");
    return { ok: true, message: msg };
  } catch (e) {
    return { ok: false, message: e instanceof Error ? e.message : String(e) };
  }
}

/** 对已加载库存中尚未写入本地缓存的条目调用 DeepSeek 分类；密钥未配置时后端直接返回原库存。 */
export async function deepseekClassifyInventory(
  inventory: AgentInventory,
): Promise<AgentInventory | null> {
  try {
    return await invoke<AgentInventory>("deepseek_classify_inventory", {
      inventory,
    });
  } catch {
    return null;
  }
}

/** 对已加载库存中尚未写入本地缓存的条目调用 DeepSeek 生成中文缩略介绍。 */
export async function deepseekSummarizeInventory(
  inventory: AgentInventory,
  locale: "zh" | "en" = "zh",
): Promise<AgentInventory | null> {
  try {
    return await invoke<AgentInventory>("deepseek_summarize_inventory", {
      inventory,
      locale,
    });
  } catch {
    return null;
  }
}

export async function deepseekResummarizeAsset(
  asset: {
    id: string;
    kind: string;
    title: string;
    description: string;
    path: string;
    active: boolean;
  },
  locale: "zh" | "en" = "zh",
): Promise<string | null> {
  try {
    return await invoke<string>("deepseek_resummarize_asset", { asset, locale });
  } catch {
    return null;
  }
}

export type ResourceUrlEnrichment = {
  title: string;
  tags: string[];
  note: string;
};

/** 根据链接由 DeepSeek 生成标题、标签与用途备注（需已在设置中配置 API Key）。 */
export async function deepseekEnrichResourceUrl(url: string): Promise<ResourceUrlEnrichment> {
  return invoke<ResourceUrlEnrichment>("deepseek_enrich_resource_url", { url });
}

/** AI 生成的自定义分类 */
export type CustomCategory = {
  slug: string;
  labelZh: string;
  labelEn?: string | null;
};

/** 让 AI 分析所有资产，生成一组新的中英双语分类方案。 */
export async function deepseekRegenerateCategories(
  inventory: AgentInventory,
  locale: "zh" | "en" = "zh",
): Promise<CustomCategory[] | null> {
  try {
    return await invoke<CustomCategory[]>("deepseek_regenerate_categories", {
      inventory,
      locale,
    });
  } catch {
    return null;
  }
}

/** 为旧的自定义分类补齐当前语言标签（英文模式会补 labelEn）。 */
export async function deepseekTranslateCustomCategories(
  categories: CustomCategory[],
  locale: "zh" | "en",
): Promise<CustomCategory[] | null> {
  try {
    return await invoke<CustomCategory[]>("deepseek_translate_custom_categories", {
      categories,
      locale,
    });
  } catch {
    return null;
  }
}

/** 使用自定义分类列表重新归类所有资产，返回 id → newSlug 映射。 */
export async function deepseekReclassifyWithCategories(
  inventory: AgentInventory,
  categories: CustomCategory[],
): Promise<Record<string, string> | null> {
  try {
    return await invoke<Record<string, string>>(
      "deepseek_reclassify_with_new_categories",
      { inventory, categories },
    );
  } catch {
    return null;
  }
}

/** 从本地持久化加载自定义分类列表。 */
export async function getCustomCategories(): Promise<CustomCategory[]> {
  try {
    return await invoke<CustomCategory[]>("get_custom_categories");
  } catch {
    return [];
  }
}

/** 清除本地持久化的自定义分类，恢复默认分类。 */
export async function clearCustomCategories(): Promise<boolean> {
  try {
    await invoke("clear_custom_categories");
    return true;
  } catch {
    return false;
  }
}

/** 重置所有分类：清除自定义分类 + 清除 AI 分类缓存，恢复到最初默认分类。 */
export async function resetAllCategories(): Promise<boolean> {
  try {
    await invoke("reset_all_categories");
    return true;
  } catch {
    return false;
  }
}
