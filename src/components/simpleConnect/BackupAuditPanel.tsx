import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { FileSearch, Loader2, RefreshCw, ShieldCheck, ShieldAlert } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { simpleConnectApi, type BackupAuditReport } from "@/lib/api/simpleConnect";
import { SC_INNER } from "./ui";

interface BackupAuditPanelProps {
  embedded?: boolean;
}

export function BackupAuditPanel({ embedded }: BackupAuditPanelProps) {
  const { t } = useTranslation();
  const [report, setReport] = useState<BackupAuditReport | null>(null);
  const [loading, setLoading] = useState(false);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      setReport(await simpleConnectApi.backupAudit());
    } catch {
      setReport(null);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const content = (
    <div className="space-y-3">
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-2 text-sm font-medium">
          {report?.all_clean !== false ? (
            <ShieldCheck className="h-4 w-4 text-emerald-600" />
          ) : (
            <ShieldAlert className="h-4 w-4 text-amber-600" />
          )}
          {t("simpleConnect.backupAudit.title", {
            defaultValue: "备份目录安全扫描",
          })}
        </div>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="h-8"
          disabled={loading}
          onClick={() => void refresh()}
        >
          {loading ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin" />
          ) : (
            <RefreshCw className="h-3.5 w-3.5" />
          )}
        </Button>
      </div>

      {report && (
        <>
          <div className="flex flex-wrap gap-2 text-xs">
            <Badge variant="outline">
              {t("simpleConnect.backupAudit.files", {
                count: report.files_scanned,
                defaultValue: "已扫描 {{count}} 个文件",
              })}
            </Badge>
            <Badge variant={report.all_clean ? "secondary" : "destructive"}>
              {report.all_clean
                ? t("simpleConnect.backupAudit.clean", {
                    defaultValue: "未发现明文 Key",
                  })
                : t("simpleConnect.backupAudit.suspicious", {
                    count: report.suspicious_count,
                    defaultValue: "{{count}} 处可疑内容",
                  })}
            </Badge>
          </div>
          {!report.all_clean && report.items.length > 0 && (
            <ul className="max-h-32 overflow-y-auto space-y-1 text-[11px] font-mono text-muted-foreground">
              {report.items
                .filter((i) => i.suspicious)
                .slice(0, 8)
                .map((i) => (
                  <li key={i.path} className="truncate">
                    {i.path}
                  </li>
                ))}
            </ul>
          )}
        </>
      )}

      <p className="text-[11px] text-muted-foreground flex items-start gap-1.5">
        <FileSearch className="h-3.5 w-3.5 shrink-0 mt-0.5" />
        {t("simpleConnect.backupAudit.hint", {
          defaultValue:
            "扫描 simple-connect/backups 下 CLI 配置备份，确保无 sk- 明文落盘。",
        })}
      </p>
    </div>
  );

  if (embedded) return content;

  return <div className={`${SC_INNER} p-4`}>{content}</div>;
}
