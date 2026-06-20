import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { FullScreenPanel } from "@/components/common/FullScreenPanel";
import type { Hook, HookEventType } from "@/lib/api/hooks";

const EVENT_TYPES: HookEventType[] = [
  "PreToolUse",
  "PostToolUse",
  "Notification",
  "Stop",
];

interface HookFormPanelProps {
  editingId?: string;
  initialData?: Hook;
  onSave: (hook: Hook) => Promise<void>;
  onClose: () => void;
}

export function HookFormPanel({
  editingId,
  initialData,
  onSave,
  onClose,
}: HookFormPanelProps) {
  const { t } = useTranslation();
  const [eventType, setEventType] = useState<HookEventType>("PreToolUse");
  const [toolPattern, setToolPattern] = useState("*");
  const [hookCommand, setHookCommand] = useState("");
  const [timeoutSeconds, setTimeoutSeconds] = useState("30");
  const [description, setDescription] = useState("");
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (initialData) {
      setEventType(initialData.eventType);
      setToolPattern(initialData.toolPattern);
      setHookCommand(initialData.hookCommand);
      setTimeoutSeconds(String(initialData.timeoutSeconds));
      setDescription(initialData.description || "");
    }
  }, [initialData]);

  const handleSave = async () => {
    const timeout = parseInt(timeoutSeconds, 10);
    if (!hookCommand.trim() || Number.isNaN(timeout)) return;
    if (timeout < 1 || timeout > 300) return;

    setSaving(true);
    try {
      const now = Math.floor(Date.now() / 1000);
      const id = editingId || `hook-${Date.now()}`;
      const hook: Hook = {
        id,
        eventType,
        toolPattern: toolPattern.trim() || "*",
        hookCommand: hookCommand.trim(),
        timeoutSeconds: timeout,
        enabledClaude: true,
        description: description.trim() || undefined,
        sortIndex: initialData?.sortIndex ?? 0,
        createdAt: initialData?.createdAt ?? now,
      };
      await onSave(hook);
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
          ? t("hooks.edit", { defaultValue: "编辑钩子" })
          : t("hooks.add", { defaultValue: "添加钩子" })
      }
      onClose={onClose}
      footer={
        <Button
          onClick={() => void handleSave()}
          disabled={saving || !hookCommand.trim()}
        >
          {t("common.save")}
        </Button>
      }
    >
      <div className="space-y-4 max-w-2xl">
        <p className="text-sm text-muted-foreground">
          {t("hooks.claudeOnlyNote", {
            defaultValue: "当前仅支持同步到 Claude Code（settings.json hooks 字段）",
          })}
        </p>
        <div className="space-y-2">
          <Label>{t("hooks.form.eventType", { defaultValue: "事件类型" })}</Label>
          <Select
            value={eventType}
            onValueChange={(v) => setEventType(v as HookEventType)}
          >
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {EVENT_TYPES.map((type) => (
                <SelectItem key={type} value={type}>
                  {t(`hooks.eventType.${type}`, { defaultValue: type })}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        <div className="space-y-2">
          <Label htmlFor="tool-pattern">
            {t("hooks.form.toolPattern", { defaultValue: "工具匹配（glob）" })}
          </Label>
          <Input
            id="tool-pattern"
            value={toolPattern}
            onChange={(e) => setToolPattern(e.target.value)}
            placeholder="Bash"
          />
        </div>
        <div className="space-y-2">
          <Label htmlFor="hook-command">
            {t("hooks.form.hookCommand", { defaultValue: "Shell 命令" })}
          </Label>
          <Input
            id="hook-command"
            value={hookCommand}
            onChange={(e) => setHookCommand(e.target.value)}
            className="font-mono"
          />
        </div>
        <div className="space-y-2">
          <Label htmlFor="hook-timeout">
            {t("hooks.form.timeout", { defaultValue: "超时（秒，1-300）" })}
          </Label>
          <Input
            id="hook-timeout"
            type="number"
            min={1}
            max={300}
            value={timeoutSeconds}
            onChange={(e) => setTimeoutSeconds(e.target.value)}
          />
        </div>
        <div className="space-y-2">
          <Label htmlFor="hook-desc">
            {t("hooks.form.description", { defaultValue: "描述" })}
          </Label>
          <Input
            id="hook-desc"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
          />
        </div>
      </div>
    </FullScreenPanel>
  );
}
