import { useTranslation } from "react-i18next";
import { Edit3, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { AppToggleGroup } from "@/components/common/AppToggleGroup";
import type { Agent } from "@/lib/api/agents";
import type { AppId } from "@/lib/api";
import {
  AGENT_APP_IDS,
  AGENT_DISABLED_APP_KEYS,
} from "./agentAppConfig";

interface AgentListItemProps {
  agent: Agent;
  onEdit: (id: string) => void;
  onDelete: (id: string) => void;
  onToggleApp: (id: string, app: AppId, enabled: boolean) => void;
}

export function AgentListItem({
  agent,
  onEdit,
  onDelete,
  onToggleApp,
}: AgentListItemProps) {
  const { t } = useTranslation();

  const apps: Partial<Record<AppId, boolean>> = {
    claude: agent.enabledClaude,
    codex: agent.enabledCodex,
    gemini: agent.enabledGemini,
    opencode: agent.enabledOpencode,
    hermes: false,
  };

  const disabledApps: Partial<Record<AppId, string>> = {
    hermes: t(AGENT_DISABLED_APP_KEYS.hermes, {
      defaultValue: "暂不支持文件同步",
    }),
  };

  return (
    <div className="group relative rounded-xl border border-border-default bg-muted/50 p-4 transition-all hover:bg-muted hover:shadow-sm">
      <div className="flex items-start gap-4">
        <div className="flex-1 min-w-0 space-y-2">
          <div>
            <h3 className="font-medium text-foreground">{agent.name}</h3>
            {agent.description && (
              <p className="text-sm text-muted-foreground truncate">
                {agent.description}
              </p>
            )}
          </div>
          <AppToggleGroup
            apps={apps}
            appIds={AGENT_APP_IDS}
            disabledApps={disabledApps}
            onToggle={(app, enabled) => onToggleApp(agent.id, app, enabled)}
          />
        </div>
        <div className="flex items-center gap-1 shrink-0">
          <Button
            type="button"
            variant="ghost"
            size="icon"
            onClick={() => onEdit(agent.id)}
            title={t("common.edit")}
          >
            <Edit3 size={16} />
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            onClick={() => onDelete(agent.id)}
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
