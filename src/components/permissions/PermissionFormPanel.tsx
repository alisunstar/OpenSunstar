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
import type { PermissionType, ToolPermission } from "@/lib/api/permissions";

const PERMISSION_TYPES: PermissionType[] = [
  "allowedTools",
  "deniedTools",
  "autoApprove",
];

interface PermissionFormPanelProps {
  editingId?: string;
  initialData?: ToolPermission;
  onSave: (permission: ToolPermission) => Promise<void>;
  onClose: () => void;
}

export function PermissionFormPanel({
  editingId,
  initialData,
  onSave,
  onClose,
}: PermissionFormPanelProps) {
  const { t } = useTranslation();
  const [permissionType, setPermissionType] =
    useState<PermissionType>("allowedTools");
  const [toolPattern, setToolPattern] = useState("");
  const [description, setDescription] = useState("");
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (initialData) {
      setPermissionType(initialData.permissionType);
      setToolPattern(initialData.toolPattern);
      setDescription(initialData.description || "");
    }
  }, [initialData]);

  const handleSave = async () => {
    if (!toolPattern.trim()) return;

    setSaving(true);
    try {
      const now = Math.floor(Date.now() / 1000);
      const id = editingId || `perm-${Date.now()}`;
      const permission: ToolPermission = {
        id,
        permissionType,
        toolPattern: toolPattern.trim(),
        enabledClaude: true,
        description: description.trim() || undefined,
        sortIndex: initialData?.sortIndex ?? 0,
        createdAt: initialData?.createdAt ?? now,
      };
      await onSave(permission);
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
          ? t("permissions.edit", { defaultValue: "编辑权限" })
          : t("permissions.add", { defaultValue: "添加权限" })
      }
      onClose={onClose}
      footer={
        <Button
          onClick={() => void handleSave()}
          disabled={saving || !toolPattern.trim()}
        >
          {t("common.save")}
        </Button>
      }
    >
      <div className="space-y-4 max-w-2xl">
        <p className="text-sm text-muted-foreground">
          {t("permissions.claudeOnlyNote", {
            defaultValue: "当前仅同步到 Claude Code settings.json permissions 字段",
          })}
        </p>
        <div className="space-y-2">
          <Label>{t("permissions.form.type", { defaultValue: "权限类型" })}</Label>
          <Select
            value={permissionType}
            onValueChange={(v) => setPermissionType(v as PermissionType)}
          >
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {PERMISSION_TYPES.map((type) => (
                <SelectItem key={type} value={type}>
                  {t(`permissions.type.${type}`, { defaultValue: type })}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
        <div className="space-y-2">
          <Label htmlFor="perm-pattern">
            {t("permissions.form.toolPattern", { defaultValue: "工具匹配" })}
          </Label>
          <Input
            id="perm-pattern"
            value={toolPattern}
            onChange={(e) => setToolPattern(e.target.value)}
            placeholder="Bash(npm run *)"
            className="font-mono"
          />
        </div>
        <div className="space-y-2">
          <Label htmlFor="perm-desc">
            {t("permissions.form.description", { defaultValue: "描述" })}
          </Label>
          <Input
            id="perm-desc"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
          />
        </div>
      </div>
    </FullScreenPanel>
  );
}
