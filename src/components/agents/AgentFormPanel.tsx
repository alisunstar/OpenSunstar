import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import MarkdownEditor from "@/components/MarkdownEditor";
import { FullScreenPanel } from "@/components/common/FullScreenPanel";
import { AppToggleGroup } from "@/components/common/AppToggleGroup";
import type { Agent } from "@/lib/api/agents";
import type { AppId } from "@/lib/api";
import {
  AGENT_APP_IDS,
  AGENT_DISABLED_APP_KEYS,
} from "./agentAppConfig";

const DEFAULT_AGENT_CONTENT = `---
name: code-reviewer
description: Reviews code for quality, bugs, and best practices
---

You are a meticulous code reviewer. Focus on correctness, security, and maintainability.
`;

interface AgentFormPanelProps {
  editingId?: string;
  initialData?: Agent;
  onSave: (agent: Agent) => Promise<void>;
  onClose: () => void;
}

export function AgentFormPanel({
  editingId,
  initialData,
  onSave,
  onClose,
}: AgentFormPanelProps) {
  const { t } = useTranslation();
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [content, setContent] = useState(DEFAULT_AGENT_CONTENT);
  const [enabledApps, setEnabledApps] = useState<
    Partial<Record<AppId, boolean>>
  >({
    claude: true,
    codex: false,
    gemini: false,
    opencode: false,
    hermes: false,
  });
  const [saving, setSaving] = useState(false);
  const [isDarkMode, setIsDarkMode] = useState(false);

  const disabledApps = useMemo(
    () => ({
      hermes: t(AGENT_DISABLED_APP_KEYS.hermes, {
        defaultValue: "暂不支持文件同步",
      }),
    }),
    [t],
  );

  useEffect(() => {
    setIsDarkMode(document.documentElement.classList.contains("dark"));
    const observer = new MutationObserver(() => {
      setIsDarkMode(document.documentElement.classList.contains("dark"));
    });
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["class"],
    });
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    if (initialData) {
      setName(initialData.name);
      setDescription(initialData.description || "");
      setContent(initialData.content);
      setEnabledApps({
        claude: initialData.enabledClaude,
        codex: initialData.enabledCodex,
        gemini: initialData.enabledGemini,
        opencode: initialData.enabledOpencode,
        hermes: false,
      });
    }
  }, [initialData]);

  const handleSave = async () => {
    const trimmedName = name.trim();
    if (!trimmedName) return;
    if (/[/\\]|\.\./.test(trimmedName)) return;

    setSaving(true);
    try {
      const now = Math.floor(Date.now() / 1000);
      const id = editingId || `agent-${Date.now()}`;
      const agent: Agent = {
        id,
        name: trimmedName,
        description: description.trim() || undefined,
        content: content.trim(),
        enabledClaude: !!enabledApps.claude,
        enabledCodex: !!enabledApps.codex,
        enabledGemini: !!enabledApps.gemini,
        enabledOpencode: !!enabledApps.opencode,
        enabledHermes: false,
        createdAt: initialData?.createdAt ?? now,
        updatedAt: now,
      };
      await onSave(agent);
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
          ? t("agents.edit", { defaultValue: "编辑 Subagent" })
          : t("agents.add", { defaultValue: "添加 Subagent" })
      }
      onClose={onClose}
      footer={
        <Button
          onClick={() => void handleSave()}
          disabled={saving || !name.trim()}
        >
          {t("common.save")}
        </Button>
      }
    >
      <div className="space-y-4 max-w-3xl">
        <div className="space-y-2">
          <Label htmlFor="agent-name">
            {t("agents.form.name", { defaultValue: "名称" })}
          </Label>
          <Input
            id="agent-name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="code-reviewer"
          />
        </div>
        <div className="space-y-2">
          <Label htmlFor="agent-desc">
            {t("agents.form.description", { defaultValue: "描述" })}
          </Label>
          <Input
            id="agent-desc"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
          />
        </div>
        <div className="space-y-2">
          <Label>
            {t("agents.form.syncTargets", { defaultValue: "同步到" })}
          </Label>
          <AppToggleGroup
            apps={enabledApps}
            appIds={AGENT_APP_IDS}
            disabledApps={disabledApps}
            onToggle={(app, enabled) =>
              setEnabledApps((prev) => ({ ...prev, [app]: enabled }))
            }
          />
          <p className="text-xs text-muted-foreground">
            {t("agents.form.geminiNote", {
              defaultValue:
                "Gemini 需在 settings.json 开启 experimental.enableSubagents",
            })}
          </p>
          <p className="text-xs text-muted-foreground">
            {t("agents.form.codexNote", {
              defaultValue:
                "Codex 将自动把 Markdown 转为 TOML 写入 ~/.codex/agents/",
            })}
          </p>
        </div>
        <div className="space-y-2">
          <Label>
            {t("agents.form.content", {
              defaultValue: "定义（Markdown + YAML frontmatter）",
            })}
          </Label>
          <MarkdownEditor
            value={content}
            onChange={setContent}
            darkMode={isDarkMode}
            minHeight="320px"
          />
        </div>
      </div>
    </FullScreenPanel>
  );
}
