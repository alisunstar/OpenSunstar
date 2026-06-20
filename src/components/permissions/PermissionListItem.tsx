import { useTranslation } from "react-i18next";
import { Edit3, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { ToolPermission } from "@/lib/api/permissions";

interface PermissionListItemProps {
  permission: ToolPermission;
  onEdit: (id: string) => void;
  onDelete: (id: string) => void;
}

export function PermissionListItem({
  permission,
  onEdit,
  onDelete,
}: PermissionListItemProps) {
  const { t } = useTranslation();

  return (
    <div className="rounded-xl border border-border-default bg-muted/50 p-4">
      <div className="flex items-start gap-3">
        <div className="flex-1 min-w-0 space-y-2">
          <div className="flex flex-wrap items-center gap-2">
            <span className="text-xs px-2 py-0.5 rounded-full bg-violet-500/10 text-violet-600 dark:text-violet-300 font-medium">
              {t(`permissions.type.${permission.permissionType}`, {
                defaultValue: permission.permissionType,
              })}
            </span>
            <code className="text-sm font-mono text-foreground break-all">
              {permission.toolPattern}
            </code>
          </div>
          {permission.description && (
            <p className="text-xs text-muted-foreground">
              {permission.description}
            </p>
          )}
        </div>
        <div className="flex gap-1 shrink-0">
          <Button variant="ghost" size="icon" onClick={() => onEdit(permission.id)}>
            <Edit3 size={16} />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            onClick={() => onDelete(permission.id)}
            className="hover:text-red-500"
          >
            <Trash2 size={16} />
          </Button>
        </div>
      </div>
    </div>
  );
}
