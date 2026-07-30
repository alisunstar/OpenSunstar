import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronDown, ChevronRight } from "lucide-react";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { ModelInputWithFetch } from "@/components/providers/forms/shared";
import type { QuickStartAppId } from "@/config/quickStartCurated";
import type { FetchedModel } from "@/lib/api/model-fetch";
import type {
  QuickStartFormFields,
  QuickStartSelection,
} from "@/lib/quickStart/types";
import type { ClaudeApiFormat } from "@/types";

interface QuickStartAdvancedPanelProps {
  appId: QuickStartAppId;
  selection: QuickStartSelection;
  fields: QuickStartFormFields;
  availableModels?: FetchedModel[];
  onChange: (patch: Partial<QuickStartFormFields>) => void;
}

export function QuickStartAdvancedPanel({
  appId,
  selection,
  fields,
  availableModels = [],
  onChange,
}: QuickStartAdvancedPanelProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);

  if (selection.mode === "official") return null;

  const protocolEditable = selection.mode === "custom";

  return (
    <div className="rounded-lg border border-border">
      <button
        type="button"
        className="flex w-full items-center gap-2 px-4 py-3 text-left text-sm font-medium"
        onClick={() => setOpen((value) => !value)}
      >
        {open ? (
          <ChevronDown className="h-4 w-4 shrink-0" />
        ) : (
          <ChevronRight className="h-4 w-4 shrink-0" />
        )}
        {t("quickStart.advanced.title", { defaultValue: "高级选项" })}
        <span className="text-xs font-normal text-muted-foreground">
          {t("quickStart.advanced.subtitle", {
            defaultValue: "大多数场景保持默认即可",
          })}
        </span>
      </button>

      {open && (
        <div className="space-y-4 border-t border-border px-4 py-3">
          {appId === "claude" && fields.advancedClaude && (
            <ClaudeAdvanced
              value={fields.advancedClaude}
              protocolEditable={protocolEditable}
              availableModels={availableModels}
              onChange={(advancedClaude) => onChange({ advancedClaude })}
            />
          )}

          {appId === "claude-desktop" && fields.advancedDesktop && (
            <DesktopAdvanced
              value={fields.advancedDesktop}
              protocolEditable={protocolEditable}
              availableModels={availableModels}
              onChange={(advancedDesktop) => onChange({ advancedDesktop })}
            />
          )}

          {appId === "codex" && fields.advancedCodex && (
            <>
              <ApiFormatControl
                value={fields.advancedCodex.apiFormat}
                editable={protocolEditable}
                allowed={["openai_responses", "openai_chat"]}
                onChange={(apiFormat) => {
                  if (
                    apiFormat === "openai_responses" ||
                    apiFormat === "openai_chat"
                  ) {
                    onChange({
                      advancedCodex: {
                        ...fields.advancedCodex!,
                        apiFormat,
                      },
                    });
                  }
                }}
              />
              <ModelField
                id="quickstart-codex-model"
                label={t("quickStart.advanced.defaultModel", {
                  defaultValue: "默认模型",
                })}
                value={fields.advancedCodex.defaultModel}
                availableModels={availableModels}
                onChange={(defaultModel) =>
                  onChange({
                    advancedCodex: {
                      ...fields.advancedCodex!,
                      defaultModel,
                    },
                  })
                }
              />
            </>
          )}

          {appId === "gemini" && fields.advancedGemini && (
            <>
              <ApiFormatControl
                value={fields.advancedGemini.apiFormat}
                editable={false}
                allowed={["gemini_native"]}
                onChange={() => undefined}
                lockedHint={t("quickStart.advanced.geminiNativeLocked", {
                  defaultValue:
                    "Gemini CLI 当前透传原生 generateContent 协议；OpenAI Chat 端点需使用已实现协议转换的客户端入口。",
                })}
              />
              <div className="space-y-2">
                <Label htmlFor="quickstart-gemini-base-url">Base URL</Label>
                <Input
                  id="quickstart-gemini-base-url"
                  value={fields.advancedGemini.baseUrl}
                  onChange={(event) =>
                    onChange({
                      advancedGemini: {
                        ...fields.advancedGemini!,
                        baseUrl: event.target.value,
                      },
                    })
                  }
                  className="font-mono text-sm"
                />
              </div>
              <ModelField
                id="quickstart-gemini-model"
                label="GEMINI_MODEL"
                value={fields.advancedGemini.model}
                availableModels={availableModels}
                onChange={(model) =>
                  onChange({
                    advancedGemini: { ...fields.advancedGemini!, model },
                  })
                }
              />
            </>
          )}
        </div>
      )}
    </div>
  );
}

function ClaudeAdvanced({
  value,
  protocolEditable,
  availableModels,
  onChange,
}: {
  value: NonNullable<QuickStartFormFields["advancedClaude"]>;
  protocolEditable: boolean;
  availableModels: FetchedModel[];
  onChange: (
    value: NonNullable<QuickStartFormFields["advancedClaude"]>,
  ) => void;
}) {
  const { t } = useTranslation();
  const rows = [
    {
      id: "sonnet",
      label: "Sonnet",
      modelKey: "sonnetModel",
      nameKey: "sonnetModelName",
      oneMKey: "sonnetSupports1m",
    },
    {
      id: "opus",
      label: "Opus",
      modelKey: "opusModel",
      nameKey: "opusModelName",
      oneMKey: "opusSupports1m",
    },
    {
      id: "fable",
      label: "Fable",
      modelKey: "fableModel",
      nameKey: "fableModelName",
      oneMKey: "fableSupports1m",
    },
    {
      id: "haiku",
      label: "Haiku",
      modelKey: "haikuModel",
      nameKey: "haikuModelName",
    },
    {
      id: "subagent",
      label: "Subagent",
      modelKey: "subagentModel",
      oneMKey: "subagentSupports1m",
    },
  ] as const;

  return (
    <>
      <ApiFormatControl
        value={value.apiFormat}
        editable={protocolEditable}
        allowed={[
          "anthropic",
          "openai_chat",
          "openai_responses",
          "gemini_native",
        ]}
        onChange={(apiFormat) => onChange({ ...value, apiFormat })}
      />

      <div className="space-y-2">
        <Label>
          {t("quickStart.advanced.authField", { defaultValue: "认证字段" })}
        </Label>
        {protocolEditable ? (
          <Select
            value={value.apiKeyField}
            onValueChange={(apiKeyField) =>
              onChange({
                ...value,
                apiKeyField: apiKeyField as typeof value.apiKeyField,
              })
            }
          >
            <SelectTrigger aria-label="认证字段">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="ANTHROPIC_AUTH_TOKEN">
                ANTHROPIC_AUTH_TOKEN (Authorization)
              </SelectItem>
              <SelectItem value="ANTHROPIC_API_KEY">
                ANTHROPIC_API_KEY (x-api-key)
              </SelectItem>
            </SelectContent>
          </Select>
        ) : (
          <ReadOnlyValue
            value={value.apiKeyField}
            label={t("quickStart.advanced.authField", {
              defaultValue: "认证字段",
            })}
          />
        )}
      </div>

      <div className="space-y-2 border-t pt-3">
        <div>
          <Label>
            {t("quickStart.advanced.roleMapping", {
              defaultValue: "模型角色映射",
            })}
          </Label>
          <p className="mt-1 text-xs text-muted-foreground">
            {t("quickStart.advanced.roleMappingHint", {
              defaultValue:
                "显示名称用于 /model 菜单；实际请求模型会写入 Claude Code 环境变量。",
            })}
          </p>
        </div>
        <div className="hidden grid-cols-[88px_1fr_1fr_70px] gap-2 text-xs text-muted-foreground md:grid">
          <span>{t("quickStart.advanced.role", { defaultValue: "角色" })}</span>
          <span>
            {t("quickStart.advanced.displayName", { defaultValue: "显示名称" })}
          </span>
          <span>
            {t("quickStart.advanced.requestModel", {
              defaultValue: "实际请求模型",
            })}
          </span>
          <span>1M</span>
        </div>
        {rows.map((row) => {
          const model = value[row.modelKey];
          const displayName = "nameKey" in row ? value[row.nameKey] : "";
          const supports1m =
            "oneMKey" in row && row.oneMKey ? value[row.oneMKey] : false;
          return (
            <div
              key={row.id}
              className="grid grid-cols-1 gap-2 md:grid-cols-[88px_1fr_1fr_70px]"
            >
              <div className="flex h-9 items-center rounded-md border bg-muted px-2 text-sm font-medium text-muted-foreground">
                {row.label}
              </div>
              {"nameKey" in row ? (
                <Input
                  aria-label={`${row.label} 显示名称`}
                  value={displayName}
                  onChange={(event) =>
                    onChange({ ...value, [row.nameKey]: event.target.value })
                  }
                />
              ) : (
                <div className="flex h-9 items-center rounded-md border bg-muted px-2 text-xs text-muted-foreground">
                  {t("quickStart.advanced.hiddenFromModelMenu", {
                    defaultValue: "不显示在 /model 菜单",
                  })}
                </div>
              )}
              <div>
                <Label
                  htmlFor={`quickstart-claude-${row.id}`}
                  className="sr-only"
                >
                  {t("quickStart.advanced.roleRequestModel", {
                    role: row.label,
                    defaultValue: `${row.label} 实际请求模型`,
                  })}
                </Label>
                <ModelInputWithFetch
                  id={`quickstart-claude-${row.id}`}
                  value={model}
                  onChange={(nextModel) => {
                    const patch: Record<string, string> = {
                      [row.modelKey]: nextModel,
                    };
                    if (
                      "nameKey" in row &&
                      (!displayName.trim() || displayName === model)
                    ) {
                      patch[row.nameKey] = nextModel;
                    }
                    onChange({ ...value, ...patch });
                  }}
                  fetchedModels={availableModels}
                  isLoading={false}
                />
              </div>
              {"oneMKey" in row && row.oneMKey ? (
                <label className="flex h-9 items-center gap-2 text-sm text-muted-foreground">
                  <Checkbox
                    checked={supports1m}
                    onCheckedChange={(checked) =>
                      onChange({
                        ...value,
                        [row.oneMKey]: checked === true,
                      })
                    }
                  />
                  1M
                </label>
              ) : (
                <span />
              )}
            </div>
          );
        })}
      </div>

      <ModelField
        id="quickstart-claude-fallback"
        label={t("quickStart.advanced.fallbackModel", {
          defaultValue: "默认兜底模型",
        })}
        value={value.fallbackModel}
        availableModels={availableModels}
        onChange={(fallbackModel) => onChange({ ...value, fallbackModel })}
      />
    </>
  );
}

function DesktopAdvanced({
  value,
  protocolEditable,
  availableModels,
  onChange,
}: {
  value: NonNullable<QuickStartFormFields["advancedDesktop"]>;
  protocolEditable: boolean;
  availableModels: FetchedModel[];
  onChange: (
    value: NonNullable<QuickStartFormFields["advancedDesktop"]>,
  ) => void;
}) {
  const { t } = useTranslation();
  const rows = [
    ["sonnet", "Sonnet"],
    ["opus", "Opus"],
    ["haiku", "Haiku"],
  ] as const;

  return (
    <>
      <ApiFormatControl
        value={value.apiFormat}
        editable={protocolEditable}
        allowed={[
          "anthropic",
          "openai_chat",
          "openai_responses",
          "gemini_native",
        ]}
        onChange={(apiFormat) => onChange({ ...value, apiFormat })}
      />
      <div className="space-y-2 border-t pt-3">
        <Label>
          {t("quickStart.advanced.desktopRoutes", {
            defaultValue: "Claude Desktop 路由映射",
          })}
        </Label>
        <div className="hidden grid-cols-[88px_1fr_1fr_70px] gap-2 text-xs text-muted-foreground md:grid">
          <span>{t("quickStart.advanced.role", { defaultValue: "角色" })}</span>
          <span>
            {t("quickStart.advanced.displayName", { defaultValue: "显示名称" })}
          </span>
          <span>
            {t("quickStart.advanced.requestModel", {
              defaultValue: "实际请求模型",
            })}
          </span>
          <span>1M</span>
        </div>
        {rows.map(([role, label]) => {
          const modelKey = `${role}Model` as const;
          const labelKey = `${role}Label` as const;
          const oneMKey = `${role}Supports1m` as const;
          return (
            <div
              key={role}
              className="grid grid-cols-1 gap-2 md:grid-cols-[88px_1fr_1fr_70px]"
            >
              <div className="flex h-9 items-center rounded-md border bg-muted px-2 text-sm font-medium text-muted-foreground">
                {label}
              </div>
              <Input
                aria-label={`${label} 显示名称`}
                value={value[labelKey]}
                onChange={(event) =>
                  onChange({ ...value, [labelKey]: event.target.value })
                }
              />
              <div>
                <Label
                  htmlFor={`quickstart-desktop-${role}`}
                  className="sr-only"
                >
                  {t("quickStart.advanced.roleRequestModel", {
                    role: label,
                    defaultValue: `${label} 实际请求模型`,
                  })}
                </Label>
                <ModelInputWithFetch
                  id={`quickstart-desktop-${role}`}
                  value={value[modelKey]}
                  onChange={(model) =>
                    onChange({ ...value, [modelKey]: model })
                  }
                  fetchedModels={availableModels}
                  isLoading={false}
                />
              </div>
              <label className="flex h-9 items-center gap-2 text-sm text-muted-foreground">
                <Checkbox
                  checked={value[oneMKey]}
                  onCheckedChange={(checked) =>
                    onChange({ ...value, [oneMKey]: checked === true })
                  }
                />
                1M
              </label>
            </div>
          );
        })}
      </div>
    </>
  );
}

function ModelField({
  id,
  label,
  value,
  availableModels,
  onChange,
}: {
  id: string;
  label: string;
  value: string;
  availableModels: FetchedModel[];
  onChange: (value: string) => void;
}) {
  return (
    <div className="space-y-2">
      <Label htmlFor={id}>{label}</Label>
      <ModelInputWithFetch
        id={id}
        value={value}
        onChange={onChange}
        fetchedModels={availableModels}
        isLoading={false}
      />
    </div>
  );
}

function ReadOnlyValue({ value, label }: { value: string; label?: string }) {
  return (
    <div
      role="textbox"
      aria-readonly="true"
      aria-label={label}
      className="rounded-md border bg-muted/50 px-3 py-2 font-mono text-sm"
    >
      {value}
    </div>
  );
}

function apiFormatLabel(value: ClaudeApiFormat): string {
  switch (value) {
    case "anthropic":
      return "Anthropic Messages";
    case "openai_chat":
      return "OpenAI Chat Completions";
    case "openai_responses":
      return "OpenAI Responses";
    case "gemini_native":
      return "Gemini Native";
  }
}

function ApiFormatControl({
  value,
  editable,
  allowed,
  onChange,
  lockedHint,
}: {
  value: ClaudeApiFormat;
  editable: boolean;
  allowed: ClaudeApiFormat[];
  onChange: (value: ClaudeApiFormat) => void;
  lockedHint?: string;
}) {
  const { t } = useTranslation();
  return (
    <div className="space-y-2">
      <Label>
        {t("quickStart.advanced.apiFormat", { defaultValue: "API 格式" })}
      </Label>
      {editable ? (
        <Select
          value={value}
          onValueChange={(next) => onChange(next as ClaudeApiFormat)}
        >
          <SelectTrigger aria-label="API 格式">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {allowed.map((format) => (
              <SelectItem key={format} value={format}>
                {apiFormatLabel(format)}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      ) : (
        <ReadOnlyValue value={apiFormatLabel(value)} label="API 格式" />
      )}
      <p className="text-xs text-muted-foreground">
        {editable
          ? t("quickStart.advanced.customProtocolHint", {
              defaultValue: "自定义配置按上游真实接口选择协议。",
            })
          : (lockedHint ??
            t("quickStart.advanced.presetProtocolLocked", {
              defaultValue:
                "供应商预设已按真实端点锁定协议，避免误选后连接失败。",
            }))}
      </p>
    </div>
  );
}
