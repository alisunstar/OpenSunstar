import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { CheckCircle, Search, ArrowRight, Sparkles } from "lucide-react";

interface ScanResult {
  providersFound: {
    appType: string;
    name: string;
    configPath: string;
    hasApiKey: boolean;
  }[];
  mcpServersFound: { name: string; sourceApp: string; command?: string }[];
  totalItems: number;
}

interface OnboardingWizardProps {
  onComplete: () => void;
}

export function OnboardingWizard({ onComplete }: OnboardingWizardProps) {
  const { t } = useTranslation();
  const [step, setStep] = useState(0);
  const [scanResult, setScanResult] = useState<ScanResult | null>(null);
  const [scanning, setScanning] = useState(false);

  const startScan = async () => {
    setScanning(true);
    try {
      const result = await invoke<ScanResult>("scan_environment");
      setScanResult(result);
      setStep(1);
    } catch (e) {
      console.error("Scan failed:", e);
      setStep(1);
    } finally {
      setScanning(false);
    }
  };

  const handleComplete = async () => {
    try {
      await invoke("complete_onboarding");
    } catch (e) {
      console.error("Failed to complete onboarding:", e);
    }
    onComplete();
  };

  if (step === 0) {
    return (
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-background/95 backdrop-blur-sm">
        <Card className="w-[480px] p-8 space-y-6 shadow-2xl">
          <div className="text-center space-y-3">
            <div className="mx-auto w-16 h-16 rounded-2xl bg-primary/10 flex items-center justify-center">
              <Sparkles className="w-8 h-8 text-primary" />
            </div>
            <h1 className="text-2xl font-bold">
              {t("onboarding.welcomeTitle", {
                defaultValue: "欢迎使用 OpenSunstar",
              })}
            </h1>
            <p className="text-muted-foreground">
              {t("onboarding.welcomeDescription", {
                defaultValue:
                  "您的 AI 编程助手统一控制面板。让我们扫描系统中已有的配置。",
              })}
            </p>
          </div>
          <Button
            className="w-full"
            size="lg"
            onClick={startScan}
            disabled={scanning}
          >
            {scanning ? (
              <>
                <Search className="w-4 h-4 mr-2 animate-spin" />
                {t("onboarding.scanning", { defaultValue: "扫描中..." })}
              </>
            ) : (
              <>
                <Search className="w-4 h-4 mr-2" />
                {t("onboarding.scanSystem", { defaultValue: "扫描我的系统" })}
              </>
            )}
          </Button>
          <Button variant="ghost" className="w-full" onClick={handleComplete}>
            {t("onboarding.skipForNow", { defaultValue: "暂时跳过" })}
          </Button>
        </Card>
      </div>
    );
  }

  if (step === 1) {
    return (
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-background/95 backdrop-blur-sm">
        <Card className="w-[520px] p-8 space-y-6 shadow-2xl">
          <div className="text-center space-y-2">
            <h2 className="text-xl font-bold">
              {t("onboarding.scanResultsTitle", { defaultValue: "扫描结果" })}
            </h2>
            <p className="text-sm text-muted-foreground">
              {t("onboarding.scanResultsSummary", {
                count: scanResult?.totalItems || 0,
                defaultValue: "在您的系统中找到 {{count}} 项配置",
              })}
            </p>
          </div>

          {scanResult && scanResult.providersFound.length > 0 && (
            <div className="space-y-2">
              <h3 className="text-sm font-medium">
                {t("onboarding.detectedProviders", {
                  defaultValue: "检测到的供应商",
                })}
              </h3>
              {scanResult.providersFound.map((p, i) => (
                <div
                  key={i}
                  className="flex items-center gap-3 p-3 rounded-lg border"
                >
                  <CheckCircle className="w-4 h-4 text-green-500 shrink-0" />
                  <div className="flex-1 min-w-0">
                    <div className="font-medium text-sm">{p.name}</div>
                    <div className="text-xs text-muted-foreground truncate">
                      {p.configPath}
                    </div>
                  </div>
                  {p.hasApiKey && (
                    <span className="text-xs px-2 py-0.5 rounded bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300">
                      {t("onboarding.apiKeyBadge", { defaultValue: "API Key" })}
                    </span>
                  )}
                </div>
              ))}
            </div>
          )}

          {scanResult && scanResult.mcpServersFound.length > 0 && (
            <div className="space-y-2">
              <h3 className="text-sm font-medium">
                {t("onboarding.detectedMcpServers", {
                  defaultValue: "检测到的 MCP 服务器",
                })}
              </h3>
              {scanResult.mcpServersFound.map((m, i) => (
                <div
                  key={i}
                  className="flex items-center gap-3 p-3 rounded-lg border"
                >
                  <CheckCircle className="w-4 h-4 text-blue-500 shrink-0" />
                  <div className="flex-1">
                    <div className="font-medium text-sm">{m.name}</div>
                    <div className="text-xs text-muted-foreground">
                      {t("onboarding.mcpFromSource", {
                        source: m.sourceApp,
                        defaultValue: "来自 {{source}}",
                      })}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}

          {(!scanResult || scanResult.totalItems === 0) && (
            <div className="text-center py-6 text-muted-foreground">
              <p>
                {t("onboarding.noConfigsFound", {
                  defaultValue: "未发现已有的 AI 工具配置。",
                })}
              </p>
              <p className="text-sm mt-1">
                {t("onboarding.addProvidersManually", {
                  defaultValue: "您可以在设置中手动添加供应商。",
                })}
              </p>
            </div>
          )}

          <Button className="w-full" size="lg" onClick={handleComplete}>
            {t("onboarding.getStarted", { defaultValue: "开始使用" })}
            <ArrowRight className="w-4 h-4 ml-2" />
          </Button>
        </Card>
      </div>
    );
  }

  return null;
}
