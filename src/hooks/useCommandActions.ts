import { useState, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { commandsApi, type Command } from "@/lib/api/commands";
import type { AppId } from "@/lib/api";

const APP_FIELD_MAP: Record<
  AppId,
  keyof Pick<
    Command,
    | "enabledClaude"
    | "enabledCodex"
    | "enabledGemini"
    | "enabledOpencode"
    | "enabledHermes"
  > | null
> = {
  claude: "enabledClaude",
  "claude-desktop": null,
  codex: "enabledCodex",
  gemini: "enabledGemini",
  opencode: "enabledOpencode",
  openclaw: null,
  hermes: "enabledHermes",
};

export function useCommandActions() {
  const { t } = useTranslation();
  const [commands, setCommands] = useState<Record<string, Command>>({});
  const [loading, setLoading] = useState(false);

  const reload = useCallback(async () => {
    setLoading(true);
    try {
      const data = await commandsApi.getAll();
      setCommands(data);
    } catch {
      toast.error(t("commands.loadFailed", { defaultValue: "加载命令失败" }));
    } finally {
      setLoading(false);
    }
  }, [t]);

  const saveCommand = useCallback(
    async (command: Command) => {
      try {
        await commandsApi.upsert(command);
        await reload();
        toast.success(t("commands.saveSuccess", { defaultValue: "保存成功" }));
      } catch {
        toast.error(t("commands.saveFailed", { defaultValue: "保存失败" }));
        throw new Error("save failed");
      }
    },
    [reload, t],
  );

  const deleteCommand = useCallback(
    async (id: string) => {
      try {
        await commandsApi.delete(id);
        await reload();
        toast.success(t("commands.deleteSuccess", { defaultValue: "删除成功" }));
      } catch {
        toast.error(t("commands.deleteFailed", { defaultValue: "删除失败" }));
        throw new Error("delete failed");
      }
    },
    [reload, t],
  );

  const toggleApp = useCallback(
    async (commandId: string, app: AppId, enabled: boolean) => {
      const field = APP_FIELD_MAP[app];
      if (!field) return;

      const previous = commands;
      setCommands((current) => {
        const cmd = current[commandId];
        if (!cmd) return current;
        return { ...current, [commandId]: { ...cmd, [field]: enabled } };
      });

      try {
        await commandsApi.toggleApp(commandId, app, enabled);
        await reload();
      } catch {
        setCommands(previous);
        toast.error(
          t("commands.toggleFailed", { defaultValue: "切换同步目标失败" }),
        );
      }
    },
    [commands, reload, t],
  );

  return { commands, loading, reload, saveCommand, deleteCommand, toggleApp };
}
