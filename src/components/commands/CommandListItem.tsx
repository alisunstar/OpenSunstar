import { useTranslation } from "react-i18next";
import { Edit3, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { AppToggleGroup } from "@/components/common/AppToggleGroup";
import type { Command } from "@/lib/api/commands";
import type { AppId } from "@/lib/api";

const COMMAND_APP_IDS: AppId[] = [
  "claude",
  "codex",
  "gemini",
  "opencode",
  "hermes",
];

interface CommandListItemProps {
  command: Command;
  onEdit: (id: string) => void;
  onDelete: (id: string) => void;
  onToggleApp: (id: string, app: AppId, enabled: boolean) => void;
}

export function CommandListItem({
  command,
  onEdit,
  onDelete,
  onToggleApp,
}: CommandListItemProps) {
  const { t } = useTranslation();

  const apps: Partial<Record<AppId, boolean>> = {
    claude: command.enabledClaude,
    codex: command.enabledCodex,
    gemini: command.enabledGemini,
    opencode: command.enabledOpencode,
    hermes: command.enabledHermes,
  };

  return (
    <div className="group relative rounded-xl border border-border-default bg-muted/50 p-4 transition-all hover:bg-muted hover:shadow-sm">
      <div className="flex items-start gap-4">
        <div className="flex-1 min-w-0 space-y-2">
          <div>
            <h3 className="font-medium text-foreground">{command.name}</h3>
            {command.description && (
              <p className="text-sm text-muted-foreground truncate">
                {command.description}
              </p>
            )}
          </div>
          <AppToggleGroup
            apps={apps}
            appIds={COMMAND_APP_IDS}
            onToggle={(app, enabled) => onToggleApp(command.id, app, enabled)}
          />
        </div>
        <div className="flex items-center gap-1 shrink-0">
          <Button
            type="button"
            variant="ghost"
            size="icon"
            onClick={() => onEdit(command.id)}
            title={t("common.edit")}
          >
            <Edit3 size={16} />
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            onClick={() => onDelete(command.id)}
            className="hover:text-red-500 hover:bg-red-100 dark:hover:text-red-400 dark:hover:bg-red-500/10"
            title={t("common.delete")}
          >
            <Trash2 size={16} />
          </Button>
        </div>
      </div>
    </div>
  );
}
