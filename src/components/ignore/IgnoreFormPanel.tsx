import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { FullScreenPanel } from "@/components/common/FullScreenPanel";
import { AppToggleGroup } from "@/components/common/AppToggleGroup";
import type { IgnoreRule } from "@/lib/api/ignore";
import type { AppId } from "@/lib/api";

const IGNORE_APP_IDS: AppId[] = [
  "claude",
  "codex",
  "gemini",
  "opencode",
  "hermes",
];

interface IgnoreFormPanelProps {
  editingId?: string;
  initialData?: IgnoreRule;
  onSave: (rule: IgnoreRule) => Promise<void>;
  onClose: () => void;
}

export function IgnoreFormPanel({
  editingId,
  initialData,
  onSave,
  onClose,
}: IgnoreFormPanelProps) {
  const { t } = useTranslation();
  const [pattern, setPattern] = useState("");
  const [description, setDescription] = useState("");
  const [enabledApps, setEnabledApps] = useState<
    Partial<Record<AppId, boolean>>
  >({
    claude: true,
    codex: true,
    gemini: true,
    opencode: true,
    hermes: true,
  });
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (initialData) {
      setPattern(initialData.pattern);
      setDescription(initialData.description || "");
      setEnabledApps({
        claude: initialData.enabledClaude,
        codex: initialData.enabledCodex,
        gemini: initialData.enabledGemini,
        opencode: initialData.enabledOpencode,
        hermes: initialData.enabledHermes,
      });
    }
  }, [initialData]);

  const handleSave = async () => {
    const trimmed = pattern.trim();
    if (!trimmed) return;

    setSaving(true);
    try {
      const now = Math.floor(Date.now() / 1000);
      const id = editingId || `ignore-${Date.now()}`;
      const rule: IgnoreRule = {
        id,
        pattern: trimmed,
        description: description.trim() || undefined,
        enabledClaude: !!enabledApps.claude,
        enabledCodex: !!enabledApps.codex,
        enabledGemini: !!enabledApps.gemini,
        enabledOpencode: !!enabledApps.opencode,
        enabledHermes: !!enabledApps.hermes,
        sortIndex: initialData?.sortIndex ?? 0,
        createdAt: initialData?.createdAt ?? now,
      };
      await onSave(rule);
      onClose();
    } finally {
      setSaving(false);
    }
  };

  return (
    <FullScreenPanel
      isOpen
      title={
        editingId
          ? t("ignore.edit", { defaultValue: "编辑忽略规则" })
          : t("ignore.add", { defaultValue: "添加忽略规则" })
      }
      onClose={onClose}
      footer={
        <Button onClick={() => void handleSave()} disabled={saving || !pattern.trim()}>
          {t("common.save")}
        </Button>
      }
    >
      <div className="space-y-4 max-w-2xl">
        <div className="space-y-2">
          <Label htmlFor="ignore-pattern">
            {t("ignore.form.pattern", { defaultValue: "Glob 模式" })}
          </Label>
          <Input
            id="ignore-pattern"
            value={pattern}
            onChange={(e) => setPattern(e.target.value)}
            placeholder="node_modules/**"
            className="font-mono"
          />
        </div>
        <div className="space-y-2">
          <Label htmlFor="ignore-desc">
            {t("ignore.form.description", { defaultValue: "描述" })}
          </Label>
          <Input
            id="ignore-desc"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
          />
        </div>
        <div className="space-y-2">
          <Label>{t("ignore.form.syncTargets", { defaultValue: "同步到" })}</Label>
          <AppToggleGroup
            apps={enabledApps}
            onToggle={(app, enabled) =>
              setEnabledApps((prev) => ({ ...prev, [app]: enabled }))
            }
            appIds={IGNORE_APP_IDS}
          />
        </div>
      </div>
    </FullScreenPanel>
  );
}
