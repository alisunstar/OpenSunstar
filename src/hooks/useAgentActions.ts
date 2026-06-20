import { useState, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { agentsApi, type Agent } from "@/lib/api/agents";
import type { AppId } from "@/lib/api";

const APP_FIELD_MAP: Record<
  AppId,
  | keyof Pick<
      Agent,
      | "enabledClaude"
      | "enabledCodex"
      | "enabledGemini"
      | "enabledOpencode"
      | "enabledHermes"
    >
  | null
> = {
  claude: "enabledClaude",
  "claude-desktop": null,
  codex: "enabledCodex",
  gemini: "enabledGemini",
  opencode: "enabledOpencode",
  openclaw: null,
  hermes: null,
};

const SYNCABLE_APPS: AppId[] = ["claude", "codex", "gemini", "opencode"];

export function useAgentActions() {
  const { t } = useTranslation();
  const [agents, setAgents] = useState<Record<string, Agent>>({});
  const [loading, setLoading] = useState(false);

  const reload = useCallback(async () => {
    setLoading(true);
    try {
      const data = await agentsApi.getAll();
      setAgents(data);
    } catch {
      toast.error(t("agents.loadFailed", { defaultValue: "加载 Subagent 失败" }));
    } finally {
      setLoading(false);
    }
  }, [t]);

  const saveAgent = useCallback(
    async (agent: Agent) => {
      try {
        await agentsApi.upsert(agent);
        await reload();
        toast.success(t("agents.saveSuccess", { defaultValue: "保存成功" }));
        if (agent.enabledGemini) {
          toast.info(
            t("agents.geminiHint", {
              defaultValue:
                "Gemini 需在 settings.json 中开启 experimental.enableSubagents 后生效",
            }),
          );
        }
        if (agent.enabledCodex) {
          toast.info(
            t("agents.codexHint", {
              defaultValue:
                "已转换为 TOML 并写入 ~/.codex/agents/{name}.toml",
            }),
          );
        }
      } catch {
        toast.error(t("agents.saveFailed", { defaultValue: "保存失败" }));
        throw new Error("save failed");
      }
    },
    [reload, t],
  );

  const deleteAgent = useCallback(
    async (id: string) => {
      try {
        await agentsApi.delete(id);
        await reload();
        toast.success(t("agents.deleteSuccess", { defaultValue: "删除成功" }));
      } catch {
        toast.error(t("agents.deleteFailed", { defaultValue: "删除失败" }));
        throw new Error("delete failed");
      }
    },
    [reload, t],
  );

  const toggleApp = useCallback(
    async (agentId: string, app: AppId, enabled: boolean) => {
      if (!SYNCABLE_APPS.includes(app)) return;

      const field = APP_FIELD_MAP[app];
      if (!field) return;

      const previous = agents;
      setAgents((current) => {
        const item = current[agentId];
        if (!item) return current;
        return { ...current, [agentId]: { ...item, [field]: enabled } };
      });

      try {
        await agentsApi.toggleApp(agentId, app, enabled);
        await reload();
        if (enabled && app === "gemini") {
          toast.info(
            t("agents.geminiHint", {
              defaultValue:
                "Gemini 需在 settings.json 中开启 experimental.enableSubagents 后生效",
            }),
          );
        }
        if (enabled && app === "codex") {
          toast.info(
            t("agents.codexHint", {
              defaultValue:
                "已转换为 TOML 并写入 ~/.codex/agents/{name}.toml",
            }),
          );
        }
      } catch {
        setAgents(previous);
        toast.error(
          t("agents.toggleFailed", { defaultValue: "切换同步目标失败" }),
        );
      }
    },
    [agents, reload, t],
  );

  return { agents, loading, reload, saveAgent, deleteAgent, toggleApp };
}
