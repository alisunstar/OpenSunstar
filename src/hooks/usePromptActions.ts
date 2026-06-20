import { useState, useCallback } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import {
  promptsApi,
  dryRunApi,
  type Prompt,
  type PromptActivationPreview,
  type AppId,
} from "@/lib/api";

export function usePromptActions(appId: AppId) {
  const { t } = useTranslation();
  const [prompts, setPrompts] = useState<Record<string, Prompt>>({});
  const [loading, setLoading] = useState(false);
  const [currentFileContent, setCurrentFileContent] = useState<string | null>(
    null,
  );
  const [pendingActivation, setPendingActivation] = useState<{
    id: string;
    preview: PromptActivationPreview;
  } | null>(null);

  const reload = useCallback(async () => {
    setLoading(true);
    try {
      const data = await promptsApi.getPrompts(appId);
      setPrompts(data);

      try {
        const content = await promptsApi.getCurrentFileContent(appId);
        setCurrentFileContent(content);
      } catch {
        setCurrentFileContent(null);
      }
    } catch {
      toast.error(t("prompts.loadFailed"));
    } finally {
      setLoading(false);
    }
  }, [appId, t]);

  const savePrompt = useCallback(
    async (id: string, prompt: Prompt) => {
      try {
        await promptsApi.upsertPrompt(appId, id, prompt);
        await reload();
        toast.success(t("prompts.saveSuccess"), { closeButton: true });
      } catch {
        toast.error(t("prompts.saveFailed"));
        throw new Error("save failed");
      }
    },
    [appId, reload, t],
  );

  const deletePrompt = useCallback(
    async (id: string) => {
      try {
        await promptsApi.deletePrompt(appId, id);
        await reload();
        toast.success(t("prompts.deleteSuccess"), { closeButton: true });
      } catch {
        toast.error(t("prompts.deleteFailed"));
        throw new Error("delete failed");
      }
    },
    [appId, reload, t],
  );

  const enablePrompt = useCallback(
    async (id: string) => {
      try {
        await promptsApi.enablePrompt(appId, id);
        await reload();
        toast.success(t("prompts.enableSuccess"), { closeButton: true });
      } catch {
        toast.error(t("prompts.enableFailed"));
        throw new Error("enable failed");
      }
    },
    [appId, reload, t],
  );

  const confirmPendingActivation = useCallback(async () => {
    if (!pendingActivation) return;
    const { id } = pendingActivation;
    setPendingActivation(null);
    await enablePrompt(id);
  }, [pendingActivation, enablePrompt]);

  const cancelPendingActivation = useCallback(() => {
    setPendingActivation(null);
    void reload();
  }, [reload]);

  const toggleEnabled = useCallback(
    async (id: string, enabled: boolean) => {
      const previousPrompts = prompts;

      if (enabled) {
        const updatedPrompts = Object.keys(prompts).reduce(
          (acc, key) => {
            acc[key] = { ...prompts[key], enabled: key === id };
            return acc;
          },
          {} as Record<string, Prompt>,
        );
        setPrompts(updatedPrompts);
      } else {
        setPrompts((prev) => ({
          ...prev,
          [id]: { ...prev[id], enabled: false },
        }));
      }

      try {
        if (enabled) {
          const dryRun = await dryRunApi.getMode();
          if (dryRun) {
            const preview = await promptsApi.previewActivation(appId, id);
            setPendingActivation({ id, preview });
            return;
          }
          await promptsApi.enablePrompt(appId, id);
          toast.success(t("prompts.enableSuccess"), { closeButton: true });
        } else {
          await promptsApi.upsertPrompt(appId, id, {
            ...prompts[id],
            enabled: false,
          });
          toast.success(t("prompts.disableSuccess"), { closeButton: true });
        }
        await reload();
      } catch {
        setPrompts(previousPrompts);
        toast.error(
          enabled ? t("prompts.enableFailed") : t("prompts.disableFailed"),
        );
        throw new Error("toggle failed");
      }
    },
    [appId, prompts, reload, t],
  );

  const importFromFile = useCallback(async () => {
    try {
      const id = await promptsApi.importFromFile(appId);
      await reload();
      toast.success(t("prompts.importSuccess"), { closeButton: true });
      return id;
    } catch {
      toast.error(t("prompts.importFailed"));
      throw new Error("import failed");
    }
  }, [appId, reload, t]);

  return {
    prompts,
    loading,
    currentFileContent,
    pendingActivation,
    reload,
    savePrompt,
    deletePrompt,
    enablePrompt,
    toggleEnabled,
    confirmPendingActivation,
    cancelPendingActivation,
    importFromFile,
  };
}
