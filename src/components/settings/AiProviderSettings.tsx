import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Loader2, CheckCircle2, XCircle, FlaskConical, Key, Globe } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { cn } from "@/lib/utils";
import {
  getAiProvider,
  saveAiProvider,
  getDeepseekSettings,
  saveDeepseekSettings,
  testDeepseekConnection,
  getGlmSettings,
  saveGlmSettings,
  testGlmConnection,
  getCustomProviderSettings,
  saveCustomProviderSettings,
  testCustomProviderConnection,
  type AiProvider,
} from "@/api/deepseek";

export function AiProviderSettings() {
  const { t } = useTranslation();

  // ── 状态 ──────────────────────────────────
  const [provider, setProvider] = useState<AiProvider>("deepseek");
  const [busy, setBusy] = useState(false);

  // DeepSeek
  const [dsKey, setDsKey] = useState("");
  const [dsConfigured, setDsConfigured] = useState(false);
  const [dsTestResult, setDsTestResult] = useState<{
    ok: boolean;
    message: string;
  } | null>(null);

  // GLM
  const [glmKey, setGlmKey] = useState("");
  const [glmUrl, setGlmUrl] = useState("");
  const [glmModel, setGlmModel] = useState("");
  const [glmConfigured, setGlmConfigured] = useState(false);
  const [glmTestResult, setGlmTestResult] = useState<{
    ok: boolean;
    message: string;
  } | null>(null);

  // Custom
  const [customKey, setCustomKey] = useState("");
  const [customUrl, setCustomUrl] = useState("");
  const [customModel, setCustomModel] = useState("");
  const [customConfigured, setCustomConfigured] = useState(false);
  const [customTestResult, setCustomTestResult] = useState<{
    ok: boolean;
    message: string;
  } | null>(null);

  // ── 初始化 ────────────────────────────────
  useEffect(() => {
    void (async () => {
      setProvider(await getAiProvider());
      const ds = await getDeepseekSettings();
      setDsConfigured(ds.apiKeyConfigured);
      const glm = await getGlmSettings();
      setGlmConfigured(glm.apiKeyConfigured);
      setGlmUrl(glm.apiUrl);
      setGlmModel(glm.model);
      const custom = await getCustomProviderSettings();
      setCustomConfigured(custom.apiKeyConfigured);
      setCustomUrl(custom.apiUrl);
      setCustomModel(custom.model);
    })();
  }, []);

  // ── 操作 ──────────────────────────────────
  const handleProviderChange = async (p: AiProvider) => {
    await saveAiProvider(p);
    setProvider(p);
  };

  const handleSaveDs = async () => {
    setBusy(true);
    const configured = await saveDeepseekSettings(dsKey);
    setDsConfigured(configured);
    setDsKey("");
    setBusy(false);
    toast.success(
      t("aiProvider.saved", { defaultValue: "DeepSeek 配置已保存" }),
    );
  };

  const handleTestDs = async () => {
    setBusy(true);
    setDsTestResult(null);
    const r = await testDeepseekConnection(dsKey || undefined);
    setDsTestResult(r);
    setBusy(false);
  };

  const handleSaveGlm = async () => {
    setBusy(true);
    const configured = await saveGlmSettings(glmKey, glmUrl, glmModel);
    setGlmConfigured(configured);
    setGlmKey("");
    setBusy(false);
    toast.success(
      t("aiProvider.saved", { defaultValue: "GLM 配置已保存" }),
    );
  };

  const handleTestGlm = async () => {
    setBusy(true);
    setGlmTestResult(null);
    const r = await testGlmConnection(
      glmKey || undefined,
      glmUrl,
      glmModel,
    );
    setGlmTestResult(r);
    setBusy(false);
  };

  const handleSaveCustom = async () => {
    setBusy(true);
    const configured = await saveCustomProviderSettings(
      customKey,
      customUrl,
      customModel,
    );
    setCustomConfigured(configured);
    setCustomKey("");
    setBusy(false);
    toast.success(t("aiProvider.saved", { defaultValue: "自定义配置已保存" }));
  };

  const handleTestCustom = async () => {
    setBusy(true);
    setCustomTestResult(null);
    const r = await testCustomProviderConnection(
      customKey || undefined,
      customUrl,
      customModel,
    );
    setCustomTestResult(r);
    setBusy(false);
  };

  // ── 渲染 ──────────────────────────────────
  return (
    <div className="space-y-6">
      {/* 提供方选择 */}
      <div className="space-y-3">
        <div className="flex items-center gap-2">
          <FlaskConical className="h-4 w-4 text-primary" />
          <Label className="text-sm font-semibold">
            {t("aiProvider.title", { defaultValue: "AI 提供方" })}
          </Label>
        </div>
        <p className="text-xs text-muted-foreground">
          {t("aiProvider.description", {
            defaultValue:
              "选择用于项目看板 AI 能力的模型提供方。DeepSeek 使用 deepseek-chat，GLM 使用智谱模型。密钥仅保存在本机。",
          })}
        </p>
        <Select
          value={provider}
          onValueChange={(v) => handleProviderChange(v as AiProvider)}
        >
          <SelectTrigger className="w-48">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="deepseek">DeepSeek</SelectItem>
            <SelectItem value="glm">GLM（智谱）</SelectItem>
            <SelectItem value="custom">
              {t("aiProvider.custom", { defaultValue: "自定义" })}
            </SelectItem>
          </SelectContent>
        </Select>
      </div>

      {/* 分隔 */}
      <div className="border-t border-border/40" />

      {/* DeepSeek 配置 */}
      {provider === "deepseek" && (
        <div className="space-y-4">
          <div className="flex items-center gap-2">
            <Key className="h-4 w-4 text-blue-500" />
            <h4 className="text-sm font-semibold">DeepSeek</h4>
            {dsConfigured && (
              <span className="inline-flex items-center gap-1 text-xs text-emerald-600 dark:text-emerald-400">
                <CheckCircle2 className="h-3 w-3" />
                {t("aiProvider.configured", { defaultValue: "已配置" })}
              </span>
            )}
          </div>

          <div className="space-y-2">
            <Label htmlFor="ds-key">API Key</Label>
            <Input
              id="ds-key"
              type="password"
              autoComplete="off"
              placeholder={
                dsConfigured
                  ? "密钥已保存；输入新密钥可覆盖（sk-…）"
                  : "sk-…"
              }
              value={dsKey}
              onChange={(e) => setDsKey(e.target.value)}
            />
          </div>

          <div className="flex gap-2">
            <Button
              variant="outline"
              size="sm"
              disabled={busy}
              onClick={handleSaveDs}
            >
              {busy ? (
                <Loader2 className="h-3.5 w-3.5 animate-spin mr-1" />
              ) : null}
              {t("common.save", { defaultValue: "保存" })}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              disabled={busy}
              onClick={handleTestDs}
            >
              <Globe className="h-3.5 w-3.5 mr-1" />
              {t("aiProvider.testConnection", { defaultValue: "测试连接" })}
            </Button>
          </div>

          {dsTestResult && (
            <div
              className={cn(
                "flex items-center gap-2 text-xs px-3 py-2 rounded-lg",
                dsTestResult.ok
                  ? "bg-emerald-500/10 text-emerald-700 dark:text-emerald-400"
                  : "bg-destructive/10 text-destructive",
              )}
            >
              {dsTestResult.ok ? (
                <CheckCircle2 className="h-3.5 w-3.5 shrink-0" />
              ) : (
                <XCircle className="h-3.5 w-3.5 shrink-0" />
              )}
              {dsTestResult.message}
            </div>
          )}
        </div>
      )}

      {/* GLM 配置 */}
      {provider === "glm" && (
        <div className="space-y-4">
          <div className="flex items-center gap-2">
            <Key className="h-4 w-4 text-violet-500" />
            <h4 className="text-sm font-semibold">GLM（智谱）</h4>
            {glmConfigured && (
              <span className="inline-flex items-center gap-1 text-xs text-emerald-600 dark:text-emerald-400">
                <CheckCircle2 className="h-3 w-3" />
                {t("aiProvider.configured", { defaultValue: "已配置" })}
              </span>
            )}
          </div>

          <div className="space-y-2">
            <Label htmlFor="glm-key">API Key</Label>
            <Input
              id="glm-key"
              type="password"
              autoComplete="off"
              placeholder={
                glmConfigured ? "密钥已保存；输入新密钥可覆盖" : "例如 7b10…"
              }
              value={glmKey}
              onChange={(e) => setGlmKey(e.target.value)}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="glm-url">
              {t("aiProvider.apiUrl", { defaultValue: "API 地址" })}
            </Label>
            <Input
              id="glm-url"
              autoComplete="off"
              placeholder="https://open.bigmodel.cn/api/coding/paas/v4/chat/completions"
              value={glmUrl}
              onChange={(e) => setGlmUrl(e.target.value)}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="glm-model">
              {t("aiProvider.model", { defaultValue: "模型" })}
            </Label>
            <Input
              id="glm-model"
              autoComplete="off"
              placeholder="GLM-5.1"
              value={glmModel}
              onChange={(e) => setGlmModel(e.target.value)}
            />
          </div>

          <div className="flex gap-2">
            <Button
              variant="outline"
              size="sm"
              disabled={busy}
              onClick={handleSaveGlm}
            >
              {busy ? (
                <Loader2 className="h-3.5 w-3.5 animate-spin mr-1" />
              ) : null}
              {t("common.save", { defaultValue: "保存" })}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              disabled={busy}
              onClick={handleTestGlm}
            >
              <Globe className="h-3.5 w-3.5 mr-1" />
              {t("aiProvider.testConnection", { defaultValue: "测试连接" })}
            </Button>
          </div>

          {glmTestResult && (
            <div
              className={cn(
                "flex items-center gap-2 text-xs px-3 py-2 rounded-lg",
                glmTestResult.ok
                  ? "bg-emerald-500/10 text-emerald-700 dark:text-emerald-400"
                  : "bg-destructive/10 text-destructive",
              )}
            >
              {glmTestResult.ok ? (
                <CheckCircle2 className="h-3.5 w-3.5 shrink-0" />
              ) : (
                <XCircle className="h-3.5 w-3.5 shrink-0" />
              )}
              {glmTestResult.message}
            </div>
          )}
        </div>
      )}

      {/* 自定义提供方配置 */}
      {provider === "custom" && (
        <div className="space-y-4">
          <div className="flex items-center gap-2">
            <Key className="h-4 w-4 text-amber-500" />
            <h4 className="text-sm font-semibold">
              {t("aiProvider.customTitle", { defaultValue: "自定义" })}
            </h4>
            {customConfigured && (
              <span className="inline-flex items-center gap-1 text-xs text-emerald-600 dark:text-emerald-400">
                <CheckCircle2 className="h-3 w-3" />
                {t("aiProvider.configured", { defaultValue: "已配置" })}
              </span>
            )}
          </div>

          <p className="text-xs text-muted-foreground">
            {t("aiProvider.customDescription", {
              defaultValue:
                "兼容 OpenAI 格式的任意 API。填入端点地址、模型名和 API Key 即可。",
            })}
          </p>

          <div className="space-y-2">
            <Label htmlFor="custom-key">API Key</Label>
            <Input
              id="custom-key"
              type="password"
              autoComplete="off"
              placeholder={customConfigured ? "密钥已保存；输入新密钥可覆盖" : "sk-…"}
              value={customKey}
              onChange={(e) => setCustomKey(e.target.value)}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="custom-url">
              {t("aiProvider.apiUrl", { defaultValue: "API 地址" })}
            </Label>
            <Input
              id="custom-url"
              autoComplete="off"
              placeholder="https://api.openai.com/v1/chat/completions"
              value={customUrl}
              onChange={(e) => setCustomUrl(e.target.value)}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="custom-model">
              {t("aiProvider.model", { defaultValue: "模型" })}
            </Label>
            <Input
              id="custom-model"
              autoComplete="off"
              placeholder="gpt-4o"
              value={customModel}
              onChange={(e) => setCustomModel(e.target.value)}
            />
          </div>

          <div className="flex gap-2">
            <Button
              variant="outline"
              size="sm"
              disabled={busy}
              onClick={handleSaveCustom}
            >
              {busy ? (
                <Loader2 className="h-3.5 w-3.5 animate-spin mr-1" />
              ) : null}
              {t("common.save", { defaultValue: "保存" })}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              disabled={busy}
              onClick={handleTestCustom}
            >
              <Globe className="h-3.5 w-3.5 mr-1" />
              {t("aiProvider.testConnection", { defaultValue: "测试连接" })}
            </Button>
          </div>

          {customTestResult && (
            <div
              className={cn(
                "flex items-center gap-2 text-xs px-3 py-2 rounded-lg",
                customTestResult.ok
                  ? "bg-emerald-500/10 text-emerald-700 dark:text-emerald-400"
                  : "bg-destructive/10 text-destructive",
              )}
            >
              {customTestResult.ok ? (
                <CheckCircle2 className="h-3.5 w-3.5 shrink-0" />
              ) : (
                <XCircle className="h-3.5 w-3.5 shrink-0" />
              )}
              {customTestResult.message}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
