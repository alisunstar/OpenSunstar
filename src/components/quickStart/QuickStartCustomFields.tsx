import { useTranslation } from "react-i18next";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import type { QuickStartAppId } from "@/config/quickStartCurated";
import type { QuickStartFormFields } from "@/lib/quickStart/types";

interface QuickStartCustomFieldsProps {
  appId: QuickStartAppId;
  fields: QuickStartFormFields;
  onChange: (patch: Partial<QuickStartFormFields>) => void;
}

export function QuickStartCustomFields({
  appId,
  fields,
  onChange,
}: QuickStartCustomFieldsProps) {
  const { t } = useTranslation();

  const modelLabel =
    appId === "gemini"
      ? t("quickStart.custom.geminiModel", { defaultValue: "模型 ID" })
      : t("quickStart.custom.defaultModel", { defaultValue: "默认模型" });

  const handleModelChange = (customModel: string) => {
    const previousModel = fields.customModel;
    const syncIfDerived = (current: string) =>
      !current.trim() || current === previousModel ? customModel : current;
    const patch: Partial<QuickStartFormFields> = { customModel };
    if (appId === "claude" && fields.advancedClaude) {
      patch.advancedClaude = {
        ...fields.advancedClaude,
        haikuModel: syncIfDerived(fields.advancedClaude.haikuModel),
        haikuModelName: syncIfDerived(fields.advancedClaude.haikuModelName),
        sonnetModel: syncIfDerived(fields.advancedClaude.sonnetModel),
        sonnetModelName: syncIfDerived(fields.advancedClaude.sonnetModelName),
        opusModel: syncIfDerived(fields.advancedClaude.opusModel),
        opusModelName: syncIfDerived(fields.advancedClaude.opusModelName),
        fableModel: syncIfDerived(fields.advancedClaude.fableModel),
        fableModelName: syncIfDerived(fields.advancedClaude.fableModelName),
        fallbackModel: syncIfDerived(fields.advancedClaude.fallbackModel),
      };
    } else if (appId === "claude-desktop" && fields.advancedDesktop) {
      patch.advancedDesktop = {
        ...fields.advancedDesktop,
        sonnetModel: syncIfDerived(fields.advancedDesktop.sonnetModel),
        sonnetLabel: syncIfDerived(fields.advancedDesktop.sonnetLabel),
        opusModel: syncIfDerived(fields.advancedDesktop.opusModel),
        opusLabel: syncIfDerived(fields.advancedDesktop.opusLabel),
        haikuModel: syncIfDerived(fields.advancedDesktop.haikuModel),
        haikuLabel: syncIfDerived(fields.advancedDesktop.haikuLabel),
      };
    } else if (appId === "codex" && fields.advancedCodex) {
      patch.advancedCodex = {
        ...fields.advancedCodex,
        defaultModel: customModel,
      };
    } else if (appId === "gemini" && fields.advancedGemini) {
      patch.advancedGemini = { ...fields.advancedGemini, model: customModel };
    }
    onChange(patch);
  };

  return (
    <div className="space-y-3 rounded-lg border border-dashed border-border p-4">
      <p className="text-xs text-muted-foreground">
        {appId === "gemini"
          ? t("quickStart.custom.geminiHint", {
              defaultValue:
                "填写名称、Gemini Native Base URL 与模型；Gemini CLI 原生协议在高级选项中展示。",
            })
          : t("quickStart.custom.hint", {
              defaultValue:
                "填写名称、Base URL 与默认模型；API 格式、认证字段和模型映射可在下方高级选项中调整。",
            })}
      </p>
      <div className="space-y-2">
        <Label htmlFor="quickstart-custom-name">
          {t("quickStart.custom.name", { defaultValue: "供应商名称" })}
        </Label>
        <Input
          id="quickstart-custom-name"
          value={fields.customName}
          onChange={(e) => onChange({ customName: e.target.value })}
          placeholder={t("quickStart.custom.namePlaceholder", {
            defaultValue: "例如：我的 DeepSeek 网关",
          })}
        />
      </div>
      <div className="space-y-2">
        <Label htmlFor="quickstart-custom-base-url">
          {t("quickStart.custom.baseUrl", { defaultValue: "Base URL" })}
        </Label>
        <Input
          id="quickstart-custom-base-url"
          value={fields.customBaseUrl}
          onChange={(e) => onChange({ customBaseUrl: e.target.value })}
          placeholder="https://api.example.com"
          className="font-mono text-sm"
        />
      </div>
      <div className="space-y-2">
        <Label htmlFor="quickstart-custom-model">{modelLabel}</Label>
        <Input
          id="quickstart-custom-model"
          value={fields.customModel}
          onChange={(e) => handleModelChange(e.target.value)}
          className="font-mono text-sm"
        />
      </div>
    </div>
  );
}
