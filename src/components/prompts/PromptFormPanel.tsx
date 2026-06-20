import React, { useState, useEffect, useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import MarkdownEditor from "@/components/MarkdownEditor";
import { FullScreenPanel } from "@/components/common/FullScreenPanel";
import type { Prompt, AppId } from "@/lib/api";

const TARGET_APPS = ["claude", "codex", "gemini", "opencode", "hermes"] as const;

interface ParentOption {
  id: string;
  name: string;
}

interface PromptFormPanelProps {
  appId: AppId;
  editingId?: string;
  initialData?: Prompt;
  parentPrompts?: ParentOption[];
  onSave: (id: string, prompt: Prompt) => Promise<void>;
  onClose: () => void;
}

const PromptFormPanel: React.FC<PromptFormPanelProps> = ({
  appId,
  editingId,
  initialData,
  parentPrompts = [],
  onSave,
  onClose,
}) => {
  const { t } = useTranslation();
  const appName = t(`apps.${appId}`);
  const filenameMap: Record<AppId, string> = {
    claude: "CLAUDE.md",
    "claude-desktop": "CLAUDE.md",
    codex: "AGENTS.md",
    gemini: "GEMINI.md",
    opencode: "AGENTS.md",
    openclaw: "AGENTS.md",
    hermes: "AGENTS.md",
  };
  const filename = filenameMap[appId];
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [content, setContent] = useState("");
  const [isFragment, setIsFragment] = useState(false);
  const [parentPromptId, setParentPromptId] = useState("");
  const [priority, setPriority] = useState("0");
  const [targetsText, setTargetsText] = useState("*");
  const [globsText, setGlobsText] = useState("[]");
  const [saving, setSaving] = useState(false);
  const [isDarkMode, setIsDarkMode] = useState(false);

  const parentOptions = useMemo(
    () => parentPrompts.filter((p) => p.id !== editingId),
    [parentPrompts, editingId],
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
      setIsFragment(!!initialData.isFragment);
      setParentPromptId(initialData.parentPromptId || "");
      setPriority(String(initialData.priority ?? 0));
      try {
        const targets: string[] = JSON.parse(initialData.targets || '["*"]');
        setTargetsText(targets.join(", "));
      } catch {
        setTargetsText("*");
      }
      setGlobsText(initialData.globs || "[]");
    }
  }, [initialData]);

  const handleSave = async () => {
    if (!name.trim()) return;
    if (isFragment && !parentPromptId) return;

    setSaving(true);
    try {
      const id = editingId || `prompt-${Date.now()}`;
      const timestamp = Math.floor(Date.now() / 1000);
      const targetList = targetsText
        .split(",")
        .map((s) => s.trim())
        .filter(Boolean);
      const targets = JSON.stringify(
        targetList.length > 0 ? targetList : ["*"],
      );

      const prompt: Prompt = {
        id,
        name: name.trim(),
        description: description.trim() || undefined,
        content: content.trim(),
        enabled: initialData?.enabled || false,
        targets,
        globs: globsText.trim() || "[]",
        priority: parseInt(priority, 10) || 0,
        isFragment,
        parentPromptId: isFragment ? parentPromptId : null,
        createdAt: initialData?.createdAt || timestamp,
        updatedAt: timestamp,
      };
      await onSave(id, prompt);
      onClose();
    } finally {
      setSaving(false);
    }
  };

  const title = editingId
    ? t("prompts.editTitle", { appName })
    : t("prompts.addTitle", { appName });

  return (
    <FullScreenPanel
      isOpen={true}
      title={title}
      onClose={onClose}
      footer={
        <Button
          type="button"
          onClick={() => void handleSave()}
          disabled={
            !name.trim() ||
            saving ||
            (isFragment && !parentPromptId)
          }
          className="bg-primary text-primary-foreground hover:bg-primary/90 disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {saving ? t("common.saving") : t("common.save")}
        </Button>
      }
    >
      <div className="glass rounded-xl p-6 border border-white/10 space-y-6">
        <div>
          <Label htmlFor="name" className="text-foreground">
            {t("prompts.name")}
          </Label>
          <Input
            id="name"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder={t("prompts.namePlaceholder")}
            className="mt-2"
          />
        </div>

        <div>
          <Label htmlFor="description" className="text-foreground">
            {t("prompts.description")}
          </Label>
          <Input
            id="description"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder={t("prompts.descriptionPlaceholder")}
            className="mt-2"
          />
        </div>

        <div className="rounded-lg border border-border/60 p-4 space-y-4">
          <label className="flex items-center gap-2 cursor-pointer">
            <Checkbox
              checked={isFragment}
              onCheckedChange={(v) => setIsFragment(v === true)}
            />
            <span className="text-sm font-medium">
              {t("prompts.fragment.isFragment", { defaultValue: "规则片段" })}
            </span>
          </label>

          {isFragment && (
            <>
              <div>
                <Label>{t("prompts.fragment.parent", { defaultValue: "所属 Prompt" })}</Label>
                <Select value={parentPromptId} onValueChange={setParentPromptId}>
                  <SelectTrigger className="mt-2">
                    <SelectValue
                      placeholder={t("prompts.fragment.selectParent", {
                        defaultValue: "选择父 Prompt",
                      })}
                    />
                  </SelectTrigger>
                  <SelectContent>
                    {parentOptions.map((p) => (
                      <SelectItem key={p.id} value={p.id}>
                        {p.name}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div>
                  <Label htmlFor="priority">
                    {t("prompts.fragment.priority", { defaultValue: "优先级" })}
                  </Label>
                  <Input
                    id="priority"
                    type="number"
                    value={priority}
                    onChange={(e) => setPriority(e.target.value)}
                    className="mt-2"
                  />
                </div>
                <div>
                  <Label htmlFor="targets">
                    {t("prompts.fragment.targets", {
                      defaultValue: "目标工具（逗号分隔，* 表示全部）",
                    })}
                  </Label>
                  <Input
                    id="targets"
                    value={targetsText}
                    onChange={(e) => setTargetsText(e.target.value)}
                    placeholder={TARGET_APPS.join(", ")}
                    className="mt-2 font-mono text-sm"
                  />
                </div>
              </div>
              <div>
                <Label htmlFor="globs">
                  {t("prompts.fragment.globs", {
                    defaultValue: "文件 Glob（JSON 数组）",
                  })}
                </Label>
                <Input
                  id="globs"
                  value={globsText}
                  onChange={(e) => setGlobsText(e.target.value)}
                  className="mt-2 font-mono text-sm"
                />
              </div>
            </>
          )}
        </div>

        <div>
          <Label htmlFor="content" className="block mb-2 text-foreground">
            {t("prompts.content")}
          </Label>
          <MarkdownEditor
            value={content}
            onChange={setContent}
            placeholder={t("prompts.contentPlaceholder", { filename })}
            darkMode={isDarkMode}
            minHeight="167px"
          />
        </div>
      </div>
    </FullScreenPanel>
  );
};

export default PromptFormPanel;
