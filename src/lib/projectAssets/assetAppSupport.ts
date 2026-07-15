import type { AppId } from "@/lib/api";
import type { ProjectAssetType } from "@/types/projectAsset";
import supportContract from "./assetAppSupport.contract.json";

export type AssetAppSupportStatus = "supported" | "partial" | "unsupported";
export type AssetWriteMode = "project_file" | "global_path" | "none";
export type AssetVerificationMode = "config_parse" | "native_probe";

export interface AssetCapability {
  support: AssetAppSupportStatus;
  write_mode: AssetWriteMode;
  verify_modes: AssetVerificationMode[];
  limitations: string[];
}

export interface AssetAppSupport {
  status: AssetAppSupportStatus;
  reasonKey?: string;
  reasonDefault?: string;
}

export type AssetAppSupportMatrix = Record<
  ProjectAssetType,
  Record<AppId, AssetAppSupport>
>;

interface AssetCapabilityContractSource {
  schema_version: number;
  apps: AppId[];
  assets: Record<
    ProjectAssetType,
    {
      adapter_id: string;
      fixture_id: string;
      supported: AppId[];
      partial: AppId[];
      unsupported: AppId[];
      write_mode: AssetWriteMode;
      write_mode_overrides?: Partial<Record<AppId, AssetWriteMode>>;
      verify_modes: AssetVerificationMode[];
      limitations: Partial<Record<AppId, string[]>>;
    }
  >;
}

export const ASSET_CAPABILITY_CONTRACT =
  supportContract as AssetCapabilityContractSource;

export function getAssetCapability(
  assetType: ProjectAssetType,
  appId: AppId,
): AssetCapability {
  const source = ASSET_CAPABILITY_CONTRACT.assets[assetType];
  const support = source.supported.includes(appId)
    ? "supported"
    : source.partial.includes(appId)
      ? "partial"
      : "unsupported";

  return {
    support,
    write_mode:
      support === "unsupported"
        ? "none"
        : (source.write_mode_overrides?.[appId] ?? source.write_mode),
    verify_modes: support === "unsupported" ? [] : source.verify_modes,
    limitations: source.limitations[appId] ?? [],
  };
}

export function getAssetCapabilityEntries(
  assetType: ProjectAssetType,
): [AppId, AssetCapability][] {
  return ASSET_CAPABILITY_CONTRACT.apps.map((appId) => [
    appId,
    getAssetCapability(assetType, appId),
  ]);
}

/**
 * UI view derived entirely from the versioned capability contract. Reason text
 * is generated from contract limitations, so adding an app or asset never
 * requires maintaining a second 8×7 matrix in TypeScript.
 */
export const ASSET_APP_SUPPORT: AssetAppSupportMatrix = Object.fromEntries(
  Object.keys(ASSET_CAPABILITY_CONTRACT.assets).map((assetTypeValue) => {
    const assetType = assetTypeValue as ProjectAssetType;
    return [
      assetType,
      Object.fromEntries(
        ASSET_CAPABILITY_CONTRACT.apps.map((appId) => {
          const capability = getAssetCapability(assetType, appId);
          const limitation = capability.limitations[0];
          const support: AssetAppSupport = { status: capability.support };
          if (capability.support !== "supported") {
            support.reasonKey = `projectAssets.capability.${capability.support}`;
            support.reasonDefault =
              limitation === "global_side_effect"
                ? `${appId} 当前会写入全局目录，需单独确认`
                : limitation === "best_effort"
                  ? `${appId} 当前为尽力写入，需人工复核`
                  : limitation === "plugin_required"
                    ? `${appId} 需要额外插件，当前未启用同步`
                    : `${appId} 当前不支持 ${assetType} 项目级同步`;
          }
          return [appId, support];
        }),
      ),
    ];
  }),
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
export function summarizeAssetSupport(assetType: ProjectAssetType): {
  hasSupported: boolean;
  hasPartial: boolean;
  allUnsupported: boolean;
} {
  const entries = Object.values(ASSET_APP_SUPPORT[assetType]);
  const hasSupported = entries.some((e) => e.status === "supported");
  const hasPartial = entries.some((e) => e.status === "partial");
  const allUnsupported = entries.every((e) => e.status === "unsupported");
  return { hasSupported, hasPartial, allUnsupported };
}
