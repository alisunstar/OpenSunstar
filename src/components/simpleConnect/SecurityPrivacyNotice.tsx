import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  EyeOff,
  Globe,
  HardDrive,
  KeyRound,
  ShieldCheck,
  SlidersHorizontal,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { SC_INNER } from "./ui";

const SECTION_KEYS = [
  "localFirst",
  "keychain",
  "network",
  "noTelemetry",
  "yourControl",
] as const;

const SECTION_ICONS: Record<(typeof SECTION_KEYS)[number], LucideIcon> = {
  localFirst: HardDrive,
  keychain: KeyRound,
  network: Globe,
  noTelemetry: EyeOff,
  yourControl: SlidersHorizontal,
};

interface SecurityPrivacyNoticeProps {
  className?: string;
}

export function SecurityPrivacyNotice({ className }: SecurityPrivacyNoticeProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);

  return (
    <>
      <div
        className={`${SC_INNER} flex flex-col gap-2 px-4 py-3 sm:flex-row sm:items-center sm:justify-between ${className ?? ""}`}
      >
        <div className="flex items-start gap-2.5 min-w-0">
          <ShieldCheck className="h-4 w-4 shrink-0 text-emerald-600 dark:text-emerald-400 mt-0.5" />
          <p className="text-xs sm:text-sm text-muted-foreground leading-relaxed">
            {t("simpleConnect.privacy.bannerShort", {
              defaultValue: "本地优先 · 密钥存系统 Keychain · 无遥测",
            })}
          </p>
        </div>
        <Button
          type="button"
          variant="outline"
          size="sm"
          className="h-8 shrink-0 self-start sm:self-auto border-emerald-500/25 text-emerald-800 hover:bg-emerald-500/10 dark:text-emerald-200"
          onClick={() => setOpen(true)}
        >
          {t("simpleConnect.privacy.viewDetails", {
            defaultValue: "查看安全与隐私说明",
          })}
        </Button>
      </div>

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="flex max-h-[min(90dvh,calc(100dvh-2rem))] w-[calc(100%-2rem)] max-w-lg flex-col gap-0 overflow-hidden p-0">
          <DialogHeader className="shrink-0 text-left">
            <div className="flex items-start gap-2">
              <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-emerald-500/10 text-emerald-600 dark:text-emerald-400">
                <ShieldCheck className="h-5 w-5" />
              </div>
              <div className="min-w-0">
                <DialogTitle>
                  {t("simpleConnect.privacy.title", {
                    defaultValue: "安全与隐私说明",
                  })}
                </DialogTitle>
                <DialogDescription className="mt-1 text-left">
                  {t("simpleConnect.privacy.intro", {
                    defaultValue:
                      "快速接入遵循「数据留在本机、密钥不进配置文件、网络由你指定」的原则。以下为 Simple Connect 范围内的处理方式。",
                  })}
                </DialogDescription>
              </div>
            </div>
          </DialogHeader>

          <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain px-6 py-4">
            <ul className="space-y-4">
              {SECTION_KEYS.map((key) => {
                const Icon = SECTION_ICONS[key];
                return (
                  <li
                    key={key}
                    className="rounded-lg border border-border/50 bg-muted/20 p-3.5 space-y-1.5"
                  >
                    <div className="flex items-center gap-2">
                      <Icon className="h-4 w-4 shrink-0 text-primary" />
                      <h4 className="text-sm font-medium">
                        {t(`simpleConnect.privacy.sections.${key}.title`, {
                          defaultValue: key,
                        })}
                      </h4>
                    </div>
                    <p className="pl-6 text-xs leading-relaxed text-muted-foreground sm:text-sm">
                      {t(`simpleConnect.privacy.sections.${key}.body`, {
                        defaultValue: "",
                      })}
                    </p>
                  </li>
                );
              })}
            </ul>
          </div>

          <DialogFooter className="shrink-0">
            <Button type="button" onClick={() => setOpen(false)}>
              {t("simpleConnect.privacy.close", { defaultValue: "知道了" })}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}
