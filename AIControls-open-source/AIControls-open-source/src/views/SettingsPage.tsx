import { useEffect, useState } from "react";
import { PageRefreshButton } from "../components/PageRefreshButton";
import { useI18n } from "../i18n/provider";
import {
  getDeepseekSettings,
  saveDeepseekSettings,
  testDeepseekConnection,
} from "../api/deepseek";
import {
  getAiProvider,
  saveAiProvider,
  getGlmSettings,
  saveGlmSettings,
  testGlmConnection,
} from "../api/glm";
import {
  getGiteeSettings,
  saveGiteeApp,
  giteeOauthLogin,
  giteeBackupNow,
  giteeDisconnect,
  giteeRestoreFromRepoUrl,
} from "../api/gitee";
import {
  clearHiddenSidebarAgents,
  detectClaudeHookStatus,
  installClaudeHooks,
  removeClaudeHooks,
  type ClaudeHookStatus,
} from "../api/agents";

function InfoTooltip({ label, content }: { label: string; content: string }) {
  return (
    <span className="settings-info" aria-label={label}>
      <span className="settings-info__icon" aria-hidden="true">
        i
      </span>
      <span className="settings-info__tip" role="tooltip">
        {content}
      </span>
    </span>
  );
}

export default function SettingsPage() {
  const { t, locale, preference, setPreference } = useI18n();
  const [apiKeyInput, setApiKeyInput] = useState("");
  const [configured, setConfigured] = useState(false);
  const [loadErr, setLoadErr] = useState<string | null>(null);
  const [saveHint, setSaveHint] = useState<string | null>(null);
  const [testHint, setTestHint] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [settingsRefreshKey, setSettingsRefreshKey] = useState(0);
  const [settingsReloading, setSettingsReloading] = useState(false);

  const [aiProvider, setAiProvider] = useState("deepseek");
  const [glmApiKeyInput, setGlmApiKeyInput] = useState("");
  const [glmApiUrlInput, setGlmApiUrlInput] = useState("");
  const [glmModelInput, setGlmModelInput] = useState("");
  const [glmConfigured, setGlmConfigured] = useState(false);
  const [glmSaveHint, setGlmSaveHint] = useState<string | null>(null);
  const [glmTestHint, setGlmTestHint] = useState<string | null>(null);

  const [giteeClientId, setGiteeClientId] = useState("");
  const [giteeSecret, setGiteeSecret] = useState("");
  const [giteeRepo, setGiteeRepo] = useState("");
  const [giteeRepoUrlInput, setGiteeRepoUrlInput] = useState("");
  const [giteePublic, setGiteePublic] = useState<Awaited<
    ReturnType<typeof getGiteeSettings>
  > | null>(null);
  const [giteeHint, setGiteeHint] = useState<string | null>(null);
  const [sidebarAgentsHint, setSidebarAgentsHint] = useState<string | null>(null);
  const [claudeHookStatus, setClaudeHookStatus] = useState<ClaudeHookStatus | null>(null);
  const [claudeHookHint, setClaudeHookHint] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setSettingsReloading(true);
    Promise.all([getDeepseekSettings(), getGiteeSettings(), getAiProvider(), getGlmSettings(), detectClaudeHookStatus()])
      .then(([ds, gs, provider, glm, hookStatus]) => {
        if (cancelled) return;
        setSettingsReloading(false);
        if (!ds) {
          setLoadErr("无法读取设置（请在 AIControls 桌面端运行）。");
          return;
        }
        setConfigured(ds.apiKeyConfigured);
        setLoadErr(null);

        setGiteePublic(gs);
        if (gs) {
          setGiteeClientId(gs.clientIdSaved ?? "");
          setGiteeRepo(gs.savedRepoName ?? "");
        }

        setAiProvider(provider ?? "deepseek");
        if (glm) {
          setGlmConfigured(glm.apiKeyConfigured);
          setGlmApiUrlInput(glm.apiUrl);
          setGlmModelInput(glm.model);
        }
        if (!("error" in hookStatus)) {
          setClaudeHookStatus(hookStatus);
        } else {
          setClaudeHookHint(hookStatus.error);
        }
      })
      .catch(() => {
        if (cancelled) return;
        setSettingsReloading(false);
        setLoadErr("无法读取设置（请在 AIControls 桌面端运行）。");
      });
    return () => {
      cancelled = true;
    };
  }, [settingsRefreshKey]);

  async function onSave() {
    setSaveHint(null);
    setBusy(true);
    const ok = await saveDeepseekSettings(apiKeyInput);
    setBusy(false);
    if (!ok) {
      setSaveHint("保存失败：请在桌面端运行并检查写入权限。");
      return;
    }
    setSaveHint("已保存到本机应用数据目录。");
    setConfigured(apiKeyInput.trim().length > 0);
    setApiKeyInput("");
  }

  async function onTest() {
    setTestHint(null);
    setBusy(true);
    const r = await testDeepseekConnection();
    setBusy(false);
    setTestHint(
      r.ok ? `连接成功：${r.message}` : `连接失败：${r.message}`,
    );
  }

  async function onProviderChange(newProvider: string) {
    setBusy(true);
    await saveAiProvider(newProvider);
    setAiProvider(newProvider);
    setBusy(false);
  }

  async function onGlmSave() {
    setGlmSaveHint(null);
    setBusy(true);
    const ok = await saveGlmSettings(glmApiKeyInput, glmApiUrlInput, glmModelInput);
    setBusy(false);
    if (!ok) {
      setGlmSaveHint("保存失败：请在桌面端运行并检查写入权限。");
      return;
    }
    setGlmSaveHint("已保存到本机应用数据目录。");
    setGlmConfigured(glmApiKeyInput.trim().length > 0);
    setGlmApiKeyInput("");
  }

  async function onGlmTest() {
    setGlmTestHint(null);
    setBusy(true);
    const r = await testGlmConnection();
    setBusy(false);
    setGlmTestHint(
      r.ok ? `连接成功：${r.message}` : `连接失败：${r.message}`,
    );
  }

  async function onGiteeSave() {
    setGiteeHint(null);
    setBusy(true);
    const r = await saveGiteeApp(giteeClientId, giteeSecret, giteeRepo);
    setBusy(false);
    setGiteeHint(r.ok ? r.message : r.message);
    if (r.ok) {
      setGiteeSecret("");
      setSettingsRefreshKey((k) => k + 1);
    }
  }

  async function onGiteeAuth() {
    setGiteeHint(null);
    setBusy(true);
    const r = await giteeOauthLogin();
    setBusy(false);
    setGiteeHint(r.ok ? r.message : r.message);
    if (r.ok) {
      setSettingsRefreshKey((k) => k + 1);
    }
  }

  async function onGiteeBackup() {
    setGiteeHint(null);
    setBusy(true);
    const r = await giteeBackupNow();
    setBusy(false);
    setGiteeHint(r.ok ? r.message : r.message);
  }

  async function onGiteeDisconnect() {
    setGiteeHint(null);
    setBusy(true);
    const r = await giteeDisconnect();
    setBusy(false);
    setGiteeHint(r.ok ? r.message : r.message);
    if (r.ok) {
      setSettingsRefreshKey((k) => k + 1);
    }
  }

  async function onGiteeRestore() {
    setGiteeHint(null);
    setBusy(true);
    const r = await giteeRestoreFromRepoUrl(giteeRepoUrlInput);
    setBusy(false);
    setGiteeHint(r.ok ? r.message : r.message);
  }

  async function refreshClaudeHookStatus() {
    const result = await detectClaudeHookStatus();
    if ("error" in result) {
      setClaudeHookHint(result.error);
      return;
    }
    setClaudeHookStatus(result);
    setClaudeHookHint(null);
  }

  async function onInstallClaudeHooks() {
    setClaudeHookHint(null);
    setBusy(true);
    const result = await installClaudeHooks();
    setBusy(false);
    if ("error" in result) {
      setClaudeHookHint(result.error);
      return;
    }
    setClaudeHookStatus(result);
    setClaudeHookHint(locale === "zh" ? "Claude hook 已安装或刷新。" : "Claude hook installed or refreshed.");
  }

  async function onRemoveClaudeHooks() {
    setClaudeHookHint(null);
    setBusy(true);
    const result = await removeClaudeHooks();
    setBusy(false);
    if ("error" in result) {
      setClaudeHookHint(result.error);
      return;
    }
    setClaudeHookStatus(result);
    setClaudeHookHint(locale === "zh" ? "Claude hook 已移除。" : "Claude hook removed.");
  }

  return (
    <div className="card settings-page">
      <div className="page-header__title-bar">
        <h2>{t("settings.title")}</h2>
        <PageRefreshButton
          onClick={() => setSettingsRefreshKey((k) => k + 1)}
          disabled={busy || settingsReloading}
          spinning={settingsReloading}
          label={t("settings.reload")}
        />
      </div>

      <section style={{ marginTop: "1.25rem" }}>
        <div className="settings-language-row">
          <div className="settings-block-head">
            <h3 className="settings-block-title">{t("settings.lang")}</h3>
          </div>
          <div className="seg" role="tablist" aria-label={t("settings.lang")}>
            <button
              type="button"
              role="tab"
              aria-selected={preference === "system"}
              className={`seg__item${preference === "system" ? " active" : ""}`}
              onClick={() => setPreference("system")}
            >
              {t("settings.lang.follow")}
            </button>
            <button
              type="button"
              role="tab"
              aria-selected={preference === "zh"}
              className={`seg__item${preference === "zh" ? " active" : ""}`}
              onClick={() => setPreference("zh")}
            >
              {t("settings.lang.zh")}
            </button>
            <button
              type="button"
              role="tab"
              aria-selected={preference === "en"}
              className={`seg__item${preference === "en" ? " active" : ""}`}
              onClick={() => setPreference("en")}
            >
              {t("settings.lang.en")}
            </button>
          </div>
        </div>
      </section>

      <section style={{ marginTop: "1.25rem" }}>
        <div className="settings-block-head">
          <h3 className="settings-block-title">{t("settings.sidebarAgents")}</h3>
        </div>
        <p className="muted" style={{ margin: "0 0 0.75rem", fontSize: "0.85rem" }}>
          {t("settings.restoreHiddenAgentsHint")}
        </p>
        <button
          type="button"
          className="btn-icon"
          disabled={busy}
          onClick={async () => {
            setSidebarAgentsHint(null);
            setBusy(true);
            const r = await clearHiddenSidebarAgents();
            setBusy(false);
            if ("error" in r) {
              setSidebarAgentsHint(r.error);
              return;
            }
            setSidebarAgentsHint(
              locale === "zh" ? "已恢复内置 Agent 侧栏列表。" : "Built-in agents restored in the sidebar.",
            );
            window.dispatchEvent(new Event("aicontrols-agents-changed"));
          }}
        >
          {t("settings.restoreHiddenAgents")}
        </button>
        {sidebarAgentsHint ? (
          <p className="muted" style={{ marginTop: "0.55rem", fontSize: "0.85rem" }}>
            {sidebarAgentsHint}
          </p>
        ) : null}
      </section>

      <section style={{ marginTop: "1.25rem" }}>
        <div className="settings-block-head">
          <h3 className="settings-block-title">{t("settings.claudeHooks")}</h3>
        </div>
        <p className="muted" style={{ margin: "0 0 0.75rem", fontSize: "0.85rem" }}>
          {t("settings.claudeHooksHint")}
        </p>
        <p className="muted" style={{ margin: "0 0 0.5rem", fontSize: "0.85rem" }}>
          {locale === "zh" ? "当前状态：" : "Status: "}
          {claudeHookStatus?.installed
            ? t("settings.claudeHooksInstalled")
            : t("settings.claudeHooksNotInstalled")}
        </p>
        {claudeHookStatus ? (
          <>
            <p className="muted" style={{ margin: "0 0 0.35rem", fontSize: "0.82rem" }}>
              {t("settings.claudeHooksSettingsPath")}: {claudeHookStatus.settingsPath}
            </p>
            <p className="muted" style={{ margin: "0 0 0.75rem", fontSize: "0.82rem" }}>
              {t("settings.claudeHooksBridgePath")}: {claudeHookStatus.bridgeScriptPath}
            </p>
          </>
        ) : null}
        <div style={{ display: "flex", gap: "0.55rem", flexWrap: "wrap" }}>
          <button
            type="button"
            className="btn-icon"
            disabled={busy}
            onClick={onInstallClaudeHooks}
          >
            {t("settings.claudeHooksInstall")}
          </button>
          <button
            type="button"
            className="btn-icon"
            disabled={busy}
            onClick={refreshClaudeHookStatus}
          >
            {t("settings.claudeHooksRefresh")}
          </button>
          <button
            type="button"
            className="btn-icon"
            disabled={busy}
            onClick={onRemoveClaudeHooks}
          >
            {t("settings.claudeHooksRemove")}
          </button>
        </div>
        {claudeHookHint ? (
          <p className="muted" style={{ marginTop: "0.55rem", fontSize: "0.85rem" }}>
            {claudeHookHint}
          </p>
        ) : null}
      </section>

      <section style={{ marginTop: "1.25rem" }}>
        <div className="settings-block-head">
          <h3 className="settings-block-title">{locale === "zh" ? "AI 提供方" : "AI Provider"}</h3>
          <InfoTooltip
            label={locale === "zh" ? "AI 提供方说明" : "About AI Provider"}
            content={
              locale === "zh"
                ? "选择用于 AI 能力的模型提供方。DeepSeek 使用 deepseek-chat，GLM 使用智谱 GLM-5.1 / GLM-4.7。密钥仅保存在本机，不上传到 AIControls 服务端。"
                : "Select the AI model provider. DeepSeek uses deepseek-chat, GLM uses Zhipu GLM-5.1 / GLM-4.7. Keys are stored locally only."
            }
          />
        </div>
        <div className="seg" role="tablist" aria-label="AI Provider" style={{ marginBottom: "1rem" }}>
          <button
            type="button"
            role="tab"
            aria-selected={aiProvider === "deepseek"}
            className={`seg__item${aiProvider === "deepseek" ? " active" : ""}`}
            disabled={busy}
            onClick={() => onProviderChange("deepseek")}
          >
            DeepSeek
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={aiProvider === "glm"}
            className={`seg__item${aiProvider === "glm" ? " active" : ""}`}
            disabled={busy}
            onClick={() => onProviderChange("glm")}
          >
            GLM
          </button>
        </div>

        {aiProvider === "deepseek" ? (
          <>
            <div className="settings-block-head">
              <h3 className="settings-block-title">DeepSeek</h3>
              <InfoTooltip
                label={locale === "zh" ? "DeepSeek 说明" : "About DeepSeek"}
                content={
                  locale === "zh"
                    ? "填写 DeepSeek API Key。扫描 Agent / 项目 / 全部时，应用会为未缓存的 Skill、MCP、Rules 生成场景分类并写入本地；后续优先读取缓存，仅在有新条目时再请求模型。"
                    : "Enter DeepSeek API key. When scanning agents/projects/assets, uncached entries are classified and cached locally."
                }
              />
            </div>
            {loadErr ? (
              <p className="muted" style={{ margin: "0 0 0.75rem" }}>
                {loadErr}
              </p>
            ) : null}

            <label
              htmlFor="deepseek-api-key"
              style={{ display: "block", marginBottom: "0.35rem", fontSize: "0.85rem" }}
            >
              API Key
            </label>
            <input
              id="deepseek-api-key"
              type="password"
              autoComplete="off"
              className="settings-input"
              placeholder={
                configured
                  ? locale === "zh"
                    ? "密钥已保存；输入新密钥可覆盖"
                    : "Key saved; enter a new key to replace"
                  : locale === "zh"
                    ? "例如 sk-…"
                    : "e.g. sk-…"
              }
              value={apiKeyInput}
              onChange={(e) => setApiKeyInput(e.target.value)}
            />

            <div
              style={{
                display: "flex",
                gap: "0.55rem",
                flexWrap: "wrap",
                marginTop: "0.85rem",
              }}
            >
              <button
                type="button"
                className="btn-icon"
                disabled={busy}
                onClick={onSave}
              >
                {locale === "zh" ? "保存" : "Save"}
              </button>
              <button
                type="button"
                className="btn-icon"
                disabled={busy}
                onClick={onTest}
              >
                {locale === "zh" ? "测试连接" : "Test connection"}
              </button>
            </div>

            {saveHint ? (
              <p className="muted" style={{ marginTop: "0.55rem", fontSize: "0.85rem" }}>
                {saveHint}
              </p>
            ) : null}
            {testHint ? (
              <p className="muted" style={{ marginTop: "0.55rem", fontSize: "0.85rem" }}>
                {testHint}
              </p>
            ) : null}
          </>
        ) : (
          <>
            <div className="settings-block-head">
              <h3 className="settings-block-title">GLM（智谱）</h3>
              <InfoTooltip
                label={locale === "zh" ? "GLM 说明" : "About GLM"}
                content={
                  locale === "zh"
                    ? "填写 GLM API Key。支持 GLM-5.1、GLM-4.7 等模型。密钥仅保存在本机。"
                    : "Enter GLM API key. Supports GLM-5.1, GLM-4.7 and more. Key stored locally only."
                }
              />
            </div>

            <label
              htmlFor="glm-api-key"
              style={{ display: "block", marginBottom: "0.35rem", fontSize: "0.85rem" }}
            >
              API Key
            </label>
            <input
              id="glm-api-key"
              type="password"
              autoComplete="off"
              className="settings-input"
              placeholder={
                glmConfigured
                  ? locale === "zh"
                    ? "密钥已保存；输入新密钥可覆盖"
                    : "Key saved; enter a new key to replace"
                  : locale === "zh"
                    ? "例如 7b10…"
                    : "e.g. 7b10…"
              }
              value={glmApiKeyInput}
              onChange={(e) => setGlmApiKeyInput(e.target.value)}
            />

            <label
              htmlFor="glm-api-url"
              style={{ display: "block", margin: "0.75rem 0 0.35rem", fontSize: "0.85rem" }}
            >
              {locale === "zh" ? "API 地址" : "API URL"}
            </label>
            <input
              id="glm-api-url"
              className="settings-input"
              autoComplete="off"
              placeholder="https://open.bigmodel.cn/api/coding/paas/v4/chat/completions"
              value={glmApiUrlInput}
              onChange={(e) => setGlmApiUrlInput(e.target.value)}
            />

            <label
              htmlFor="glm-model"
              style={{ display: "block", margin: "0.75rem 0 0.35rem", fontSize: "0.85rem" }}
            >
              {locale === "zh" ? "模型" : "Model"}
            </label>
            <input
              id="glm-model"
              className="settings-input"
              autoComplete="off"
              placeholder="GLM-5.1"
              value={glmModelInput}
              onChange={(e) => setGlmModelInput(e.target.value)}
            />

            <div
              style={{
                display: "flex",
                gap: "0.55rem",
                flexWrap: "wrap",
                marginTop: "0.85rem",
              }}
            >
              <button
                type="button"
                className="btn-icon"
                disabled={busy}
                onClick={onGlmSave}
              >
                {locale === "zh" ? "保存" : "Save"}
              </button>
              <button
                type="button"
                className="btn-icon"
                disabled={busy}
                onClick={onGlmTest}
              >
                {locale === "zh" ? "测试连接" : "Test connection"}
              </button>
            </div>

            {glmSaveHint ? (
              <p className="muted" style={{ marginTop: "0.55rem", fontSize: "0.85rem" }}>
                {glmSaveHint}
              </p>
            ) : null}
            {glmTestHint ? (
              <p className="muted" style={{ marginTop: "0.55rem", fontSize: "0.85rem" }}>
                {glmTestHint}
              </p>
            ) : null}
          </>
        )}
      </section>

      <section style={{ marginTop: "2rem" }}>
        <div className="settings-block-head">
          <h3 className="settings-block-title">{locale === "zh" ? "Gitee 云备份" : "Gitee Cloud Backup"}</h3>
          <InfoTooltip
            label={locale === "zh" ? "Gitee 云备份说明" : "About Gitee backup"}
            content={
              locale === "zh"
                ? "先在 Gitee 第三方应用页面创建应用，把下方回调地址原样填入并勾选仓库相关权限（如 projects）。保存 Client ID 与 Secret 后，点击在 Gitee 授权完成登录。应用会创建或复用指定仓库，并将提示词库与资源库 JSON 同步到 aicontrols-data/。授权后会立即备份一次；运行期间每 5 分钟检查本地文件，仅在变更时上传。"
                : "Create a Gitee OAuth app, use the callback URL below, then save Client ID/Secret and authorize. The app syncs prompt/resource JSON into repository folder aicontrols-data/."
            }
          />
        </div>
        <p className="muted" style={{ margin: "0 0 1rem" }}>
          {locale === "zh" ? "在" : "Create and authorize in "}
          <a href="https://gitee.com/oauth/applications" target="_blank" rel="noreferrer">
            {locale === "zh" ? "Gitee 第三方应用" : "Gitee OAuth Applications"}
          </a>{" "}
          {locale === "zh"
            ? "创建应用并完成授权后即可自动备份。"
            : "to enable automatic backup."}
        </p>

        {giteePublic ? (
          <p className="muted" style={{ margin: "0 0 0.75rem", fontSize: "0.85rem" }}>
            {locale === "zh" ? "状态：" : "Status: "}
            {giteePublic.connected
              ? locale === "zh"
                ? `已授权（${giteePublic.ownerLogin ?? "?"} / ${giteePublic.repoName ?? "?"})`
                : `Authorized (${giteePublic.ownerLogin ?? "?"} / ${giteePublic.repoName ?? "?"})`
              : giteePublic.appConfigured
                ? locale === "zh"
                  ? "已保存应用凭据，尚未授权"
                  : "App credentials saved, not authorized"
                : locale === "zh"
                  ? "未配置"
                  : "Not configured"}
          </p>
        ) : null}

        <label
          htmlFor="gitee-callback"
          style={{ display: "block", marginBottom: "0.35rem", fontSize: "0.85rem" }}
        >
          {locale === "zh"
            ? "回调地址（须与 Gitee 应用配置一致）"
            : "Callback URL (must match Gitee app config)"}
        </label>
        <input
          id="gitee-callback"
          readOnly
          className="settings-input"
          style={{ marginBottom: "0.85rem" }}
          value={giteePublic?.oauthCallbackUrl ?? "http://127.0.0.1:19876/oauth/gitee/callback"}
          onFocus={(e) => e.currentTarget.select()}
        />

        <label
          htmlFor="gitee-client-id"
          style={{ display: "block", marginBottom: "0.35rem", fontSize: "0.85rem" }}
        >
          Client ID
        </label>
        <input
          id="gitee-client-id"
          className="settings-input"
          autoComplete="off"
          placeholder={locale === "zh" ? "OAuth 应用 Client ID" : "OAuth Client ID"}
          value={giteeClientId}
          onChange={(e) => setGiteeClientId(e.target.value)}
        />

        <label
          htmlFor="gitee-secret"
          style={{ display: "block", margin: "0.75rem 0 0.35rem", fontSize: "0.85rem" }}
        >
          Client Secret
        </label>
        <input
          id="gitee-secret"
          type="password"
          className="settings-input"
          autoComplete="off"
          placeholder={
            giteePublic?.appConfigured
              ? locale === "zh"
                ? "留空则保留已保存的 Secret；修改时请填写新值"
                : "Leave empty to keep existing secret"
              : locale === "zh"
                ? "OAuth 应用密钥"
                : "OAuth app secret"
          }
          value={giteeSecret}
          onChange={(e) => setGiteeSecret(e.target.value)}
        />

        <label
          htmlFor="gitee-repo"
          style={{ display: "block", margin: "0.75rem 0 0.35rem", fontSize: "0.85rem" }}
        >
          {locale === "zh" ? "备份仓库名" : "Backup repository"}
        </label>
        <input
          id="gitee-repo"
          className="settings-input"
          autoComplete="off"
          placeholder={
            locale === "zh"
              ? "默认 aicontrols-backup（仅小写字母、数字、-、_）"
              : "Default aicontrols-backup (a-z, 0-9, -, _)"
          }
          value={giteeRepo}
          onChange={(e) => setGiteeRepo(e.target.value)}
        />

        <label
          htmlFor="gitee-restore-url"
          style={{ display: "block", margin: "0.75rem 0 0.35rem", fontSize: "0.85rem" }}
        >
          {locale === "zh" ? "载入仓库地址（恢复）" : "Repository URL (restore)"}
        </label>
        <input
          id="gitee-restore-url"
          className="settings-input"
          autoComplete="off"
          placeholder={
            locale === "zh"
              ? "https://gitee.com/<owner>/<repo> 或 .../tree/<branch>/aicontrols-data"
              : "https://gitee.com/<owner>/<repo> or .../tree/<branch>/aicontrols-data"
          }
          value={giteeRepoUrlInput}
          onChange={(e) => setGiteeRepoUrlInput(e.target.value)}
        />

        <div
          style={{
            display: "flex",
            gap: "0.55rem",
            flexWrap: "wrap",
            marginTop: "0.85rem",
          }}
        >
          <button
            type="button"
            className="btn-icon"
            disabled={busy}
            onClick={onGiteeSave}
          >
            {locale === "zh" ? "保存 Gitee 配置" : "Save Gitee config"}
          </button>
          <button
            type="button"
            className="btn-icon"
            disabled={busy || !giteePublic?.appConfigured}
            onClick={onGiteeAuth}
          >
            {locale === "zh" ? "在 Gitee 授权" : "Authorize on Gitee"}
          </button>
          <button
            type="button"
            className="btn-icon"
            disabled={busy || !giteePublic?.connected}
            onClick={onGiteeBackup}
          >
            {locale === "zh" ? "立即备份" : "Backup now"}
          </button>
          <button
            type="button"
            className="btn-icon"
            disabled={busy || !giteePublic?.connected}
            onClick={onGiteeDisconnect}
          >
            {locale === "zh" ? "解除授权" : "Disconnect"}
          </button>
          <button
            type="button"
            className="btn-icon"
            disabled={busy || !giteePublic?.connected || giteeRepoUrlInput.trim().length === 0}
            onClick={onGiteeRestore}
          >
            {locale === "zh" ? "从仓库载入" : "Restore from repo"}
          </button>
        </div>

        {giteeHint ? (
          <p className="muted" style={{ marginTop: "0.55rem", fontSize: "0.85rem" }}>
            {giteeHint}
          </p>
        ) : null}
      </section>
    </div>
  );
}
