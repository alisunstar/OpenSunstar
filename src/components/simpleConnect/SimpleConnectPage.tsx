import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Loader2, Sparkles, Wrench, Zap } from "lucide-react";
import { motion, AnimatePresence } from "framer-motion";
import { toast } from "sonner";
import { ConfirmDialog } from "@/components/ConfirmDialog";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  simpleConnectApi,
  type SimpleConnectState,
  type SupplierProfile,
  type ToolConfigStatus,
} from "@/lib/api/simpleConnect";
import { KeyPoolPanel } from "./KeyPoolPanel";
import { ExpertProviderPanel } from "./ExpertProviderPanel";
import { WizardStepper } from "./WizardStepper";
import { ConnectSummaryBar } from "./ConnectSummaryBar";
import { SupplierGrid } from "./SupplierGrid";
import { CliToolGrid } from "./CliToolGrid";
import { TOOL_LABELS } from "./constants";
import { SimpleConnectImportDialog } from "./SimpleConnectImportDialog";
import { Step3DetailAccordion } from "./Step3DetailAccordion";
import { UsageSummaryPanel } from "./UsageSummaryPanel";
import { SecurityPrivacyNotice } from "./SecurityPrivacyNotice";
import { SC_PANEL, SC_STEP, SectionHeader } from "./ui";
import { Cpu, Store } from "lucide-react";

interface SimpleConnectPageProps {
  onOpenSettings?: () => void;
}

export function SimpleConnectPage({ onOpenSettings }: SimpleConnectPageProps) {
  const { t } = useTranslation();
  const [tab, setTab] = useState("simple");
  const [step, setStep] = useState(1);
  const [loading, setLoading] = useState(true);
  const [suppliers, setSuppliers] = useState<SupplierProfile[]>([]);
  const [tools, setTools] = useState<string[]>([]);
  const [state, setState] = useState<SimpleConnectState | null>(null);
  const [models, setModels] = useState<string[]>([]);
  const [modelsLoading, setModelsLoading] = useState(false);
  const [selectedModel, setSelectedModel] = useState("");
  const [selectedTool, setSelectedTool] = useState("claude-code");
  const [customBase, setCustomBase] = useState("");
  const [applying, setApplying] = useState(false);
  const [statusRefresh, setStatusRefresh] = useState(0);
  const [keyReady, setKeyReady] = useState(false);
  const [primaryKeyHint, setPrimaryKeyHint] = useState<string | null>(null);
  const [toolStatuses, setToolStatuses] = useState<ToolConfigStatus[]>([]);
  const [supplierConfirmOpen, setSupplierConfirmOpen] = useState(false);
  const [pendingSupplierId, setPendingSupplierId] = useState<string | null>(
    null,
  );

  const selectedSupplier = useMemo(
    () => suppliers.find((s) => s.id === state?.supplier_id),
    [suppliers, state?.supplier_id],
  );

  const supplierLabel = useMemo(() => {
    if (state?.supplier_id === "custom") {
      return t("simpleConnect.customSupplier", {
        defaultValue: "自定义 OpenAI 兼容",
      });
    }
    return (
      selectedSupplier?.name ??
      state?.supplier_id ??
      t("simpleConnect.supplier", { defaultValue: "供应商" })
    );
  }, [state?.supplier_id, selectedSupplier?.name, t]);

  const configuredTools = useMemo(
    () =>
      new Set(
        toolStatuses.filter((s) => s.configured).map((s) => s.tool),
      ),
    [toolStatuses],
  );

  const loadToolStatuses = useCallback(async () => {
    try {
      const list = await simpleConnectApi.listToolStatus();
      setToolStatuses(list);
    } catch {
      /* optional */
    }
  }, []);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [supplierList, toolList, saved, statuses] = await Promise.all([
        simpleConnectApi.listSuppliers(),
        simpleConnectApi.listTools(),
        simpleConnectApi.getState(),
        simpleConnectApi.listToolStatus(),
      ]);
      setSuppliers(supplierList);
      setTools(toolList);
      setState(saved);
      setToolStatuses(statuses);
      setCustomBase(saved.custom_openai_base ?? "");
      setSelectedModel(saved.last_model ?? "");
      setSelectedTool(saved.last_tool ?? toolList[0] ?? "claude-code");
      const configured = await simpleConnectApi.keyConfigured(saved.supplier_id);
      setKeyReady(configured);
    } catch (e) {
      toast.error(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const persistState = async (next: SimpleConnectState) => {
    setState(next);
    await simpleConnectApi.saveState(next);
  };

  const verifyBeforeStore = async (secret: string) => {
    if (!state?.require_key_verify) return;
    const result = await simpleConnectApi.verifyKey(
      state.supplier_id,
      secret,
      state.supplier_id === "custom" ? customBase : undefined,
    );
    if (!result.ok) {
      throw new Error(result.error ?? "Key 校验失败");
    }
  };

  const handleSupplierSelect = async (supplierId: string) => {
    if (supplierId === "custom") {
      setState((prev) =>
        prev ? { ...prev, supplier_id: "custom" } : prev,
      );
      return;
    }
    const next = await simpleConnectApi.setSupplier(supplierId);
    setState(next);
    setCustomBase("");
    const configured = await simpleConnectApi.keyConfigured(next.supplier_id);
    setKeyReady(configured);
    setPrimaryKeyHint(null);
  };

  const needsSupplierConfirm = (nextId: string) => {
    if (!state || nextId === state.supplier_id) return false;
    if (keyReady) return true;
    if (configuredTools.size > 0) return true;
    if (
      state.last_applied_supplier_id &&
      state.last_applied_supplier_id !== nextId
    ) {
      return true;
    }
    return false;
  };

  const requestSupplierSelect = (supplierId: string) => {
    if (supplierId === "custom") {
      setState((prev) =>
        prev ? { ...prev, supplier_id: "custom" } : prev,
      );
      return;
    }
    if (needsSupplierConfirm(supplierId)) {
      setPendingSupplierId(supplierId);
      setSupplierConfirmOpen(true);
      return;
    }
    void handleSupplierSelect(supplierId);
  };

  const confirmSupplierChange = () => {
    if (pendingSupplierId) {
      void handleSupplierSelect(pendingSupplierId);
    }
    setPendingSupplierId(null);
    setSupplierConfirmOpen(false);
  };

  const handleFetchModels = useCallback(
    async (silent = false) => {
      if (!state) return;
      if (state.supplier_id === "custom" && !customBase.trim()) {
        if (!silent) {
          toast.error(
            t("simpleConnect.customBaseRequired", {
              defaultValue: "请先填写自定义 API Base URL",
            }),
          );
        }
        return;
      }
      setModelsLoading(true);
      try {
        const list = await simpleConnectApi.fetchModels(
          state.supplier_id,
          state.supplier_id === "custom" ? customBase : undefined,
        );
        setModels(list);
        const fallback = selectedSupplier?.default_model;
        if (list.length) {
          setSelectedModel((prev) =>
            prev && list.includes(prev) ? prev : list[0],
          );
        } else if (fallback) {
          setSelectedModel(fallback);
          setModels([fallback]);
        }
        if (!silent) {
          toast.success(
            t("simpleConnect.modelsLoaded", {
              count: list.length || (fallback ? 1 : 0),
              defaultValue: "已加载 {{count}} 个模型",
            }),
          );
        }
      } catch (e) {
        const fallback = selectedSupplier?.default_model;
        if (fallback) {
          setModels([fallback]);
          setSelectedModel((prev) => prev || fallback);
          if (!silent) {
            toast.warning(
              t("simpleConnect.modelsFallback", {
                model: fallback,
                defaultValue: "拉取失败，已使用预设模型 {{model}}",
              }),
            );
          }
        } else if (!silent) {
          toast.error(String(e));
        }
      } finally {
        setModelsLoading(false);
      }
    },
    [state, customBase, selectedSupplier?.default_model, t],
  );

  useEffect(() => {
    if (step !== 3 || !state) return;
    void handleFetchModels(true);
  }, [step, state, handleFetchModels]);

  const handleGoToStep2 = async () => {
    if (state?.supplier_id === "custom") {
      if (!customBase.trim()) {
        toast.error(
          t("simpleConnect.customBaseRequired", {
            defaultValue: "请先填写自定义 API Base URL",
          }),
        );
        return;
      }
      const next = await simpleConnectApi.setSupplier("custom", customBase);
      setState(next);
    }
    setStep(2);
  };

  const handleGoToStep3 = async () => {
    if (!state) return;
    const configured = await simpleConnectApi.keyConfigured(state.supplier_id);
    if (!configured) {
      toast.error(
        t("simpleConnect.keyRequired", {
          defaultValue: "请先在 Keychain 中保存 API Key",
        }),
      );
      return;
    }
    setKeyReady(true);
    setStep(3);
  };

  const handleApply = async () => {
    if (!state || !selectedModel) return;
    setApplying(true);
    try {
      const result = await simpleConnectApi.apply({
        tool: selectedTool,
        supplierId: state.supplier_id,
        model: selectedModel,
        customBase: state.supplier_id === "custom" ? customBase : undefined,
        usePool: state.pool_enabled,
      });
      await persistState({
        ...state,
        last_model: selectedModel,
        last_tool: selectedTool,
      });
      setStatusRefresh((n) => n + 1);
      await loadToolStatuses();
      toast.success(
        t("simpleConnect.applySuccess", {
          tool: TOOL_LABELS[selectedTool] ?? selectedTool,
          defaultValue: "{{tool}} 配置已应用",
        }),
      );
      if (
        state.last_applied_supplier_id &&
        state.last_applied_supplier_id !== state.supplier_id
      ) {
        toast.info(
          t("simpleConnect.supplierChangedApply", {
            defaultValue: "已切换供应商并写入 CLI；原配置已备份",
          }),
        );
      }
      if (result.proxy_port) {
        toast.info(
          t("simpleConnect.poolProxy", {
            port: result.proxy_port,
            defaultValue: "本地代理 :{{port}} 已启用",
          }),
        );
      }
    } catch (e) {
      toast.error(String(e));
    } finally {
      setApplying(false);
    }
  };

  const handleStepClick = (target: number) => {
    if (target < step) setStep(target);
  };

  if (loading || !state) {
    return (
      <div className="flex flex-1 items-center justify-center">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="flex flex-col flex-1 min-h-0 px-4 sm:px-6 pb-6">
      <SimpleConnectImportDialog
        onImported={(next) => {
          if (next) setState(next);
          void load();
        }}
      />
      <ConfirmDialog
        isOpen={supplierConfirmOpen}
        variant="destructive"
        title={t("simpleConnect.supplierChangeTitle", {
          defaultValue: "切换供应商？",
        })}
        message={t("simpleConnect.supplierChangeBody", {
          defaultValue:
            "切换后将使用新的 upstream；已保存的 Key 仍对应原供应商命名空间。应用配置时会自动备份并重写 CLI。",
        })}
        confirmText={t("simpleConnect.supplierChangeConfirm", {
          defaultValue: "继续切换",
        })}
        onConfirm={confirmSupplierChange}
        onCancel={() => {
          setSupplierConfirmOpen(false);
          setPendingSupplierId(null);
        }}
      />
      <div className="mx-auto w-full max-w-3xl flex flex-col flex-1 min-h-0">
        <Tabs value={tab} onValueChange={setTab} className="flex flex-col flex-1 min-h-0">
          <TabsList className="grid w-full max-w-md grid-cols-2 mb-5">
            <TabsTrigger value="simple" className="gap-2">
              <Sparkles className="h-4 w-4" />
              {t("simpleConnect.tabSimple", { defaultValue: "快速接入" })}
            </TabsTrigger>
            <TabsTrigger value="expert" className="gap-2">
              <Wrench className="h-4 w-4" />
              {t("simpleConnect.tabExpert", { defaultValue: "高级 Provider" })}
            </TabsTrigger>
          </TabsList>

          <SecurityPrivacyNotice className="mb-4" />

          <TabsContent value="simple" className="flex-1 overflow-y-auto mt-0 space-y-5">
            <div className={`${SC_PANEL} p-5 sm:p-6 space-y-3`}>
              <div className="flex items-start gap-3">
                <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-primary/15 text-primary">
                  <Zap className="h-5 w-5" />
                </div>
                <div>
                  <h2 className="text-lg font-semibold tracking-tight">
                    {t("simpleConnect.title", { defaultValue: "Simple Connect" })}
                  </h2>
                  <p className="text-sm text-muted-foreground mt-1">
                    {t("simpleConnect.subtitle", {
                      defaultValue:
                        "3 步完成：选供应商 → 保存 Key → 选 CLI 并应用（Keychain + 本地代理 + 六 CLI）",
                    })}
                  </p>
                </div>
              </div>
            </div>

            <WizardStepper step={step} onStepClick={handleStepClick} />

            <ConnectSummaryBar
              supplierLabel={supplierLabel}
              keyReady={keyReady}
              keyHint={primaryKeyHint}
              poolEnabled={state.pool_enabled}
              poolKeyCount={state.pool_keys.filter((k) => k.enabled).length}
              configuredCliCount={configuredTools.size}
              totalCliCount={tools.length}
              currentStep={step}
            />

            <AnimatePresence mode="wait">
              <motion.div
                key={step}
                initial={{ opacity: 0, y: 8 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -8 }}
                transition={{ duration: 0.2 }}
                className="space-y-4"
              >
                {step === 1 && (
                  <section className={SC_STEP}>
                    <SectionHeader
                      icon={Store}
                      title={t("simpleConnect.pickSupplier", {
                        defaultValue: "选择供应商",
                      })}
                      description={t("simpleConnect.customSupplierHint", {
                        defaultValue: "任意 OpenAI-compatible Base URL",
                      })}
                    />
                    <SupplierGrid
                      suppliers={suppliers}
                      selectedId={state.supplier_id}
                      customBase={customBase}
                      onSelect={requestSupplierSelect}
                      onCustomBaseChange={setCustomBase}
                    />
                  </section>
                )}

                {step === 2 && (
                  <section className="space-y-4">
                    <KeyPoolPanel
                      supplierId={state.supplier_id}
                      poolEnabled={state.pool_enabled}
                      keys={state.pool_keys}
                      primaryKeyHint={primaryKeyHint}
                      requireKeyVerify={state.require_key_verify ?? true}
                      deeplinkImportEnabled={state.deeplink_import_enabled ?? true}
                      onPoolEnabledChange={(enabled) =>
                        void persistState({ ...state, pool_enabled: enabled })
                      }
                      onRequireKeyVerifyChange={(enabled) =>
                        void persistState({ ...state, require_key_verify: enabled })
                      }
                      onDeeplinkImportEnabledChange={(enabled) =>
                        void persistState({
                          ...state,
                          deeplink_import_enabled: enabled,
                        })
                      }
                      onKeysChange={(keys) =>
                        void persistState({ ...state, pool_keys: keys })
                      }
                      onStoreKey={async (keyId, secret) => {
                        await verifyBeforeStore(secret);
                        if (keyId === "primary") {
                          const hint = await simpleConnectApi.storeKey(
                            state.supplier_id,
                            secret,
                          );
                          setPrimaryKeyHint(hint);
                          setKeyReady(true);
                          return hint;
                        }
                        return simpleConnectApi.storePoolKey(
                          state.supplier_id,
                          keyId,
                          secret,
                        );
                      }}
                      onRemoveKey={(keyId) =>
                        simpleConnectApi.removePoolKey(state.supplier_id, keyId)
                      }
                    />
                  </section>
                )}

                {step === 3 && (
                  <section className={SC_STEP}>
                    <SectionHeader
                      icon={Cpu}
                      title={t("simpleConnect.pickCli", {
                        defaultValue: "选择要配置的 CLI",
                      })}
                      description={t("simpleConnect.step3Hint", {
                        defaultValue: "选择 CLI 与模型后应用；详情见下方折叠面板",
                      })}
                    />
                    <CliToolGrid
                      tools={tools}
                      selectedTool={selectedTool}
                      configuredTools={configuredTools}
                      onSelect={setSelectedTool}
                    />

                    <div className="space-y-2">
                      <div className="flex items-center justify-between gap-2">
                        <Label>{t("simpleConnect.model", { defaultValue: "模型" })}</Label>
                        <Button
                          type="button"
                          variant="ghost"
                          size="sm"
                          className="h-8"
                          disabled={modelsLoading}
                          onClick={() => void handleFetchModels()}
                        >
                          {modelsLoading && (
                            <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />
                          )}
                          {t("simpleConnect.fetchModels", {
                            defaultValue: "拉取模型",
                          })}
                        </Button>
                      </div>
                      <Select
                        value={selectedModel}
                        onValueChange={setSelectedModel}
                        disabled={!models.length && modelsLoading}
                      >
                        <SelectTrigger>
                          <SelectValue
                            placeholder={
                              modelsLoading
                                ? t("simpleConnect.modelsLoading", {
                                    defaultValue: "正在拉取模型…",
                                  })
                                : t("simpleConnect.modelPlaceholder", {
                                    defaultValue: "先拉取模型列表",
                                  })
                            }
                          />
                        </SelectTrigger>
                        <SelectContent>
                          {models.map((m) => (
                            <SelectItem key={m} value={m}>
                              {m}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    </div>

                    <Step3DetailAccordion
                      poolEnabled={state.pool_enabled}
                      statusRefresh={statusRefresh}
                      selectedTool={selectedTool}
                      onSelectTool={setSelectedTool}
                    />
                  </section>
                )}
              </motion.div>
            </AnimatePresence>

            <div className="sticky bottom-0 -mx-1 flex flex-wrap gap-2 border-t border-border/40 bg-background/80 backdrop-blur-sm py-4 px-1">
              {step > 1 && (
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => setStep(step - 1)}
                >
                  {t("common.back", { defaultValue: "上一步" })}
                </Button>
              )}
              {step === 1 && (
                <Button type="button" className="ml-auto" onClick={() => void handleGoToStep2()}>
                  {t("common.next", { defaultValue: "下一步" })}
                </Button>
              )}
              {step === 2 && (
                <Button
                  type="button"
                  className="ml-auto"
                  disabled={!keyReady}
                  onClick={() => void handleGoToStep3()}
                >
                  {t("common.next", { defaultValue: "下一步" })}
                </Button>
              )}
              {step === 3 && (
                <Button
                  type="button"
                  className="ml-auto gap-2"
                  disabled={applying || !selectedModel}
                  onClick={() => void handleApply()}
                >
                  {applying && (
                    <Loader2 className="h-4 w-4 animate-spin" />
                  )}
                  {t("simpleConnect.applyTo", {
                    tool: TOOL_LABELS[selectedTool] ?? selectedTool,
                    defaultValue: "应用到 {{tool}}",
                  })}
                </Button>
              )}
            </div>
          </TabsContent>

          <TabsContent value="expert" className="flex-1 overflow-y-auto mt-0 space-y-4">
            <ExpertProviderPanel
              onSwitchToSimple={() => setTab("simple")}
              onOpenSettings={onOpenSettings}
            />
            <UsageSummaryPanel />
          </TabsContent>
        </Tabs>
      </div>
    </div>
  );
}
