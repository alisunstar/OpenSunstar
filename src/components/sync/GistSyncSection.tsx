import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Github, Upload, Download, Trash2, CheckCircle, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

interface ConnectionStatus {
  connected: boolean;
  username?: string;
  gistId?: string;
}

export function GistSyncSection() {
  const { t } = useTranslation();
  const [pat, setPat] = useState("");
  const [status, setStatus] = useState<ConnectionStatus | null>(null);
  const [configured, setConfigured] = useState(false);
  const [loading, setLoading] = useState<string | null>(null);

  useEffect(() => {
    invoke<boolean>("gist_sync_is_configured").then(setConfigured).catch(() => {});
  }, []);

  const testConnection = useCallback(async () => {
    setLoading("test");
    try {
      const result = await invoke<{ status: string; username: string; gist_id: string }>("gist_sync_test_connection");
      setStatus({ connected: true, username: result.username, gistId: result.gist_id });
      toast.success(
        t("gistSync.connectedAs", {
          username: result.username,
          defaultValue: "已连接为 {{username}}",
        }),
      );
    } catch (e) {
      setStatus({ connected: false });
      toast.error(
        t("gistSync.connectionFailed", {
          error: String(e),
          defaultValue: "连接失败：{{error}}",
        }),
      );
    } finally {
      setLoading(null);
    }
  }, [t]);

  const savePat = useCallback(async () => {
    if (!pat.trim()) {
      toast.error(
        t("gistSync.enterPat", { defaultValue: "请输入 GitHub PAT" }),
      );
      return;
    }
    setLoading("save");
    try {
      await invoke("gist_sync_save_pat", { pat: pat.trim() });
      setConfigured(true);
      setPat("");
      toast.success(
        t("gistSync.patSaved", { defaultValue: "GitHub PAT 已安全保存" }),
      );
      await testConnection();
    } catch (e) {
      toast.error(
        t("gistSync.savePatFailed", {
          error: String(e),
          defaultValue: "保存 PAT 失败：{{error}}",
        }),
      );
    } finally {
      setLoading(null);
    }
  }, [pat, t, testConnection]);

  const upload = useCallback(async () => {
    setLoading("upload");
    try {
      const result = await invoke<{ status: string; gist_id: string }>("gist_sync_upload");
      toast.success(
        t("gistSync.uploadSuccess", {
          gistId: result.gist_id.slice(0, 8),
          defaultValue: "已上传到 Gist {{gistId}}...",
        }),
      );
      setStatus((prev) => prev ? { ...prev, gistId: result.gist_id } : prev);
    } catch (e) {
      toast.error(
        t("gistSync.uploadFailed", {
          error: String(e),
          defaultValue: "上传失败：{{error}}",
        }),
      );
    } finally {
      setLoading(null);
    }
  }, [t]);

  const download = useCallback(async () => {
    setLoading("download");
    try {
      const result = await invoke<{ status: string; gist_id: string; device_name: string }>("gist_sync_download");
      toast.success(
        t("gistSync.downloadSuccess", {
          device: result.device_name || t("gistSync.remoteDevice", { defaultValue: "远程设备" }),
          defaultValue: "已从 {{device}} 下载",
        }),
      );
    } catch (e) {
      toast.error(
        t("gistSync.downloadFailed", {
          error: String(e),
          defaultValue: "下载失败：{{error}}",
        }),
      );
    } finally {
      setLoading(null);
    }
  }, [t]);

  const clearConfig = useCallback(async () => {
    try {
      await invoke("gist_sync_clear");
      setConfigured(false);
      setStatus(null);
      toast.success(
        t("gistSync.configCleared", { defaultValue: "Gist 同步配置已清除" }),
      );
    } catch (e) {
      toast.error(
        t("gistSync.clearFailed", {
          error: String(e),
          defaultValue: "清除失败：{{error}}",
        }),
      );
    }
  }, [t]);

  return (
    <div className="space-y-4">
      <div>
        <h4 className="text-sm font-semibold">
          {t("gistSync.title", { defaultValue: "GitHub Gist 同步" })}
        </h4>
        <p className="text-sm text-muted-foreground mt-1">
          {t("gistSync.description", {
            defaultValue: "通过私有 GitHub Gist 同步设置与 Skills",
          })}
        </p>
      </div>

      <div className="flex items-center gap-2 text-sm text-muted-foreground">
        <Github className="w-4 h-4" />
        <span>{t("gistSync.sectionLabel", { defaultValue: "GitHub Gist 同步" })}</span>
        {status?.connected && (
          <span className="flex items-center gap-1 text-green-600 dark:text-green-400">
            <CheckCircle className="w-3 h-3" />
            {status.username}
          </span>
        )}
      </div>

      {!configured ? (
        <div className="space-y-3">
          <p className="text-sm text-muted-foreground">
            {t("gistSync.patHint", {
              defaultValue:
                "输入具有 gist 权限的 GitHub Personal Access Token 以启用 Gist 同步。",
            })}
          </p>
          <div className="flex gap-2">
            <Input
              type="password"
              placeholder="ghp_xxxxxxxxxxxx"
              value={pat}
              onChange={(e) => setPat(e.target.value)}
              className="flex-1"
            />
            <Button onClick={savePat} disabled={loading === "save" || !pat.trim()}>
              {loading === "save" ? (
                <Loader2 className="w-4 h-4 animate-spin" />
              ) : (
                t("gistSync.save", { defaultValue: "保存" })
              )}
            </Button>
          </div>
        </div>
      ) : (
        <div className="space-y-3">
          {status?.gistId && (
            <p className="text-xs text-muted-foreground">
              {t("gistSync.gistIdLabel", { defaultValue: "Gist ID" })}:{" "}
              <code className="px-1 py-0.5 rounded bg-muted">
                {status.gistId.slice(0, 12)}...
              </code>
            </p>
          )}
          <div className="flex flex-wrap gap-2">
            <Button variant="outline" size="sm" onClick={testConnection} disabled={!!loading}>
              {loading === "test" ? (
                <Loader2 className="w-4 h-4 mr-1 animate-spin" />
              ) : (
                <CheckCircle className="w-4 h-4 mr-1" />
              )}
              {t("gistSync.test", { defaultValue: "测试连接" })}
            </Button>
            <Button variant="outline" size="sm" onClick={upload} disabled={!!loading}>
              {loading === "upload" ? (
                <Loader2 className="w-4 h-4 mr-1 animate-spin" />
              ) : (
                <Upload className="w-4 h-4 mr-1" />
              )}
              {t("gistSync.upload", { defaultValue: "上传" })}
            </Button>
            <Button variant="outline" size="sm" onClick={download} disabled={!!loading}>
              {loading === "download" ? (
                <Loader2 className="w-4 h-4 mr-1 animate-spin" />
              ) : (
                <Download className="w-4 h-4 mr-1" />
              )}
              {t("gistSync.download", { defaultValue: "下载" })}
            </Button>
            <Button variant="ghost" size="sm" onClick={clearConfig} disabled={!!loading} className="text-destructive hover:text-destructive">
              <Trash2 className="w-4 h-4 mr-1" />
              {t("gistSync.clear", { defaultValue: "清除" })}
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
