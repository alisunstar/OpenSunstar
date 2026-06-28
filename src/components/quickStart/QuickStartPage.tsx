import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQueryClient } from "@tanstack/react-query";
import {
  ExternalLink,
  KeyRound,
  Loader2,
  PlusCircle,
  Search,
  Sparkles,
  Zap,
} from "lucide-react";
import { toast } from "sonner";
import { motion, AnimatePresence } from "framer-motion";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ProviderIcon } from "@/components/ProviderIcon";
import {
  useAddProviderMutation,
  useSwitchProviderMutation,
} from "@/lib/query/mutations";
import { cn } from "@/lib/utils";
import {
  QUICKSTART_CATEGORY_LABEL_KEYS,
  QUICKSTART_CUSTOM_PRESET_ID,
  type QuickStartAppId,
} from "@/config/quickStartCurated";
import {
  buildQuickStartProviderInput,
  defaultAdvancedFields,
  getCuratedPresetGroups,
  type QuickStartFormFields,
  type QuickStartSelection,
  type QuickStartStep,
  type ResolvedQuickStartPreset,
  runQuickStartApplyPipeline,
} from "@/lib/quickStart";
import { QuickStartAppTabs, quickStartAppLabel } from "./QuickStartAppTabs";
import { QuickStartOfficialPanel } from "./QuickStartOfficialPanel";
import { QuickStartCustomFields } from "./QuickStartCustomFields";
import { QuickStartAdvancedPanel } from "./QuickStartAdvancedPanel";
import { QuickStartVerifyBlock } from "./QuickStartVerifyBlock";
import { QuickStartProviderList } from "./QuickStartProviderList";

interface QuickStartPageProps {
  onOpenSettings?: () => void;
}

const EMPTY_FIELDS: QuickStartFormFields = {
  apiKey: "",
  customName: "",
  customBaseUrl: "",
  customModel: "",
};

export function QuickStartPage({ onOpenSettings }: QuickStartPageProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const [activeApp, setActiveApp] = useState<QuickStartAppId>("claude");
  const [step, setStep] = useState<QuickStartStep>(1);
  const [searchQuery, setSearchQuery] = useState("");
  const [selection, setSelection] = useState<QuickStartSelection | null>(null);
  const [selectedPreset, setSelectedPreset] =
    useState<ResolvedQuickStartPreset | null>(null);
  const [fields, setFields] = useState<QuickStartFormFields>(EMPTY_FIELDS);
  const [applying, setApplying] = useState(false);
  const [lastTakeoverOk, setLastTakeoverOk] = useState(true);

  const addProviderMutation = useAddProviderMutation(activeApp);
  const switchProviderMutation = useSwitchProviderMutation(activeApp);

  const groupedPresets = useMemo(
    () => getCuratedPresetGroups(activeApp, searchQuery),
    [activeApp, searchQuery],
  );

  const resetWizard = useCallback(() => {
    setStep(1);
    setSelection(null);
    setSelectedPreset(null);
    setFields(EMPTY_FIELDS);
    setSearchQuery("");
  }, []);

  const handleAppChange = useCallback(
    (app: QuickStartAppId) => {
      if (app === activeApp) return;
      setActiveApp(app);
      resetWizard();
    },
    [activeApp, resetWizard],
  );

  const patchFields = useCallback((patch: Partial<QuickStartFormFields>) => {
    setFields((prev) => ({ ...prev, ...patch }));
  }, []);

  const handleSelectPreset = useCallback(
    (preset: ResolvedQuickStartPreset, isCustom: boolean) => {
      if (isCustom) {
        const sel: QuickStartSelection = { mode: "custom", appId: activeApp };
        setSelection(sel);
        setSelectedPreset(preset);
        setFields({
          ...EMPTY_FIELDS,
          ...defaultAdvancedFields(activeApp, sel),
        });
        setStep(2);
        return;
      }

      if (preset.isOfficial || preset.category === "official") {
        const sel: QuickStartSelection = {
          mode: "official",
          appId: activeApp,
          presetName: preset.name,
        };
        setSelection(sel);
        setSelectedPreset(preset);
        setStep(2);
        return;
      }

      const sel: QuickStartSelection = {
        mode: "preset",
        appId: activeApp,
        presetName: preset.name,
        isOfficial: false,
      };
      setSelection(sel);
      setSelectedPreset(preset);
      setFields({
        ...EMPTY_FIELDS,
        ...defaultAdvancedFields(activeApp, sel),
      });
      setStep(2);
    },
    [activeApp],
  );

  const handleBack = useCallback(() => {
    setStep(1);
    setSelection(null);
    setSelectedPreset(null);
    setFields(EMPTY_FIELDS);
  }, []);

  const validateBeforeApply = useCallback((): boolean => {
    if (!selection) return false;

    if (selection.mode === "official") {
      return false;
    }

    if (!fields.apiKey.trim()) {
      toast.error(t("quickStart.error.emptyKey", { defaultValue: "请填写 API Key" }));
      return false;
    }

    if (selection.mode === "custom") {
      if (!fields.customBaseUrl.trim()) {
        toast.error(
          t("quickStart.error.noBaseUrl", { defaultValue: "请填写 Base URL" }),
        );
        return false;
      }
      if (!fields.customModel.trim()) {
        toast.error(
          t("quickStart.error.noModel", { defaultValue: "请填写默认模型" }),
        );
        return false;
      }
    }

    return true;
  }, [selection, fields, t]);

  const handleApply = useCallback(async () => {
    if (!selection || !validateBeforeApply()) return;

    setApplying(true);
    try {
      const displayName =
        selection.mode === "custom"
          ? fields.customName.trim() ||
            t("quickStart.custom.defaultName", { defaultValue: "自定义供应商" })
          : selectedPreset?.nameKey
            ? String(t(selectedPreset.nameKey))
            : (selectedPreset?.name ?? "");

      const providerInput = buildQuickStartProviderInput(
        activeApp,
        selection,
        fields,
        displayName,
      );

      const { takeoverOk } = await runQuickStartApplyPipeline(
        {
          appId: activeApp,
          addProvider: (input) => addProviderMutation.mutateAsync(input),
          switchProvider: (id) => switchProviderMutation.mutateAsync(id),
          queryClient,
        },
        providerInput,
      );

      setLastTakeoverOk(takeoverOk);

      if (!takeoverOk) {
        toast.warning(
          t("quickStart.warn.takeoverFailed", {
            defaultValue:
              "供应商已添加，但本地路由开启失败。请在设置中手动开启并保持 OpenSunstar 运行",
          }),
        );
      } else {
        toast.success(
          t("quickStart.success", {
            defaultValue: "接入成功！已切换为当前供应商",
          }),
        );
      }

      setStep(3);
    } catch (error) {
      console.error("[QuickStart] apply failed:", error);
      toast.error(
        t("quickStart.error.failed", {
          defaultValue: "接入失败，请检查 API Key 或网络后重试",
        }),
      );
    } finally {
      setApplying(false);
    }
  }, [
    selection,
    validateBeforeApply,
    fields,
    selectedPreset,
    activeApp,
    addProviderMutation,
    switchProviderMutation,
    queryClient,
    t,
  ]);

  const step2Title =
    selectedPreset?.nameKey && selectedPreset.name !== QUICKSTART_CUSTOM_PRESET_ID
      ? String(t(selectedPreset.nameKey))
      : selectedPreset?.name === QUICKSTART_CUSTOM_PRESET_ID
        ? t("quickStart.custom.cardTitle", { defaultValue: "自定义配置" })
        : (selectedPreset?.name ?? "");

  const proxyHintKey =
    activeApp === "claude-desktop"
      ? "quickStart.proxyRunningHintDesktop"
      : "quickStart.proxyRunningHint";

  return (
    <div className="mx-auto max-w-4xl space-y-6 p-6">
      <div className="space-y-3">
        <div className="space-y-1">
          <h1 className="flex items-center gap-2 text-2xl font-bold">
            <Sparkles className="h-6 w-6 text-primary" />
            {t("quickStart.title", { defaultValue: "快速接入" })}
          </h1>
          <p className="text-sm text-muted-foreground">
            {t("quickStart.subtitle", {
              defaultValue: "三步完成接入：选供应商 → 填 Key → 一键启用",
            })}
          </p>
        </div>
        <QuickStartAppTabs activeApp={activeApp} onChange={handleAppChange} />
      </div>

      <div className="flex items-center gap-2">
        {([1, 2, 3] as QuickStartStep[]).map((s, idx) => (
          <div key={s} className="flex flex-1 items-center gap-2">
            <div
              className={cn(
                "flex h-8 w-8 shrink-0 items-center justify-center rounded-full text-sm font-medium transition-colors",
                step >= s
                  ? "bg-primary text-primary-foreground"
                  : "bg-muted text-muted-foreground",
              )}
            >
              {s}
            </div>
            {idx < 2 && (
              <div
                className={cn(
                  "h-0.5 flex-1 transition-colors",
                  step > s ? "bg-primary" : "bg-muted",
                )}
              />
            )}
          </div>
        ))}
      </div>

      <AnimatePresence mode="wait">
        {step === 1 && (
          <motion.div
            key={`step1-${activeApp}`}
            initial={{ opacity: 0, x: 20 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: -20 }}
            className="space-y-4"
          >
            <div className="relative">
              <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder={t("quickStart.searchPlaceholder", {
                  defaultValue: "搜索供应商名称或网址…",
                })}
                className="pl-9"
              />
            </div>

            <div className="max-h-[60vh] space-y-6 overflow-y-auto pr-2">
              {groupedPresets.map((group) => (
                <div key={group.category} className="space-y-2">
                  <h3 className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                    {t(
                      QUICKSTART_CATEGORY_LABEL_KEYS[group.category] ??
                        "providerPreset.other",
                      { defaultValue: group.category },
                    )}
                  </h3>
                  {group.emptyHintKey && group.presets.length === 0 && (
                    <p className="text-xs text-muted-foreground rounded-md border border-dashed border-border p-3">
                      {t(group.emptyHintKey)}
                    </p>
                  )}
                  <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
                    {group.presets.map((preset) => (
                      <PresetCard
                        key={`${group.category}-${preset.name}`}
                        preset={preset}
                        isCustom={group.isCustomGroup}
                        onSelect={() =>
                          handleSelectPreset(preset, Boolean(group.isCustomGroup))
                        }
                      />
                    ))}
                  </div>
                </div>
              ))}
            </div>
          </motion.div>
        )}

        {step === 2 && selection && selectedPreset && (
          <motion.div
            key="step2"
            initial={{ opacity: 0, x: 20 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0, x: -20 }}
            className="space-y-6"
          >
            {selection.mode === "official" ? (
              <QuickStartOfficialPanel
                preset={selectedPreset}
                onBack={handleBack}
                onOpenSettings={onOpenSettings}
              />
            ) : (
              <>
                <div className="flex items-center gap-3 rounded-lg border border-border bg-card p-4">
                  <ProviderIcon
                    icon={selectedPreset.icon}
                    name={selectedPreset.name}
                    color={selectedPreset.iconColor}
                    size={40}
                  />
                  <div className="flex-1">
                    <p className="font-semibold">{step2Title}</p>
                    {selectedPreset.websiteUrl && (
                      <a
                        href={selectedPreset.websiteUrl}
                        target="_blank"
                        rel="noreferrer"
                        className="inline-flex items-center gap-1 text-xs text-primary hover:underline"
                      >
                        {selectedPreset.websiteUrl}
                        <ExternalLink className="h-3 w-3" />
                      </a>
                    )}
                  </div>
                  <Button variant="ghost" size="sm" onClick={handleBack}>
                    {t("common.back", { defaultValue: "返回" })}
                  </Button>
                </div>

                {selection.mode === "custom" && (
                  <QuickStartCustomFields
                    appId={activeApp}
                    fields={fields}
                    onChange={patchFields}
                  />
                )}

                <div className="space-y-3">
                  <Label className="flex items-center gap-2">
                    <KeyRound className="h-4 w-4" />
                    {t("quickStart.apiKeyLabel", { defaultValue: "API Key" })}
                  </Label>
                  <Input
                    type="password"
                    value={fields.apiKey}
                    onChange={(e) => patchFields({ apiKey: e.target.value })}
                    placeholder={t("quickStart.apiKeyPlaceholder", {
                      defaultValue: "粘贴你的 API Key",
                    })}
                    className="font-mono"
                    autoFocus
                  />
                  <p className="text-xs text-muted-foreground">
                    {t("quickStart.keychainHint", {
                      defaultValue:
                        "Key 将安全存储于系统钥匙串，不会明文落盘",
                    })}
                  </p>
                  {selectedPreset.apiKeyUrl && (
                    <a
                      href={selectedPreset.apiKeyUrl}
                      target="_blank"
                      rel="noreferrer"
                      className="inline-flex items-center gap-1 text-xs text-primary hover:underline"
                    >
                      {t("quickStart.getKey", { defaultValue: "获取 API Key" })}
                      <ExternalLink className="h-3 w-3" />
                    </a>
                  )}
                </div>

                <QuickStartVerifyBlock
                  appId={activeApp}
                  selection={selection}
                  fields={fields}
                />

                <QuickStartAdvancedPanel
                  appId={activeApp}
                  selection={selection}
                  fields={fields}
                  onChange={patchFields}
                />

                <div className="flex justify-end gap-2">
                  <Button variant="outline" onClick={handleBack} disabled={applying}>
                    {t("common.back", { defaultValue: "返回" })}
                  </Button>
                  <Button
                    onClick={handleApply}
                    disabled={applying || !fields.apiKey.trim()}
                  >
                    {applying ? (
                      <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    ) : (
                      <Zap className="mr-2 h-4 w-4" />
                    )}
                    {t("quickStart.apply", { defaultValue: "一键启用" })}
                  </Button>
                </div>
              </>
            )}
          </motion.div>
        )}

        {step === 3 && (
          <motion.div
            key="step3"
            initial={{ opacity: 0, scale: 0.95 }}
            animate={{ opacity: 1, scale: 1 }}
            className="flex flex-col items-center justify-center gap-4 py-8 text-center"
          >
            <div className="flex h-16 w-16 items-center justify-center rounded-full bg-green-100 dark:bg-green-900/30">
              <Sparkles className="h-8 w-8 text-green-600 dark:text-green-400" />
            </div>
            <h2 className="text-xl font-semibold">
              {t("quickStart.doneTitle", { defaultValue: "接入成功！" })}
            </h2>
            <p className="max-w-md text-sm text-muted-foreground">
              {t("quickStart.doneDescApp", {
                app: quickStartAppLabel(activeApp, t),
                defaultValue: `${quickStartAppLabel(activeApp, t)} 已配置并切换为当前供应商。`,
              })}
            </p>

            {lastTakeoverOk &&
              (activeApp === "claude" ||
                activeApp === "codex" ||
                activeApp === "gemini" ||
                activeApp === "claude-desktop") && (
                <div className="flex items-center gap-2 rounded-lg border border-amber-200 bg-amber-50 px-4 py-2 text-xs text-amber-800 dark:border-amber-900 dark:bg-amber-950/30 dark:text-amber-300">
                  <span className="text-base">⚠️</span>
                  <span className="text-left">
                    {t(proxyHintKey, {
                      defaultValue:
                        "请保持 OpenSunstar 运行，否则 CLI 将无法连接本地代理。",
                    })}
                  </span>
                </div>
              )}

            <QuickStartProviderList appId={activeApp} />

            <div className="flex gap-2 pt-2">
              <Button variant="outline" onClick={resetWizard}>
                {t("quickStart.addAnother", { defaultValue: "再接入一个" })}
              </Button>
              {onOpenSettings && (
                <Button variant="outline" onClick={onOpenSettings}>
                  {t("quickStart.openSettings", {
                    defaultValue: "前往供应商管理",
                  })}
                </Button>
              )}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}

interface PresetCardProps {
  preset: ResolvedQuickStartPreset;
  isCustom?: boolean;
  onSelect: () => void;
}

function PresetCard({ preset, isCustom, onSelect }: PresetCardProps) {
  const { t } = useTranslation();
  const displayName = isCustom
    ? t("quickStart.custom.cardTitle", { defaultValue: "自定义配置" })
    : preset.nameKey
      ? t(preset.nameKey)
      : preset.name;

  return (
    <button
      type="button"
      onClick={onSelect}
      className={cn(
        "group flex items-center gap-3 rounded-lg border border-border bg-card p-3 text-left transition-all",
        "hover:border-primary/50 hover:shadow-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
        isCustom && "border-dashed",
      )}
    >
      {isCustom ? (
        <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-md bg-muted">
          <PlusCircle className="h-5 w-5 text-muted-foreground" />
        </div>
      ) : (
        <ProviderIcon
          icon={preset.icon}
          name={preset.name}
          color={preset.iconColor}
          size={32}
          className="shrink-0"
        />
      )}
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium">{displayName}</p>
        {preset.isOfficial && !isCustom && (
          <span className="text-[10px] text-green-600 dark:text-green-400">
            {t("quickStart.official", { defaultValue: "官方" })}
          </span>
        )}
        {isCustom && (
          <span className="text-[10px] text-muted-foreground">
            {t("quickStart.custom.cardSubtitle", {
              defaultValue: "自填 Base URL 与模型",
            })}
          </span>
        )}
      </div>
    </button>
  );
}
