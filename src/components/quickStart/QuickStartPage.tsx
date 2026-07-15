import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQueryClient } from "@tanstack/react-query";
import {
  CheckCircle2,
  CircleAlert,
  ClipboardList,
  ExternalLink,
  KeyRound,
  Loader2,
  Plus,
  PlusCircle,
  Search,
  Settings2,
  Sparkles,
  Undo2,
  X,
  Zap,
} from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { ProviderIcon } from "@/components/ProviderIcon";
import { cn } from "@/lib/utils";
import {
  QUICKSTART_CATEGORY_LABEL_KEYS,
  QUICKSTART_CUSTOM_PRESET_ID,
  type QuickStartAppId,
} from "@/config/quickStartCurated";
import {
  buildQuickStartProviderInput,
  createQuickStartAttemptIdentity,
  defaultAdvancedFields,
  getQuickStartOperationEvents,
  getCuratedPresetGroups,
  listRecentQuickStartOperations,
  listRecoverableQuickStartOperations,
  rollbackQuickStartOperation,
  runQuickStartApplyPipeline,
  type QuickStartAttemptIdentity,
  type QuickStartFormFields,
  type QuickStartOperation,
  type QuickStartOperationEvent,
  type QuickStartSelection,
  type ResolvedQuickStartPreset,
} from "@/lib/quickStart";
import { QuickStartAdvancedPanel } from "./QuickStartAdvancedPanel";
import { QuickStartAppTabs, quickStartAppLabel } from "./QuickStartAppTabs";
import { QuickStartCustomFields } from "./QuickStartCustomFields";
import { QuickStartProviderList } from "./QuickStartProviderList";
import { QuickStartVerifyBlock } from "./QuickStartVerifyBlock";

interface QuickStartPageProps {
  onOpenSettings?: () => void;
}

const EMPTY_FIELDS: QuickStartFormFields = {
  apiKey: "",
  customName: "",
  customBaseUrl: "",
  customModel: "",
};

const RECOVERABLE_LABELS: Record<string, string> = {
  pending: "等待开始",
  applying: "正在应用配置",
  verifying: "正在验证配置",
  rolling_back: "正在恢复配置",
  rollback_failed: "恢复需要继续处理",
};

function formatUpstreamVerificationReceipt(
  event: QuickStartOperationEvent,
): string | null {
  if (
    event.eventType !== "upstream_verification_succeeded" ||
    !event.detailJson
  ) {
    return null;
  }
  try {
    const detail: unknown = JSON.parse(event.detailJson);
    if (!detail || typeof detail !== "object") return null;
    const { protocol, endpointHost, modelCount } = detail as Record<
      string,
      unknown
    >;
    if (
      typeof protocol !== "string" ||
      typeof endpointHost !== "string" ||
      typeof modelCount !== "number"
    ) {
      return null;
    }
    return `上游验证：${protocol} · ${endpointHost} · ${modelCount} 个模型`;
  } catch {
    return null;
  }
}

export function QuickStartPage({ onOpenSettings }: QuickStartPageProps) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [activeApp, setActiveApp] = useState<QuickStartAppId>("claude");
  const [searchQuery, setSearchQuery] = useState("");
  const [isAddProviderOpen, setIsAddProviderOpen] = useState(false);
  const [selection, setSelection] = useState<QuickStartSelection | null>(null);
  const [selectedPreset, setSelectedPreset] =
    useState<ResolvedQuickStartPreset | null>(null);
  const [fields, setFields] = useState<QuickStartFormFields>(EMPTY_FIELDS);
  const [connectivityVerified, setConnectivityVerified] = useState(false);
  const [applying, setApplying] = useState(false);
  const [lastOperation, setLastOperation] =
    useState<QuickStartOperation | null>(null);
  const [recoverableOperations, setRecoverableOperations] = useState<
    QuickStartOperation[]
  >([]);
  const [recentOperations, setRecentOperations] = useState<QuickStartOperation[]>(
    [],
  );
  const [auditEvents, setAuditEvents] = useState<QuickStartOperationEvent[]>(
    [],
  );
  const [auditOperationId, setAuditOperationId] = useState<string | null>(null);
  const attemptIdentityRef = useRef<QuickStartAttemptIdentity | null>(null);

  const groupedPresets = useMemo(
    () => getCuratedPresetGroups(activeApp, searchQuery),
    [activeApp, searchQuery],
  );
  const officialGroups = useMemo(
    () => groupedPresets.filter((group) => group.category === "official"),
    [groupedPresets],
  );
  const addProviderGroups = useMemo(
    () => groupedPresets.filter((group) => group.category !== "official"),
    [groupedPresets],
  );

  const refreshRecoverableOperations = useCallback(async () => {
    const operations = await listRecoverableQuickStartOperations();
    setRecoverableOperations(operations);
  }, []);

  const refreshRecentOperations = useCallback(async () => {
    const operations = await listRecentQuickStartOperations();
    setRecentOperations(operations);
  }, []);

  useEffect(() => {
    void refreshRecoverableOperations().catch((error) => {
      console.error(
        "[QuickStart] failed to load recoverable operations:",
        error,
      );
    });
    void refreshRecentOperations().catch((error) => {
      console.error("[QuickStart] failed to load operation history:", error);
    });
    const timer = window.setInterval(() => {
      void refreshRecoverableOperations().catch(() => undefined);
      void refreshRecentOperations().catch(() => undefined);
    }, 5000);
    return () => window.clearInterval(timer);
  }, [refreshRecentOperations, refreshRecoverableOperations]);

  const handleShowAudit = useCallback(async (operationId: string) => {
    const events = await getQuickStartOperationEvents(operationId);
    setAuditOperationId(operationId);
    setAuditEvents(events);
  }, []);

  const resetForm = useCallback(() => {
    setSelection(null);
    setSelectedPreset(null);
    setFields(EMPTY_FIELDS);
    setConnectivityVerified(false);
    attemptIdentityRef.current = null;
  }, []);

  const handleAppChange = useCallback(
    (app: QuickStartAppId) => {
      if (app === activeApp) return;
      setActiveApp(app);
      setSearchQuery("");
      setIsAddProviderOpen(false);
      resetForm();
    },
    [activeApp, resetForm],
  );

  const handleOpenAddProvider = useCallback(() => {
    setSearchQuery("");
    resetForm();
    setIsAddProviderOpen(true);
  }, [resetForm]);

  const handleAddProviderOpenChange = useCallback(
    (open: boolean) => {
      if (applying) return;
      setIsAddProviderOpen(open);
      if (!open) {
        setSearchQuery("");
        resetForm();
      }
    },
    [applying, resetForm],
  );

  const patchFields = useCallback((patch: Partial<QuickStartFormFields>) => {
    setFields((previous) => ({ ...previous, ...patch }));
    setConnectivityVerified(false);
  }, []);

  const handleSelectPreset = useCallback(
    (preset: ResolvedQuickStartPreset, isCustom: boolean) => {
      const nextSelection: QuickStartSelection = isCustom
        ? { mode: "custom", appId: activeApp }
        : preset.isOfficial || preset.category === "official"
          ? { mode: "official", appId: activeApp, presetName: preset.name }
          : {
              mode: "preset",
              appId: activeApp,
              presetName: preset.name,
              isOfficial: false,
            };
      setSelection(nextSelection);
      setSelectedPreset(preset);
      setFields({
        ...EMPTY_FIELDS,
        ...defaultAdvancedFields(activeApp, nextSelection),
      });
      setConnectivityVerified(false);
      attemptIdentityRef.current = null;
    },
    [activeApp],
  );

  const validateBeforeApply = useCallback((): boolean => {
    if (!selection || selection.mode === "official") return false;
    if (!fields.apiKey.trim()) {
      toast.error(
        t("quickStart.error.emptyKey", { defaultValue: "请填写 API Key" }),
      );
      return false;
    }
    if (
      selection.mode === "custom" &&
      (!fields.customBaseUrl.trim() || !fields.customModel.trim())
    ) {
      toast.error(
        t("quickStart.error.customRequired", {
          defaultValue: "请填写 Base URL 和默认模型",
        }),
      );
      return false;
    }
    return true;
  }, [fields, selection, t]);

  const invalidateAppState = useCallback(
    async (appId: QuickStartAppId) => {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ["providers", appId] }),
        queryClient.invalidateQueries({ queryKey: ["proxyStatus"] }),
        queryClient.invalidateQueries({ queryKey: ["proxyTakeover"] }),
      ]);
    },
    [queryClient],
  );

  const handleApply = useCallback(async () => {
    if (!selection || !validateBeforeApply()) return;
    if (!connectivityVerified) {
      toast.error(
        t("quickStart.error.verifyRequired", {
          defaultValue: "请先验证 API Key 和上游连接后再启用",
        }),
      );
      return;
    }
    setApplying(true);
    try {
      const displayName =
        selection.mode === "custom"
          ? fields.customName.trim() ||
            t("quickStart.custom.defaultName", { defaultValue: "自定义供应商" })
          : selectedPreset?.nameKey
            ? String(t(selectedPreset.nameKey))
            : (selectedPreset?.name ?? "供应商");
      const providerInput = buildQuickStartProviderInput(
        activeApp,
        selection,
        fields,
        displayName,
      );
      const identity =
        attemptIdentityRef.current ?? createQuickStartAttemptIdentity();
      attemptIdentityRef.current = identity;

      const { operation } = await runQuickStartApplyPipeline(
        { appId: activeApp, queryClient },
        providerInput,
        identity,
      );
      setLastOperation(operation);
      attemptIdentityRef.current = null;
      await refreshRecoverableOperations();
      await refreshRecentOperations();

      if (operation.status === "succeeded") {
        setIsAddProviderOpen(false);
        toast.success(
          t("quickStart.success", { defaultValue: "供应商已连接" }),
        );
      } else {
        toast.error(
          t("quickStart.error.compensated", {
            defaultValue: "接入未完成，系统已尝试恢复到操作前状态",
          }),
        );
      }
    } catch (error) {
      console.error("[QuickStart] apply failed:", error);
      toast.error(
        t("quickStart.error.failed", {
          defaultValue: "接入失败，请检查 API Key 和网络后重试",
        }),
      );
    } finally {
      setApplying(false);
    }
  }, [
    activeApp,
    connectivityVerified,
    fields,
    queryClient,
    refreshRecentOperations,
    refreshRecoverableOperations,
    selectedPreset,
    selection,
    t,
    validateBeforeApply,
  ]);

  const handleRecover = useCallback(
    async (operation: QuickStartOperation) => {
      setApplying(true);
      try {
        const recovered = await rollbackQuickStartOperation(operation);
        setLastOperation(recovered);
        await invalidateAppState(operation.appType);
        await refreshRecoverableOperations();
        await refreshRecentOperations();
        toast.success(
          t("quickStart.rollbackDone", {
            defaultValue: "已恢复到操作前状态",
          }),
        );
      } catch (error) {
        console.error("[QuickStart] recovery failed:", error);
        toast.error(
          t("quickStart.rollbackFailed", {
            defaultValue: "恢复失败，请稍后重试或检查代理设置",
          }),
        );
      } finally {
        setApplying(false);
      }
    },
    [invalidateAppState, refreshRecentOperations, refreshRecoverableOperations, t],
  );

  const operationSucceeded = lastOperation?.status === "succeeded";
  const selectedName =
    selectedPreset?.nameKey &&
    selectedPreset.name !== QUICKSTART_CUSTOM_PRESET_ID
      ? String(t(selectedPreset.nameKey))
      : (selectedPreset?.name ?? "供应商");

  return (
    <div
      data-testid="quick-start-workbench"
      className="mx-auto max-w-4xl space-y-8 p-6"
    >
      <header className="flex flex-wrap items-start justify-between gap-4">
        <div className="space-y-2">
          <h1 className="flex items-center gap-2 text-2xl font-bold">
            <Sparkles className="h-6 w-6 text-primary" />
            {t("quickStart.workbenchTitle", { defaultValue: "模型与供应商" })}
          </h1>
          <p className="text-sm text-muted-foreground">
            {t("quickStart.workbenchSubtitle", {
              defaultValue:
                "选择供应商并填写 API Key，其余由 OpenSunstar 自动完成。",
            })}
          </p>
        </div>
        <Button onClick={handleOpenAddProvider}>
          <Plus className="mr-1.5 h-4 w-4" />
          {t("quickStart.addProvider", { defaultValue: "新增供应商" })}
        </Button>
      </header>

      <QuickStartAppTabs activeApp={activeApp} onChange={handleAppChange} />

      <section className="space-y-3">
        <div className="space-y-1">
          <h2 className="text-base font-semibold">
            {t("quickStart.officialConnection", { defaultValue: "官方接入" })}
          </h2>
          <p className="text-sm text-muted-foreground">
            {t("quickStart.officialConnectionHint", {
              defaultValue: "使用官方账户登录或订阅授权，无需配置中转服务。",
            })}
          </p>
        </div>
        {selection?.mode === "official" && selectedPreset ? (
          <OfficialLoginNotice preset={selectedPreset} onCancel={resetForm} />
        ) : (
          <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
            {officialGroups.flatMap((group) =>
              group.presets.map((preset) => (
                <PresetCard
                  key={`${group.category}-${preset.name}`}
                  preset={preset}
                  onSelect={() => handleSelectPreset(preset, false)}
                />
              )),
            )}
          </div>
        )}
      </section>

      {recoverableOperations.length > 0 && (
        <section
          aria-label={t("quickStart.recovery.title", {
            defaultValue: "恢复未完成操作",
          })}
          className="space-y-3 rounded-xl border border-amber-500/40 bg-amber-500/5 p-4"
        >
          <div className="flex items-center gap-2 font-medium text-amber-800 dark:text-amber-200">
            <CircleAlert className="h-4 w-4" />
            {t("quickStart.recovery.title", { defaultValue: "恢复未完成操作" })}
          </div>
          {recoverableOperations.map((operation) => (
            <div
              key={operation.id}
              className="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-border/70 bg-background/60 px-3 py-2"
            >
              <div className="min-w-0">
                <p className="text-sm font-medium">
                  {quickStartAppLabel(operation.appType, t)} ·{" "}
                  {RECOVERABLE_LABELS[operation.status] ?? operation.status}
                </p>
                <p className="truncate font-mono text-xs text-muted-foreground">
                  {operation.id}
                </p>
              </div>
              <Button
                size="sm"
                onClick={() => void handleRecover(operation)}
                disabled={applying}
              >
                {applying ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : (
                  <Undo2 className="mr-1.5 h-4 w-4" />
                )}
                {t("quickStart.recovery.action", {
                  defaultValue: "恢复未完成操作",
                })}
              </Button>
            </div>
          ))}
        </section>
      )}

      {recentOperations.filter((operation) => operation.appType === activeApp).length >
        0 && (
        <section
          data-testid="quick-start-operation-history"
          aria-label={t("quickStart.history.title", { defaultValue: "最近接入操作" })}
          className="space-y-3 rounded-xl border border-border/70 p-4"
        >
          <div>
            <h2 className="text-sm font-semibold">
              {t("quickStart.history.title", { defaultValue: "最近接入操作" })}
            </h2>
            <p className="text-xs text-muted-foreground">
              {t("quickStart.history.hint", {
                defaultValue: "重启后仍可查看审计记录；成功操作可在安全守卫通过时撤销。",
              })}
            </p>
          </div>
          {recentOperations
            .filter((operation) => operation.appType === activeApp)
            .map((operation) => (
              <div
                key={operation.id}
                className="flex flex-wrap items-center justify-between gap-3 rounded-lg border border-border/60 px-3 py-2"
              >
                <div className="min-w-0">
                  <p className="text-sm font-medium">
                    {operation.providerId ?? operation.id} · {operation.status}
                  </p>
                  <p className="truncate font-mono text-xs text-muted-foreground">
                    {operation.id}
                  </p>
                </div>
                <div className="flex gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => void handleShowAudit(operation.id)}
                  >
                    <ClipboardList className="mr-1.5 h-4 w-4" />
                    {t("quickStart.audit", { defaultValue: "查看审计记录" })}
                  </Button>
                  {operation.status === "succeeded" && (
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => void handleRecover(operation)}
                      disabled={applying}
                    >
                      <Undo2 className="mr-1.5 h-4 w-4" />
                      {t("quickStart.rollback", { defaultValue: "撤销本次接入" })}
                    </Button>
                  )}
                </div>
                {auditOperationId === operation.id && (
                  <ol className="w-full space-y-1 border-t border-border/60 pt-3 text-xs text-muted-foreground">
                    {auditEvents.map((event) => (
                      <li key={event.sequence}>
                        #{event.sequence} · {event.step} · {event.toStatus ?? event.eventType}
                        {formatUpstreamVerificationReceipt(event)
                          ? ` · ${formatUpstreamVerificationReceipt(event)}`
                          : ""}
                      </li>
                    ))}
                  </ol>
                )}
              </div>
            ))}
        </section>
      )}

      {lastOperation && (
        <section
          aria-live="polite"
          className={cn(
            "flex flex-wrap items-center justify-between gap-3 rounded-xl border p-4",
            operationSucceeded
              ? "border-emerald-500/35 bg-emerald-500/5"
              : "border-amber-500/35 bg-amber-500/5",
          )}
        >
          <div className="flex items-center gap-3">
            {operationSucceeded ? (
              <CheckCircle2 className="h-5 w-5 text-emerald-600" />
            ) : (
              <CircleAlert className="h-5 w-5 text-amber-600" />
            )}
            <div>
              <p className="font-medium">
                {operationSucceeded
                  ? t("quickStart.connected", {
                      provider: selectedName,
                      defaultValue: `${selectedName} 已连接`,
                    })
                  : t("quickStart.operationNeedsAttention", {
                      defaultValue: "本次接入需要处理",
                    })}
              </p>
              <p className="font-mono text-xs text-muted-foreground">
                {lastOperation.id}
              </p>
            </div>
          </div>
          {operationSucceeded && (
            <div className="flex gap-2">
              <Button
                variant="outline"
                size="sm"
                onClick={() => void handleShowAudit(lastOperation.id)}
              >
                <ClipboardList className="mr-1.5 h-4 w-4" />
                {t("quickStart.audit", { defaultValue: "查看审计记录" })}
              </Button>
              <Button
                variant="outline"
                size="sm"
                onClick={() => void handleRecover(lastOperation)}
                disabled={applying}
              >
                <Undo2 className="mr-1.5 h-4 w-4" />
                {t("quickStart.rollback", { defaultValue: "撤销本次接入" })}
              </Button>
            </div>
          )}
          {auditOperationId === lastOperation.id && (
            <ol
              data-testid="quick-start-audit-events"
              className="w-full space-y-1 border-t border-border/60 pt-3 text-xs text-muted-foreground"
            >
              {auditEvents.map((event) => (
                <li key={event.sequence}>
                  {event.eventType === "upstream_verification_succeeded" &&
                    formatUpstreamVerificationReceipt(event) && (
                      <span className="ml-1 text-foreground">
                        {formatUpstreamVerificationReceipt(event)}
                      </span>
                    )}
                  #{event.sequence} · {event.step} ·{" "}
                  {event.toStatus ?? event.eventType}
                  {event.errorCode ? ` · ${event.errorCode}` : ""}
                </li>
              ))}
            </ol>
          )}
        </section>
      )}

      <section className="space-y-3">
        <div className="flex items-center justify-between gap-3">
          <div>
            <h2 className="text-base font-semibold">
              {t("quickStart.connectedProviders", {
                defaultValue: "已接入供应商",
              })}
            </h2>
            <p className="text-sm text-muted-foreground">
              {t("quickStart.connectedProvidersHint", {
                defaultValue:
                  "可直接切换当前供应商；高级能力在各供应商的设置中管理。",
              })}
            </p>
          </div>
          <div className="flex items-center gap-1">
            {onOpenSettings && (
              <Button variant="ghost" size="sm" onClick={onOpenSettings}>
                <Settings2 className="mr-1.5 h-4 w-4" />
                {t("quickStart.proxySettings", {
                  defaultValue: "代理与故障设置",
                })}
              </Button>
            )}
          </div>
        </div>
        <QuickStartProviderList
          appId={activeApp}
          onAddProvider={handleOpenAddProvider}
        />
      </section>

      <Dialog
        open={isAddProviderOpen}
        onOpenChange={handleAddProviderOpenChange}
      >
        <DialogContent className="max-w-3xl overflow-hidden p-0">
          <DialogHeader className="relative pr-14">
            <DialogTitle>
              {selection
                ? t("quickStart.configureProvider", {
                    defaultValue: "配置供应商",
                  })
                : t("quickStart.addProvider", { defaultValue: "新增供应商" })}
            </DialogTitle>
            <DialogDescription>
              {selection
                ? t("quickStart.configureProviderHint", {
                    defaultValue:
                      "只需填写必要信息；连接、写入与验证由系统自动处理。",
                  })
                : t("quickStart.addProviderHint", {
                    defaultValue: "从精选供应商开始，或添加自定义兼容接口。",
                  })}
            </DialogDescription>
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="absolute right-3 top-3"
              aria-label={t("common.close", { defaultValue: "关闭" })}
              onClick={() => handleAddProviderOpenChange(false)}
              disabled={applying}
            >
              <X className="h-4 w-4" />
            </Button>
          </DialogHeader>
          <div className="min-h-0 overflow-y-auto p-6">
            <section className="space-y-4">
              {!selection ? (
                <>
                  <div className="relative">
                    <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
                    <Input
                      value={searchQuery}
                      onChange={(event) => setSearchQuery(event.target.value)}
                      placeholder={t("quickStart.searchPlaceholder", {
                        defaultValue: "搜索精选供应商名称或网址…",
                      })}
                      className="pl-9"
                    />
                  </div>
                  <div className="space-y-6">
                    {addProviderGroups.map((group) => (
                      <div key={group.category} className="space-y-2">
                        <h3 className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                          {t(
                            QUICKSTART_CATEGORY_LABEL_KEYS[group.category] ??
                              "providerPreset.other",
                            { defaultValue: group.category },
                          )}
                        </h3>
                        {group.emptyHintKey && group.presets.length === 0 && (
                          <p className="rounded-md border border-dashed border-border p-3 text-xs text-muted-foreground">
                            {t(group.emptyHintKey)}
                          </p>
                        )}
                        <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-3">
                          {group.presets.map((preset) => (
                            <PresetCard
                              key={`${group.category}-${preset.name}`}
                              preset={preset}
                              isCustom={Boolean(group.isCustomGroup)}
                              onSelect={() =>
                                handleSelectPreset(
                                  preset,
                                  Boolean(group.isCustomGroup),
                                )
                              }
                            />
                          ))}
                        </div>
                      </div>
                    ))}
                  </div>
                </>
              ) : selection.mode === "official" ? (
                <OfficialLoginNotice
                  preset={selectedPreset!}
                  onCancel={resetForm}
                />
              ) : (
                <ProviderForm
                  activeApp={activeApp}
                  selection={selection}
                  selectedPreset={selectedPreset!}
                  fields={fields}
                  connectivityVerified={connectivityVerified}
                  applying={applying}
                  onChange={patchFields}
                  onVerificationChange={setConnectivityVerified}
                  onCancel={resetForm}
                  onApply={() => void handleApply()}
                />
              )}
            </section>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}

function ProviderForm({
  activeApp,
  selection,
  selectedPreset,
  fields,
  connectivityVerified,
  applying,
  onChange,
  onVerificationChange,
  onCancel,
  onApply,
}: {
  activeApp: QuickStartAppId;
  selection: QuickStartSelection;
  selectedPreset: ResolvedQuickStartPreset;
  fields: QuickStartFormFields;
  connectivityVerified: boolean;
  applying: boolean;
  onChange: (patch: Partial<QuickStartFormFields>) => void;
  onVerificationChange: (verified: boolean) => void;
  onCancel: () => void;
  onApply: () => void;
}) {
  const { t } = useTranslation();
  const title = selectedPreset.nameKey
    ? String(t(selectedPreset.nameKey))
    : selectedPreset.name;
  return (
    <div className="space-y-5 rounded-xl border border-border bg-card p-5">
      <div className="flex items-start gap-3">
        <ProviderIcon
          icon={selectedPreset.icon}
          name={selectedPreset.name}
          color={selectedPreset.iconColor}
          size={40}
        />
        <div className="min-w-0 flex-1">
          <p className="font-semibold">{title}</p>
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
      </div>

      {selection.mode === "custom" && (
        <QuickStartCustomFields
          appId={activeApp}
          fields={fields}
          onChange={onChange}
        />
      )}

      <div className="space-y-3">
        <Label className="flex items-center gap-2">
          <KeyRound className="h-4 w-4" />
          API Key
        </Label>
        <Input
          aria-label="API Key"
          type="password"
          value={fields.apiKey}
          onChange={(event) => onChange({ apiKey: event.target.value })}
          placeholder={t("quickStart.apiKeyPlaceholder", {
            defaultValue: "粘贴你的 API Key",
          })}
          className="font-mono"
          autoFocus
        />
        <p className="text-xs text-muted-foreground">
          {t("quickStart.keychainHint", {
            defaultValue: "凭据安全保存；接入完成前还会进行真实配置验证。",
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
        onVerificationChange={onVerificationChange}
      />
      <QuickStartAdvancedPanel
        appId={activeApp}
        selection={selection}
        fields={fields}
        onChange={onChange}
      />

      <div className="flex justify-end gap-2">
        <Button variant="outline" onClick={onCancel} disabled={applying}>
          {t("common.cancel", { defaultValue: "取消" })}
        </Button>
        <Button
          onClick={onApply}
          disabled={applying || !fields.apiKey.trim() || !connectivityVerified}
        >
          {applying ? (
            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
          ) : (
            <Zap className="mr-2 h-4 w-4" />
          )}
          {t("quickStart.connectAndEnable", { defaultValue: "连接并启用" })}
        </Button>
      </div>
    </div>
  );
}

function OfficialLoginNotice({
  preset,
  onCancel,
}: {
  preset: ResolvedQuickStartPreset;
  onCancel: () => void;
}) {
  const { t } = useTranslation();
  const title = preset.nameKey ? String(t(preset.nameKey)) : preset.name;
  return (
    <div className="space-y-4 rounded-xl border border-border bg-card p-5">
      <h3 className="font-semibold">{title}</h3>
      <p className="text-sm text-muted-foreground">
        {t("quickStart.officialLoginNotice", {
          defaultValue:
            "官方供应商使用浏览器登录或订阅授权，不属于 API Key 快速接入流程。",
        })}
      </p>
      <div className="flex justify-end gap-2">
        <Button variant="outline" onClick={onCancel}>
          {t("common.back", { defaultValue: "返回" })}
        </Button>
        {preset.websiteUrl && (
          <Button asChild>
            <a href={preset.websiteUrl} target="_blank" rel="noreferrer">
              {t("quickStart.openOfficialSite", {
                defaultValue: "打开官网登录",
              })}
              <ExternalLink className="ml-1.5 h-4 w-4" />
            </a>
          </Button>
        )}
      </div>
    </div>
  );
}

function PresetCard({
  preset,
  isCustom,
  onSelect,
}: {
  preset: ResolvedQuickStartPreset;
  isCustom?: boolean;
  onSelect: () => void;
}) {
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
            {t("quickStart.official", { defaultValue: "官方登录" })}
          </span>
        )}
        {isCustom && (
          <span className="text-[10px] text-muted-foreground">
            {t("quickStart.custom.cardSubtitle", {
              defaultValue: "填写 Base URL 与模型",
            })}
          </span>
        )}
      </div>
    </button>
  );
}
