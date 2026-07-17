import { Github, ShieldCheck } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { CodexIcon } from "@/components/BrandIcons";
import { CopilotAuthSection } from "@/components/providers/forms/CopilotAuthSection";
import { CodexOAuthSection } from "@/components/providers/forms/CodexOAuthSection";
import { LocalCliAuthStatusPanel } from "@/components/settings/LocalCliAuthStatusPanel";
import { SubscriptionAccountsPanel } from "@/components/settings/SubscriptionAccountsPanel";

export function AuthCenterPanel() {
  const { t } = useTranslation();

  return (
    <div className="space-y-6">
      <section className="rounded-xl border border-border/60 bg-card/60 p-6">
        <div className="flex items-start justify-between gap-4">
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <ShieldCheck className="h-5 w-5 text-primary" />
              <h3 className="text-base font-semibold">
                {t("settings.authCenter.title", {
                  defaultValue: "认证中心",
                })}
              </h3>
            </div>
            <p className="text-sm text-muted-foreground">
              {t("settings.authCenter.description", {
                defaultValue:
                  "把第三方 Key 与官方订阅登录拆成两条认证轨道，帮助你识别每个 CLI 当前依赖的凭据来源。",
              })}
            </p>
          </div>
          <Badge variant="secondary">
            {t("settings.authCenter.beta", { defaultValue: "MVP" })}
          </Badge>
        </div>
      </section>

      <SubscriptionAccountsPanel />

      <LocalCliAuthStatusPanel />

      <div className="space-y-1 px-1">
        <h4 className="font-medium">
          {t("settings.authCenter.managedAccountsTitle", {
            defaultValue: "托管账号",
          })}
        </h4>
        <p className="text-sm text-muted-foreground">
          {t("settings.authCenter.managedAccountsDescription", {
            defaultValue:
              "由 OpenSunstar 管理的账号登录，可绑定到对应 Provider 或本地代理链路。",
          })}
        </p>
      </div>

      <section className="rounded-xl border border-border/60 bg-card/60 p-6">
        <div className="mb-4 flex items-center gap-3">
          <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-muted">
            <Github className="h-5 w-5" />
          </div>
          <div>
            <h4 className="font-medium">GitHub Copilot</h4>
            <p className="text-sm text-muted-foreground">
              {t("settings.authCenter.copilotDescription", {
                defaultValue: "管理 GitHub Copilot 账号",
              })}
            </p>
          </div>
        </div>

        <CopilotAuthSection />
      </section>

      <section className="rounded-xl border border-border/60 bg-card/60 p-6">
        <div className="mb-4 flex items-center gap-3">
          <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-muted">
            <CodexIcon size={20} />
          </div>
          <div>
            <h4 className="font-medium">ChatGPT (Codex OAuth)</h4>
            <p className="text-sm text-muted-foreground">
              {t("settings.authCenter.codexOauthDescription", {
                defaultValue: "管理 ChatGPT 账号",
              })}
            </p>
          </div>
        </div>

        <CodexOAuthSection />
      </section>
    </div>
  );
}
