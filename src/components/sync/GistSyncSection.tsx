import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
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
      toast.success(`Connected as ${result.username}`);
    } catch (e) {
      setStatus({ connected: false });
      toast.error(`Connection failed: ${e}`);
    } finally {
      setLoading(null);
    }
  }, []);

  const savePat = useCallback(async () => {
    if (!pat.trim()) {
      toast.error("Please enter a GitHub PAT");
      return;
    }
    setLoading("save");
    try {
      await invoke("gist_sync_save_pat", { pat: pat.trim() });
      setConfigured(true);
      setPat("");
      toast.success("GitHub PAT saved securely");
      await testConnection();
    } catch (e) {
      toast.error(`Failed to save PAT: ${e}`);
    } finally {
      setLoading(null);
    }
  }, [pat, testConnection]);

  const upload = useCallback(async () => {
    setLoading("upload");
    try {
      const result = await invoke<{ status: string; gist_id: string }>("gist_sync_upload");
      toast.success(`Uploaded to Gist ${result.gist_id.slice(0, 8)}...`);
      setStatus((prev) => prev ? { ...prev, gistId: result.gist_id } : prev);
    } catch (e) {
      toast.error(`Upload failed: ${e}`);
    } finally {
      setLoading(null);
    }
  }, []);

  const download = useCallback(async () => {
    setLoading("download");
    try {
      const result = await invoke<{ status: string; gist_id: string; device_name: string }>("gist_sync_download");
      toast.success(`Downloaded from ${result.device_name || "remote"}`);
    } catch (e) {
      toast.error(`Download failed: ${e}`);
    } finally {
      setLoading(null);
    }
  }, []);

  const clearConfig = useCallback(async () => {
    try {
      await invoke("gist_sync_clear");
      setConfigured(false);
      setStatus(null);
      toast.success("Gist sync configuration cleared");
    } catch (e) {
      toast.error(`Clear failed: ${e}`);
    }
  }, []);

  return (
    <div className="space-y-4">
      <div className="flex items-center gap-2 text-sm text-muted-foreground">
        <Github className="w-4 h-4" />
        <span>GitHub Gist Sync</span>
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
            Enter a GitHub Personal Access Token with <code className="px-1 py-0.5 rounded bg-muted">gist</code> scope to enable Gist sync.
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
              {loading === "save" ? <Loader2 className="w-4 h-4 animate-spin" /> : "Save"}
            </Button>
          </div>
        </div>
      ) : (
        <div className="space-y-3">
          {status?.gistId && (
            <p className="text-xs text-muted-foreground">
              Gist ID: <code className="px-1 py-0.5 rounded bg-muted">{status.gistId.slice(0, 12)}...</code>
            </p>
          )}
          <div className="flex flex-wrap gap-2">
            <Button variant="outline" size="sm" onClick={testConnection} disabled={!!loading}>
              {loading === "test" ? <Loader2 className="w-4 h-4 mr-1 animate-spin" /> : <CheckCircle className="w-4 h-4 mr-1" />}
              Test
            </Button>
            <Button variant="outline" size="sm" onClick={upload} disabled={!!loading}>
              {loading === "upload" ? <Loader2 className="w-4 h-4 mr-1 animate-spin" /> : <Upload className="w-4 h-4 mr-1" />}
              Upload
            </Button>
            <Button variant="outline" size="sm" onClick={download} disabled={!!loading}>
              {loading === "download" ? <Loader2 className="w-4 h-4 mr-1 animate-spin" /> : <Download className="w-4 h-4 mr-1" />}
              Download
            </Button>
            <Button variant="ghost" size="sm" onClick={clearConfig} disabled={!!loading} className="text-destructive hover:text-destructive">
              <Trash2 className="w-4 h-4 mr-1" />
              Clear
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
