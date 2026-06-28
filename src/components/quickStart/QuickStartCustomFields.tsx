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

  return (
    <div className="space-y-3 rounded-lg border border-dashed border-border p-4">
      <p className="text-xs text-muted-foreground">
        {t("quickStart.custom.hint", {
          defaultValue: "自定义网关：填写最小必填项即可接入，高级配置可在供应商管理中编辑。",
        })}
      </p>
      <div className="space-y-2">
        <Label>{t("quickStart.custom.name", { defaultValue: "供应商名称" })}</Label>
        <Input
          value={fields.customName}
          onChange={(e) => onChange({ customName: e.target.value })}
          placeholder={t("quickStart.custom.namePlaceholder", {
            defaultValue: "例如：我的 DeepSeek 网关",
          })}
        />
      </div>
      <div className="space-y-2">
        <Label>{t("quickStart.custom.baseUrl", { defaultValue: "Base URL" })}</Label>
        <Input
          value={fields.customBaseUrl}
          onChange={(e) => onChange({ customBaseUrl: e.target.value })}
          placeholder="https://api.example.com"
          className="font-mono text-sm"
        />
      </div>
      <div className="space-y-2">
        <Label>{modelLabel}</Label>
        <Input
          value={fields.customModel}
          onChange={(e) => onChange({ customModel: e.target.value })}
          className="font-mono text-sm"
        />
      </div>
    </div>
  );
}
