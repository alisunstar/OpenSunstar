import { useCallback } from "react";
import { useTranslation } from "react-i18next";
import { motion } from "framer-motion";
import { Cloud, Github, HardDriveDownload } from "lucide-react";
import { toast } from "sonner";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import { Loader2 } from "lucide-react";
import { useSettings } from "@/hooks/useSettings";
import { BackupListSection } from "@/components/settings/BackupListSection";
import { WebdavSyncSection } from "@/components/settings/WebdavSyncSection";
import { GistSyncSection } from "./GistSyncSection";
import type { SettingsFormState } from "@/hooks/useSettings";

export function SyncBackupPage() {
  const { t } = useTranslation();
  const {
    settings,
    isLoading,
    updateSettings,
    autoSaveSettings,
  } = useSettings();

  const handleAutoSave = useCallback(
    async (updates: Partial<SettingsFormState>) => {
      if (!settings) return;
      updateSettings(updates);
      try {
        await autoSaveSettings(updates);
      } catch (error) {
        console.error("[SyncBackupPage] Autosave failed", error);
        toast.error(
          t("settings.saveFailedGeneric", {
            defaultValue: "保存失败，请重试",
          }),
        );
      }
    },
    [autoSaveSettings, settings, t, updateSettings],
  );

  if (isLoading && !settings) {
    return (
      <div className="flex flex-1 items-center justify-center">
        <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <motion.div
      className="flex-1 overflow-y-auto px-6 py-6 space-y-6"
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3 }}
    >
      <div>
        <h2 className="text-lg font-semibold text-foreground">
          {t("sidebar.syncBackup", { defaultValue: "同步备份" })}
        </h2>
        <p className="text-sm text-muted-foreground mt-1">
          {t("syncBackup.description", {
            defaultValue: "管理云端同步与本地数据库备份",
          })}
        </p>
      </div>

      <Accordion
        type="multiple"
        defaultValue={["cloudSync", "gistSync", "backup"]}
        className="w-full space-y-4"
      >
        {/* 云端同步 */}
        <AccordionItem value="cloudSync" className="rounded-xl glass-card">
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
          <AccordionContent className="px-6 pb-6 pt-4 border-t border-border/50">
            <WebdavSyncSection
              config={settings?.webdavSync}
              s3Config={settings?.s3Sync}
              settings={settings ?? undefined}
              onAutoSave={handleAutoSave}
            />
          </AccordionContent>
        </AccordionItem>

        {/* Gist 同步 */}
        <AccordionItem value="gistSync" className="rounded-xl glass-card">
          <AccordionTrigger className="px-6 py-4 hover:no-underline hover:bg-muted/50 data-[state=open]:bg-muted/50">
            <div className="flex items-center gap-3">
              <Github className="h-5 w-5 text-gray-800 dark:text-gray-200" />
              <div className="text-left">
                <h3 className="text-base font-semibold">
                  {t("gistSync.title", { defaultValue: "GitHub Gist 同步" })}
                </h3>
                <p className="text-sm text-muted-foreground font-normal">
                  {t("gistSync.description", {
                    defaultValue: "通过私有 GitHub Gist 同步设置与 Skills",
                  })}
                </p>
              </div>
            </div>
          </AccordionTrigger>
          <AccordionContent className="px-6 pb-6 pt-4 border-t border-border/50">
            <GistSyncSection />
          </AccordionContent>
        </AccordionItem>

        {/* 本地备份 */}
        <AccordionItem value="backup" className="rounded-xl glass-card">
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
              backupIntervalHours={settings?.backupIntervalHours}
              backupRetainCount={settings?.backupRetainCount}
              onSettingsChange={(updates) => handleAutoSave(updates)}
            />
          </AccordionContent>
        </AccordionItem>
      </Accordion>
    </motion.div>
  );
}
