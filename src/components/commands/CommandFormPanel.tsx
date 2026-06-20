import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import MarkdownEditor from "@/components/MarkdownEditor";
import { FullScreenPanel } from "@/components/common/FullScreenPanel";
import { AppToggleGroup } from "@/components/common/AppToggleGroup";
import { CommandVariableHelp } from "./CommandVariableHelp";
import type { Command } from "@/lib/api/commands";
import type { AppId } from "@/lib/api";

const COMMAND_APP_IDS: AppId[] = [
  "claude",
  "codex",
  "gemini",
  "opencode",
  "hermes",
];

interface CommandFormPanelProps {
  editingId?: string;
  initialData?: Command;
  onSave: (command: Command) => Promise<void>;
  onClose: () => void;
}

export function CommandFormPanel({
  editingId,
  initialData,
  onSave,
  onClose,
}: CommandFormPanelProps) {
  const { t } = useTranslation();
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [content, setContent] = useState("");
  const [argumentsJson, setArgumentsJson] = useState("[]");
  const [enabledApps, setEnabledApps] = useState<
    Partial<Record<AppId, boolean>>
  >({
    claude: false,
    codex: false,
    gemini: false,
    opencode: false,
    hermes: false,
  });
  const [saving, setSaving] = useState(false);
  const [isDarkMode, setIsDarkMode] = useState(false);

  useEffect(() => {
    setIsDarkMode(document.documentElement.classList.contains("dark"));
    const observer = new MutationObserver(() => {
      setIsDarkMode(document.documentElement.classList.contains("dark"));
    });
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["class"],
    });
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    if (initialData) {
      setName(initialData.name);
      setDescription(initialData.description || "");
      setContent(initialData.content);
      setArgumentsJson(initialData.arguments || "[]");
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
    const trimmedName = name.trim();
    if (!trimmedName) return;
    if (/[/\\]|\.\./.test(trimmedName)) {
      return;
    }

    setSaving(true);
    try {
      const now = Math.floor(Date.now() / 1000);
      const id = editingId || `cmd-${Date.now()}`;
      const command: Command = {
        id,
        name: trimmedName,
        description: description.trim() || undefined,
        content: content.trim(),
        arguments: argumentsJson.trim() || "[]",
        enabledClaude: !!enabledApps.claude,
        enabledCodex: !!enabledApps.codex,
        enabledGemini: !!enabledApps.gemini,
        enabledOpencode: !!enabledApps.opencode,
        enabledHermes: !!enabledApps.hermes,
        createdAt: initialData?.createdAt ?? now,
        updatedAt: now,
      };
      await onSave(command);
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
          ? t("commands.edit", { defaultValue: "编辑命令" })
          : t("commands.add", { defaultValue: "添加命令" })
      }
      onClose={onClose}
      footer={
        <Button onClick={() => void handleSave()} disabled={saving || !name.trim()}>
          {t("common.save")}
        </Button>
      }
    >
      <div className="space-y-4 max-w-3xl">
        <div className="space-y-2">
          <Label htmlFor="command-name">
            {t("commands.form.name", { defaultValue: "命令名称" })}
          </Label>
          <Input
            id="command-name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="review-pr"
          />
        </div>
        <div className="space-y-2">
          <Label htmlFor="command-desc">
            {t("commands.form.description", { defaultValue: "描述" })}
          </Label>
          <Input
            id="command-desc"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
          />
        </div>
        <div className="space-y-2">
          <Label>
            {t("commands.form.syncTargets", { defaultValue: "同步到" })}
          </Label>
          <AppToggleGroup
            apps={enabledApps}
            appIds={COMMAND_APP_IDS}
            onToggle={(app, enabled) =>
              setEnabledApps((prev) => ({ ...prev, [app]: enabled }))
            }
          />
        </div>
        <CommandVariableHelp />
        <div className="space-y-2">
          <Label>
            {t("commands.form.content", { defaultValue: "命令内容（Markdown）" })}
          </Label>
          <MarkdownEditor
            value={content}
            onChange={setContent}
            darkMode={isDarkMode}
            minHeight="280px"
          />
        </div>
        <div className="space-y-2">
          <Label htmlFor="command-args">
            {t("commands.form.arguments", { defaultValue: "参数定义（JSON）" })}
          </Label>
          <Input
            id="command-args"
            value={argumentsJson}
            onChange={(e) => setArgumentsJson(e.target.value)}
            className="font-mono text-sm"
          />
        </div>
      </div>
    </FullScreenPanel>
  );
}
