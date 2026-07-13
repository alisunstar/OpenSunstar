import { useCallback, useEffect, useMemo, useState } from "react";
import { motion } from "framer-motion";
import {
  Loader2,
  Save,
  FolderSearch,
  Database,
  Cloud,
  ScrollText,
  HardDriveDownload,
  FlaskConical,
  Eye,
} from "lucide-react";
import { toast } from "sonner";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Button } from "@/components/ui/button";
import { settingsApi } from "@/lib/api";
import { LanguageSettings } from "@/components/settings/LanguageSettings";
import { ThemeSettings } from "@/components/settings/ThemeSettings";
import { WindowSettings } from "@/components/settings/WindowSettings";
import { AppVisibilitySettings } from "@/components/settings/AppVisibilitySettings";
import { SkillStorageLocationSettings } from "@/components/settings/SkillStorageLocationSettings";
import { SkillSyncMethodSettings } from "@/components/settings/SkillSyncMethodSettings";
import { TerminalSettings } from "@/components/settings/TerminalSettings";
import { DirectorySettings } from "@/components/settings/DirectorySettings";
import { ImportExportSection } from "@/components/settings/ImportExportSection";
import { BackupListSection } from "@/components/settings/BackupListSection";
import { WebdavSyncSection } from "@/components/settings/WebdavSyncSection";
import { GistSyncSection } from "@/components/sync/GistSyncSection";
import { AboutSection } from "@/components/settings/AboutSection";
import { AuthCenterPanel } from "@/components/settings/AuthCenterPanel";
import { ModelTestConfigPanel } from "@/components/usage/ModelTestConfigPanel";
import { LogConfigPanel } from "@/components/settings/LogConfigPanel";
import { CodexAuthSettings } from "@/components/settings/CodexAuthSettings";
import { DryRunSettings } from "@/components/settings/DryRunSettings";
import { AiProviderSettings } from "@/components/settings/AiProviderSettings";
import { ProxyTabContent } from "@/components/settings/ProxyTabContent";
import { useInstalledSkills } from "@/hooks/useSkills";
import { useSettings } from "@/hooks/useSettings";
import { useImportExport } from "@/hooks/useImportExport";
import { useTranslation } from "react-i18next";
import type { SettingsFormState } from "@/hooks/useSettings";
import {
  consumeSettingsNavIntent,
  type SettingsNavIntent,
} from "@/lib/settingsNavigation";

// ── 类型 ──────────────────────────────────────────

interface SettingsPageContentProps {
  onImportSuccess?: () => void | Promise<void>;
  defaultTab?: string;
  settingsNavIntent?: SettingsNavIntent | null;
  /** Dialog 模式下，保存成功后回调（用于关闭 Dialog） */
  onAfterSave?: () => void;
  /** Dialog 打开挂载时重置导入/导出状态 */
  resetImportOnMount?: boolean;
}

interface SettingsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onImportSuccess?: () => void | Promise<void>;
  defaultTab?: string;
}

// ── 纯内容组件（内联 / Dialog 通用）───────────────

export function SettingsPageContent({
  onImportSuccess,
  defaultTab = "general",
  settingsNavIntent = null,
  onAfterSave,
  resetImportOnMount = false,
}: SettingsPageContentProps) {
  const { t } = useTranslation();
  const {
    settings,
    isLoading,
    isSaving,
    isPortable,
    appConfigDir,
    resolvedDirs,
    updateSettings,
    updateDirectory,
    updateAppConfigDir,
    browseDirectory,
    browseAppConfigDir,
    resetDirectory,
    resetAppConfigDir,
    saveSettings,
    autoSaveSettings,
    requiresRestart,
    acknowledgeRestart,
  } = useSettings();

  const {
    selectedFile,
    status: importStatus,
    errorMessage,
    backupId,
    isImporting,
    selectImportFile,
    importConfig,
    exportConfig,
    clearSelection,
    resetStatus,
  } = useImportExport({ onImportSuccess });

  useEffect(() => {
    if (resetImportOnMount) {
      resetStatus();
    }
  }, [resetImportOnMount, resetStatus]);

  const { data: installedSkills } = useInstalledSkills();

  const [activeTab, setActiveTab] = useState<string>(
    settingsNavIntent?.tab ?? defaultTab,
  );
  const [proxyOpenSections, setProxyOpenSections] = useState<string[]>(
    settingsNavIntent?.openSections ?? [],
  );
  const [showRestartPrompt, setShowRestartPrompt] = useState(false);

  useEffect(() => {
    const intent = settingsNavIntent ?? consumeSettingsNavIntent();
    if (!intent) return;
    if (intent.tab) setActiveTab(intent.tab);
    if (intent.openSections?.length) {
      setProxyOpenSections(intent.openSections);
    }
  }, [settingsNavIntent]);

  useEffect(() => {
    if (requiresRestart) {
      setShowRestartPrompt(true);
    }
  }, [requiresRestart]);

  const closeAfterSave = useCallback(() => {
    acknowledgeRestart();
    clearSelection();
    resetStatus();
    onAfterSave?.();
  }, [acknowledgeRestart, clearSelection, resetStatus, onAfterSave]);

  const handleSave = useCallback(async () => {
    try {
      const result = await saveSettings(undefined, { silent: false });
      if (!result) return;
      if (result.requiresRestart) {
        setShowRestartPrompt(true);
        return;
      }
      closeAfterSave();
    } catch (error) {
      console.error("[SettingsPage] Failed to save settings", error);
    }
  }, [closeAfterSave, saveSettings]);

  const handleRestartLater = useCallback(() => {
    setShowRestartPrompt(false);
    closeAfterSave();
  }, [closeAfterSave]);

  const handleRestartNow = useCallback(async () => {
    setShowRestartPrompt(false);
    if (import.meta.env.DEV) {
      toast.success(t("settings.devModeRestartHint"), { closeButton: true });
      closeAfterSave();
      return;
    }

    try {
      await settingsApi.restart();
    } catch (error) {
      console.error("[SettingsPage] Failed to restart app", error);
      toast.error(t("settings.restartFailed"));
    } finally {
      closeAfterSave();
    }
  }, [closeAfterSave, t]);

  const handleAutoSave = useCallback(
    async (updates: Partial<SettingsFormState>) => {
      if (!settings) return;
      updateSettings(updates);
      try {
        await autoSaveSettings(updates);
      } catch (error) {
        console.error("[SettingsPage] Failed to autosave settings", error);
        toast.error(
          t("settings.saveFailedGeneric", {
            defaultValue: "保存失败，请重试",
          }),
        );
      }
    },
    [autoSaveSettings, settings, t, updateSettings],
  );

  const isBusy = useMemo(() => isLoading && !settings, [isLoading, settings]);

  return (
    <div className="flex flex-col h-full overflow-hidden px-6">
      {isBusy ? (
        <div className="flex flex-1 items-center justify-center">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      ) : (
        <Tabs
          value={activeTab}
          onValueChange={setActiveTab}
          className="flex flex-col h-full"
        >
          <TabsList className="grid w-full grid-cols-4 mb-6 glass rounded-lg">
            <TabsTrigger value="general">
              {t("settings.tabGeneral")}
            </TabsTrigger>
            <TabsTrigger value="auth">
              {t("settings.tabAuth", { defaultValue: "认证" })}
            </TabsTrigger>
            <TabsTrigger value="advanced">
              {t("settings.tabAdvanced")}
            </TabsTrigger>
            <TabsTrigger value="about">{t("common.about")}</TabsTrigger>
          </TabsList>

          <div className="flex-1 min-h-0 flex flex-col">
            <div className="flex-1 overflow-y-auto overflow-x-hidden pr-2">
              <TabsContent value="general" className="space-y-6 mt-0">
                {settings ? (
                  <motion.div
                    initial={{ opacity: 0, y: 10 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ duration: 0.3 }}
                    className="space-y-6"
                  >
                    <LanguageSettings
                      value={settings.language}
                      onChange={(lang) => handleAutoSave({ language: lang })}
                    />
                    <ThemeSettings />
                    <AppVisibilitySettings
                      settings={settings}
                      onChange={handleAutoSave}
                    />
                    <SkillStorageLocationSettings
                      value={settings.skillStorageLocation ?? "open_sunstar"}
                      installedCount={installedSkills?.length ?? 0}
                      onMigrated={(location) =>
                        updateSettings({ skillStorageLocation: location })
                      }
                    />
                    <SkillSyncMethodSettings
                      value={settings.skillSyncMethod ?? "auto"}
                      onChange={(method) =>
                        handleAutoSave({ skillSyncMethod: method })
                      }
                    />
                    <CodexAuthSettings
                      settings={settings}
                      onChange={handleAutoSave}
                    />
                    <WindowSettings
                      settings={settings}
                      onChange={handleAutoSave}
                    />
                    <TerminalSettings
                      value={settings.preferredTerminal}
                      onChange={(terminal) =>
                        handleAutoSave({ preferredTerminal: terminal })
                      }
                    />
                  </motion.div>
                ) : null}
              </TabsContent>

              <TabsContent value="auth" className="space-y-6 mt-0 pb-4">
                <motion.div
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ duration: 0.3 }}
                >
                  <AuthCenterPanel />
                </motion.div>
              </TabsContent>

              <TabsContent value="advanced" className="space-y-6 mt-0 pb-4">
                {settings ? (
                  <motion.div
                    initial={{ opacity: 0, y: 10 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ duration: 0.3 }}
                    className="space-y-4"
                  >
                    <ProxyTabContent
                      settings={settings}
                      onAutoSave={handleAutoSave}
                      defaultOpenSections={proxyOpenSections}
                    />

                    <Accordion
                      type="multiple"
                      defaultValue={[]}
                      className="w-full space-y-4"
                    >
                      <AccordionItem
                        value="directory"
                        className="rounded-xl glass-card"
                      >
                        <AccordionTrigger className="px-6 py-4 hover:no-underline hover:bg-muted/50 data-[state=open]:bg-muted/50">
                          <div className="flex items-center gap-3">
                            <FolderSearch className="h-5 w-5 text-primary" />
                            <div className="text-left">
                              <h3 className="text-base font-semibold">
                                {t("settings.advanced.configDir.title")}
                              </h3>
                              <p className="text-sm text-muted-foreground font-normal">
                                {t("settings.advanced.configDir.description")}
                              </p>
                            </div>
                          </div>
                        </AccordionTrigger>
                        <AccordionContent className="px-6 pb-6 pt-4 border-t border-border/50">
                          <DirectorySettings
                            appConfigDir={appConfigDir}
                            resolvedDirs={resolvedDirs}
                            onAppConfigChange={updateAppConfigDir}
                            onBrowseAppConfig={browseAppConfigDir}
                            onResetAppConfig={resetAppConfigDir}
                            claudeDir={settings.claudeConfigDir}
                            codexDir={settings.codexConfigDir}
                            geminiDir={settings.geminiConfigDir}
                            opencodeDir={settings.opencodeConfigDir}
                            openclawDir={settings.openclawConfigDir}
                            hermesDir={settings.hermesConfigDir}
                            onDirectoryChange={updateDirectory}
                            onBrowseDirectory={browseDirectory}
                            onResetDirectory={resetDirectory}
                          />
                        </AccordionContent>
                      </AccordionItem>

                      <AccordionItem
                        value="data"
                        className="rounded-xl glass-card"
                      >
                        <AccordionTrigger className="px-6 py-4 hover:no-underline hover:bg-muted/50 data-[state=open]:bg-muted/50">
                          <div className="flex items-center gap-3">
                            <Database className="h-5 w-5 text-blue-500" />
                            <div className="text-left">
                              <h3 className="text-base font-semibold">
                                {t("settings.advanced.data.title")}
                              </h3>
                              <p className="text-sm text-muted-foreground font-normal">
                                {t("settings.advanced.data.description")}
                              </p>
                            </div>
                          </div>
                        </AccordionTrigger>
                        <AccordionContent className="px-6 pb-6 pt-4 border-t border-border/50">
                          <ImportExportSection
                            status={importStatus}
                            selectedFile={selectedFile}
                            errorMessage={errorMessage}
                            backupId={backupId}
                            isImporting={isImporting}
                            onSelectFile={selectImportFile}
                            onImport={importConfig}
                            onExport={exportConfig}
                            onClear={clearSelection}
                          />
                        </AccordionContent>
                      </AccordionItem>

                      <AccordionItem
                        value="backup"
                        className="rounded-xl glass-card"
                      >
                        <AccordionTrigger className="px-6 py-4 hover:no-underline hover:bg-muted/50 data-[state=open]:bg-muted/50">
                          <div className="flex items-center gap-3">
                            <HardDriveDownload className="h-5 w-5 text-amber-500" />
                            <div className="text-left">
                              <h3 className="text-base font-semibold">
                                {t("settings.advanced.backup.title", {
                                  defaultValue: "Backup & Restore",
                                })}
                              </h3>
                              <p className="text-sm text-muted-foreground font-normal">
                                {t("settings.advanced.backup.description", {
                                  defaultValue:
                                    "Manage automatic backups, view and restore database snapshots",
                                })}
                              </p>
                            </div>
                          </div>
                        </AccordionTrigger>
                        <AccordionContent className="px-6 pb-6 pt-4 border-t border-border/50">
                          <BackupListSection
                            backupIntervalHours={settings.backupIntervalHours}
                            backupRetainCount={settings.backupRetainCount}
                            onSettingsChange={(updates) =>
                              handleAutoSave(updates)
                            }
                          />
                        </AccordionContent>
                      </AccordionItem>

                      <AccordionItem
                        value="cloudSync"
                        className="rounded-xl glass-card"
                      >
                        <AccordionTrigger className="px-6 py-4 hover:no-underline hover:bg-muted/50 data-[state=open]:bg-muted/50">
                          <div className="flex items-center gap-3">
                            <Cloud className="h-5 w-5 text-blue-500" />
                            <div className="text-left">
                              <h3 className="text-base font-semibold">
                                {t("settings.advanced.cloudSync.title")}
                              </h3>
                              <p className="text-sm text-muted-foreground font-normal">
                                {t("settings.advanced.cloudSync.description")}
                              </p>
                            </div>
                          </div>
                        </AccordionTrigger>
                        <AccordionContent className="px-6 pb-6 pt-4 border-t border-border/50 space-y-8">
                          <WebdavSyncSection
                            config={settings?.webdavSync}
                            s3Config={settings?.s3Sync}
                            settings={settings}
                            onAutoSave={handleAutoSave}
                          />
                          <div className="border-t border-border/50 pt-6">
                            <GistSyncSection />
                          </div>
                        </AccordionContent>
                      </AccordionItem>

                      <AccordionItem
                        value="test"
                        className="rounded-xl glass-card"
                      >
                        <AccordionTrigger className="px-6 py-4 hover:no-underline hover:bg-muted/50 data-[state=open]:bg-muted/50">
                          <div className="flex items-center gap-3">
                            <FlaskConical className="h-5 w-5 text-emerald-500" />
                            <div className="text-left">
                              <h3 className="text-base font-semibold">
                                {t("settings.advanced.modelTest.title")}
                              </h3>
                              <p className="text-sm text-muted-foreground font-normal">
                                {t("settings.advanced.modelTest.description")}
                              </p>
                            </div>
                          </div>
                        </AccordionTrigger>
                        <AccordionContent className="px-6 pb-6 pt-4 border-t border-border/50">
                          <ModelTestConfigPanel />
                        </AccordionContent>
                      </AccordionItem>

                      <AccordionItem
                        value="dryRun"
                        className="rounded-xl glass-card"
                      >
                        <AccordionTrigger className="px-6 py-4 hover:no-underline hover:bg-muted/50 data-[state=open]:bg-muted/50">
                          <div className="flex items-center gap-3">
                            <Eye className="h-5 w-5 text-sky-500" />
                            <div className="text-left">
                              <h3 className="text-base font-semibold">
                                {t("dryRun.settingsTitle", {
                                  defaultValue: "预览模式（Dry Run）",
                                })}
                              </h3>
                              <p className="text-sm text-muted-foreground font-normal">
                                {t("dryRun.settingsDescription", {
                                  defaultValue:
                                    "写入前预览 Diff，确认后再应用到磁盘",
                                })}
                              </p>
                            </div>
                          </div>
                        </AccordionTrigger>
                        <AccordionContent className="px-6 pb-6 pt-4 border-t border-border/50">
                          <DryRunSettings />
                        </AccordionContent>
                      </AccordionItem>

                      <AccordionItem
                        value="logConfig"
                        className="rounded-xl glass-card"
                      >
                        <AccordionTrigger className="px-6 py-4 hover:no-underline hover:bg-muted/50 data-[state=open]:bg-muted/50">
                          <div className="flex items-center gap-3">
                            <ScrollText className="h-5 w-5 text-cyan-500" />
                            <div className="text-left">
                              <h3 className="text-base font-semibold">
                                {t("settings.advanced.logConfig.title")}
                              </h3>
                              <p className="text-sm text-muted-foreground font-normal">
                                {t("settings.advanced.logConfig.description")}
                              </p>
                            </div>
                          </div>
                        </AccordionTrigger>
                        <AccordionContent className="px-6 pb-6 pt-4 border-t border-border/50">
                          <LogConfigPanel />
                        </AccordionContent>
                      </AccordionItem>

                      <AccordionItem
                        value="aiProvider"
                        className="rounded-xl glass-card"
                      >
                        <AccordionTrigger className="px-6 py-4 hover:no-underline hover:bg-muted/50 data-[state=open]:bg-muted/50">
                          <div className="flex items-center gap-3">
                            <FlaskConical className="h-5 w-5 text-violet-500" />
                            <div className="text-left">
                              <h3 className="text-base font-semibold">
                                {t("settings.advanced.aiProvider.title", {
                                  defaultValue: "AI 模型路由",
                                })}
                              </h3>
                              <p className="text-sm text-muted-foreground font-normal">
                                {t("settings.advanced.aiProvider.description", {
                                  defaultValue:
                                    "配置项目看板的 AI 推理提供方（DeepSeek / GLM / 自定义）",
                                })}
                              </p>
                            </div>
                          </div>
                        </AccordionTrigger>
                        <AccordionContent className="px-6 pb-6 pt-4 border-t border-border/50">
                          <AiProviderSettings />
                        </AccordionContent>
                      </AccordionItem>
                    </Accordion>
                  </motion.div>
                ) : null}
              </TabsContent>

              <TabsContent value="about" className="mt-0">
                <AboutSection isPortable={isPortable} />
              </TabsContent>
            </div>

            {activeTab === "advanced" && settings && (
              <div
                className="flex-shrink-0 pt-4 border-t border-border-default"
                style={{ backgroundColor: "hsl(var(--background))" }}
              >
                <div className="px-6 flex items-center justify-end gap-3">
                  <Button onClick={handleSave} disabled={isSaving}>
                    {isSaving ? (
                      <span className="inline-flex items-center gap-2">
                        <Loader2 className="h-4 w-4 animate-spin" />
                        {t("settings.saving")}
                      </span>
                    ) : (
                      <>
                        <Save className="mr-2 h-4 w-4" />
                        {t("common.save")}
                      </>
                    )}
                  </Button>
                </div>
              </div>
            )}
          </div>
        </Tabs>
      )}

      <Dialog
        open={showRestartPrompt}
        onOpenChange={(open) => !open && handleRestartLater()}
      >
        <DialogContent zIndex="alert" className="max-w-md glass border-border">
          <DialogHeader>
            <DialogTitle>{t("settings.restartRequired")}</DialogTitle>
          </DialogHeader>
          <div className="px-6">
            <p className="text-sm text-muted-foreground">
              {t("settings.restartRequiredMessage")}
            </p>
          </div>
          <DialogFooter>
            <Button
              variant="ghost"
              onClick={handleRestartLater}
              className="hover:bg-muted/50"
            >
              {t("settings.restartLater")}
            </Button>
            <Button
              onClick={handleRestartNow}
              className="bg-primary hover:bg-primary/90"
            >
              {t("settings.restartNow")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

// ── Dialog 包裹版（向后兼容）───────────────────────

export function SettingsPage({
  open,
  onOpenChange,
  onImportSuccess,
  defaultTab = "general",
}: SettingsDialogProps) {
  if (!open) return null;

  return (
    <SettingsPageContent
      key={String(open)}
      onImportSuccess={onImportSuccess}
      defaultTab={defaultTab}
      onAfterSave={() => onOpenChange(false)}
      resetImportOnMount
    />
  );
}
