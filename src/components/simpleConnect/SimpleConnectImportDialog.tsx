import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { Shield, KeyRound } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  simpleConnectApi,
  type SimpleConnectImportPayload,
  type SimpleConnectState,
} from "@/lib/api/simpleConnect";

interface SimpleConnectImportDialogProps {
  onImported?: (state?: SimpleConnectState) => void;
}

export function SimpleConnectImportDialog({
  onImported,
}: SimpleConnectImportDialogProps) {
  const { t } = useTranslation();
  const [payload, setPayload] = useState<SimpleConnectImportPayload | null>(
    null,
  );
  const [open, setOpen] = useState(false);
  const [importing, setImporting] = useState(false);

  useEffect(() => {
    const unlisten = listen<SimpleConnectImportPayload>(
      "simple-connect-import",
      (event) => {
        setPayload(event.payload);
        setOpen(true);
      },
    );

    const unlistenErr = listen<{ url: string; error: string }>(
      "simple-connect-import-error",
      (event) => {
        toast.error(
          t("simpleConnect.import.parseError", {
            defaultValue: "导入链接解析失败",
          }),
          { description: event.payload.error },
        );
      },
    );

    return () => {
      void unlisten.then((fn) => fn());
      void unlistenErr.then((fn) => fn());
    };
  }, [t]);

  const handleImport = async () => {
    if (!payload) return;
    setImporting(true);
    try {
      const result = await simpleConnectApi.importKeys(payload, false);
      const state = await simpleConnectApi.getState();
      toast.success(
        t("simpleConnect.import.success", {
          count: result.keysAdded,
          defaultValue: "已导入 {{count}} 个 Key 到 Keychain",
        }),
      );
      if (result.duplicates > 0) {
        toast.info(
          t("simpleConnect.import.duplicates", {
            count: result.duplicates,
            defaultValue: "跳过 {{count}} 个重复 Key",
          }),
        );
      }
      setOpen(false);
      setPayload(null);
      onImported?.(state);
    } catch (e) {
      toast.error(String(e));
    } finally {
      setImporting(false);
    }
  };

  const maskedKeys =
    payload?.keys.map((k) =>
      k.length > 8 ? `${k.slice(0, 4)}****${k.slice(-4)}` : "****",
    ) ?? [];

  return (
    <Dialog open={open && !!payload} onOpenChange={setOpen}>
      <DialogContent className="sm:max-w-md" zIndex="top">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <KeyRound className="h-5 w-5" />
            {t("simpleConnect.import.title", {
              defaultValue: "确认导入 API Key",
            })}
          </DialogTitle>
          <DialogDescription>
            {t("simpleConnect.import.description", {
              defaultValue:
                "密钥将写入 Keychain，不会出现在配置文件中。导入前会校验 Key 有效性。",
            })}
          </DialogDescription>
        </DialogHeader>

        {payload && (
          <div className="space-y-3 px-1">
            <div className="flex flex-wrap gap-2">
              {maskedKeys.map((hint, i) => (
                <Badge key={i} variant="outline" className="font-mono text-xs">
                  {hint}
                </Badge>
              ))}
            </div>
            {payload.label && (
              <p className="text-sm text-muted-foreground">
                {t("simpleConnect.import.label", { defaultValue: "标签" })}:{" "}
                {payload.label}
              </p>
            )}
            {payload.supplierId && (
              <p className="text-sm text-muted-foreground">
                {t("simpleConnect.import.supplier", {
                  defaultValue: "供应商",
                })}
                : {payload.supplierId}
              </p>
            )}
            <div className="flex items-start gap-2 rounded-lg border border-amber-500/30 bg-amber-500/5 p-3 text-xs text-muted-foreground">
              <Shield className="h-4 w-4 shrink-0 text-amber-600 mt-0.5" />
              {t("simpleConnect.import.warning", {
                defaultValue:
                  "请确认链接来源可信。可在快速接入中禁用 URL 导入。",
              })}
            </div>
          </div>
        )}

        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => setOpen(false)}
            disabled={importing}
          >
            {t("common.cancel", { defaultValue: "取消" })}
          </Button>
          <Button onClick={() => void handleImport()} disabled={importing}>
            {importing
              ? t("simpleConnect.import.importing", {
                  defaultValue: "导入中…",
                })
              : t("simpleConnect.import.confirm", {
                  defaultValue: "确认导入",
                })}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
