import { useTranslation } from "react-i18next";
import { Edit3, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
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

interface IgnoreListItemProps {
  rule: IgnoreRule;
  onEdit: (id: string) => void;
  onDelete: (id: string) => void;
  onToggleApp: (id: string, app: AppId, enabled: boolean) => void;
}

export function IgnoreListItem({
  rule,
  onEdit,
  onDelete,
  onToggleApp,
}: IgnoreListItemProps) {
  const { t } = useTranslation();

  return (
    <div className="rounded-xl border border-border-default bg-muted/50 p-4">
      <div className="flex items-start gap-3">
        <div className="flex-1 min-w-0 space-y-2">
          <code className="text-sm font-mono text-foreground break-all">
            {rule.pattern}
          </code>
          {rule.description && (
            <p className="text-xs text-muted-foreground">{rule.description}</p>
          )}
          <AppToggleGroup
            apps={{
              claude: rule.enabledClaude,
              codex: rule.enabledCodex,
              gemini: rule.enabledGemini,
              opencode: rule.enabledOpencode,
              hermes: rule.enabledHermes,
            }}
            onToggle={(app, enabled) => onToggleApp(rule.id, app, enabled)}
            appIds={IGNORE_APP_IDS}
          />
        </div>
        <div className="flex gap-1 shrink-0">
          <Button variant="ghost" size="icon" onClick={() => onEdit(rule.id)}>
            <Edit3 size={16} />
          </Button>
          <Button
            variant="ghost"
            size="icon"
            onClick={() => onDelete(rule.id)}
            className="hover:text-red-500"
          >
            <Trash2 size={16} />
          </Button>
        </div>
      </div>
    </div>
  );
}
