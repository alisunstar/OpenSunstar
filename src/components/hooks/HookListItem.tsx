import { useTranslation } from "react-i18next";
import { Edit3, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { Hook } from "@/lib/api/hooks";
import { HookEventTypeBadge } from "./HookEventTypeBadge";

interface HookListItemProps {
  hook: Hook;
  onEdit: (id: string) => void;
  onDelete: (id: string) => void;
}

export function HookListItem({ hook, onEdit, onDelete }: HookListItemProps) {
  const { t } = useTranslation();

  return (
    <div className="rounded-xl border border-border-default bg-muted/50 p-4">
      <div className="flex items-start gap-3">
        <div className="flex-1 min-w-0 space-y-2">
          <div className="flex flex-wrap items-center gap-2">
            <HookEventTypeBadge eventType={hook.eventType} />
            <code className="text-xs px-1.5 py-0.5 rounded bg-muted font-mono">
              {hook.toolPattern}
            </code>
          </div>
          <p className="text-sm font-mono text-foreground break-all">
            {hook.hookCommand}
          </p>
          {hook.description && (
            <p className="text-xs text-muted-foreground">{hook.description}</p>
          )}
          <p className="text-xs text-muted-foreground">
            {t("hooks.timeoutLabel", {
              seconds: hook.timeoutSeconds,
              defaultValue: "超时 {{seconds}}s",
            })}
          </p>
        </div>
        <div className="flex gap-1 shrink-0">
          <Button variant="ghost" size="icon" onClick={() => onEdit(hook.id)}>
            <Edit3 size={16} />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            onClick={() => onDelete(hook.id)}
            className="hover:text-red-500"
          >
            <Trash2 size={16} />
          </Button>
        </div>
      </div>
    </div>
  );
}
