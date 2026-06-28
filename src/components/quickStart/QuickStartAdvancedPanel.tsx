import { useTranslation } from "react-i18next";
import { ChevronDown, ChevronRight } from "lucide-react";
import { useState } from "react";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import type { QuickStartAppId } from "@/config/quickStartCurated";
import type { QuickStartFormFields, QuickStartSelection } from "@/lib/quickStart/types";

interface QuickStartAdvancedPanelProps {
  appId: QuickStartAppId;
  selection: QuickStartSelection;
  fields: QuickStartFormFields;
  onChange: (patch: Partial<QuickStartFormFields>) => void;
}

export function QuickStartAdvancedPanel({
  appId,
  selection,
  fields,
  onChange,
}: QuickStartAdvancedPanelProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);

  if (selection.mode === "official") {
    return null;
  }

  return (
    <div className="rounded-lg border border-border">
      <button
        type="button"
        className="flex w-full items-center gap-2 px-4 py-3 text-left text-sm font-medium"
        onClick={() => setOpen((v) => !v)}
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
        <div className="space-y-3 border-t border-border px-4 py-3">
          {appId === "claude" && fields.advancedClaude && (
            <ClaudeAdvanced
              value={fields.advancedClaude}
              onChange={(advancedClaude) => onChange({ advancedClaude })}
            />
          )}
          {appId === "claude-desktop" && fields.advancedDesktop && (
            <div className="space-y-2">
              <Label>
                {t("quickStart.advanced.upstreamModel", {
                  defaultValue: "上游模型",
                })}
              </Label>
              <Input
                value={fields.advancedDesktop.upstreamModel}
                onChange={(e) =>
                  onChange({
                    advancedDesktop: {
                      ...fields.advancedDesktop!,
                      upstreamModel: e.target.value,
                    },
                  })
                }
                className="font-mono text-sm"
              />
            </div>
          )}
          {appId === "codex" && fields.advancedCodex && (
            <div className="space-y-2">
              <Label>
                {t("quickStart.advanced.defaultModel", {
                  defaultValue: "默认模型",
                })}
              </Label>
              <Input
                value={fields.advancedCodex.defaultModel}
                onChange={(e) =>
                  onChange({
                    advancedCodex: {
                      ...fields.advancedCodex!,
                      defaultModel: e.target.value,
                    },
                  })
                }
                className="font-mono text-sm"
              />
            </div>
          )}
          {appId === "gemini" && fields.advancedGemini && (
            <>
              <div className="space-y-2">
                <Label>Base URL</Label>
                <Input
                  value={fields.advancedGemini.baseUrl}
                  onChange={(e) =>
                    onChange({
                      advancedGemini: {
                        ...fields.advancedGemini!,
                        baseUrl: e.target.value,
                      },
                    })
                  }
                  className="font-mono text-sm"
                />
              </div>
              <div className="space-y-2">
                <Label>GEMINI_MODEL</Label>
                <Input
                  value={fields.advancedGemini.model}
                  onChange={(e) =>
                    onChange({
                      advancedGemini: {
                        ...fields.advancedGemini!,
                        model: e.target.value,
                      },
                    })
                  }
                  className="font-mono text-sm"
                />
              </div>
            </>
          )}
        </div>
      )}
    </div>
  );
}

function ClaudeAdvanced({
  value,
  onChange,
}: {
  value: NonNullable<QuickStartFormFields["advancedClaude"]>;
  onChange: (v: NonNullable<QuickStartFormFields["advancedClaude"]>) => void;
}) {
  const { t } = useTranslation();
  return (
    <>
      <div className="space-y-2">
        <Label>{t("quickStart.advanced.apiFormat", { defaultValue: "API 格式" })}</Label>
        <Select
          value={value.apiFormat}
          onValueChange={(apiFormat) =>
            onChange({
              ...value,
              apiFormat: apiFormat as typeof value.apiFormat,
            })
          }
        >
          <SelectTrigger>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="anthropic">Anthropic</SelectItem>
            <SelectItem value="openai_chat">OpenAI Chat</SelectItem>
            <SelectItem value="openai_responses">OpenAI Responses</SelectItem>
            <SelectItem value="gemini_native">Gemini Native</SelectItem>
          </SelectContent>
        </Select>
      </div>
      <div className="grid gap-2 sm:grid-cols-3">
        {(
          [
            ["haikuModel", "Haiku"],
            ["sonnetModel", "Sonnet"],
            ["opusModel", "Opus"],
          ] as const
        ).map(([key, label]) => (
          <div key={key} className="space-y-1">
            <Label className="text-xs">{label}</Label>
            <Input
              value={value[key]}
              onChange={(e) => onChange({ ...value, [key]: e.target.value })}
              className="font-mono text-xs"
            />
          </div>
        ))}
      </div>
    </>
  );
}
