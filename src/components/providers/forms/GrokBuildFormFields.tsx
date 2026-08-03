import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  buildGrokBuildConfig,
  parseGrokBuildConfig,
  type GrokBuildConfigValues,
} from "@/utils/grokBuildConfig";

interface GrokBuildFormFieldsProps {
  value: string;
  onChange: (value: string) => void;
}

export function GrokBuildFormFields({
  value,
  onChange,
}: GrokBuildFormFieldsProps) {
  const { t } = useTranslation();
  const parsed = useMemo(() => {
    try {
      const settings = JSON.parse(value || "{}") as { config?: unknown };
      return parseGrokBuildConfig(
        typeof settings.config === "string" ? settings.config : "",
      );
    } catch {
      return parseGrokBuildConfig("");
    }
  }, [value]);

  const update = (patch: Partial<GrokBuildConfigValues>) => {
    const next = buildGrokBuildConfig({ ...parsed, ...patch });
    onChange(JSON.stringify({ config: next }, null, 2));
  };

  return (
    <div className="space-y-4 rounded-lg border border-border p-4">
      <div className="space-y-1">
        <h3 className="text-sm font-medium">Grok Build</h3>
        <p className="text-xs text-muted-foreground">
          {t("providerForm.grokBuildHint", {
            defaultValue:
              "配置会写入 ~/.grok/config.toml；仅维护 Grok Build 原生模型配置。",
          })}
        </p>
      </div>
      <div className="grid gap-4 md:grid-cols-2">
        <Field
          label="Base URL"
          value={parsed.baseUrl}
          onChange={(baseUrl) => update({ baseUrl })}
        />
        <Field
          label="默认模型"
          value={parsed.model}
          onChange={(model) => update({ model })}
        />
        <Field
          label="API Key"
          value={parsed.apiKey}
          type="password"
          onChange={(apiKey) => update({ apiKey })}
        />
        <Field
          label="环境变量名（可选）"
          value={parsed.envKey}
          onChange={(envKey) => update({ envKey })}
          placeholder="XAI_API_KEY"
        />
      </div>
    </div>
  );
}

function Field({
  label,
  value,
  onChange,
  type = "text",
  placeholder,
}: {
  label: string;
  value: string;
  onChange: (value: string) => void;
  type?: string;
  placeholder?: string;
}) {
  return (
    <div className="space-y-2">
      <Label>{label}</Label>
      <Input
        type={type}
        value={value}
        placeholder={placeholder}
        onChange={(event) => onChange(event.target.value)}
        className="font-mono text-sm"
      />
    </div>
  );
}
