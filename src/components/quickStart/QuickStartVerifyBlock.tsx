import { useCallback, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronDown, ChevronRight, Loader2, ShieldCheck } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type { QuickStartAppId } from "@/config/quickStartCurated";
import type { FetchedModel } from "@/lib/api/model-fetch";
import type {
  QuickStartFormFields,
  QuickStartSelection,
} from "@/lib/quickStart/types";
import { verifyQuickStartKey } from "@/lib/quickStart/verify";

interface QuickStartVerifyBlockProps {
  appId: QuickStartAppId;
  selection: QuickStartSelection;
  fields: QuickStartFormFields;
  onVerificationChange?: (verified: boolean) => void;
}

export function QuickStartVerifyBlock({
  appId,
  selection,
  fields,
  onVerificationChange,
}: QuickStartVerifyBlockProps) {
  const { t } = useTranslation();
  const [verifying, setVerifying] = useState(false);
  const [message, setMessage] = useState<string | null>(null);
  const [ok, setOk] = useState<boolean | null>(null);
  const [models, setModels] = useState<FetchedModel[]>([]);
  const [modelsOpen, setModelsOpen] = useState(false);

  const handleVerify = useCallback(async () => {
    setVerifying(true);
    setMessage(null);
    setOk(null);
    setModels([]);
    try {
      const outcome = await verifyQuickStartKey(appId, selection, fields, t);
      setOk(outcome.ok);
      setMessage(outcome.message);
      setModels(outcome.models);
      onVerificationChange?.(outcome.ok);
      if (outcome.models.length > 0) {
        setModelsOpen(true);
      }
    } catch (error) {
      setOk(false);
      setMessage(String(error));
      onVerificationChange?.(false);
    } finally {
      setVerifying(false);
    }
  }, [appId, selection, fields, onVerificationChange, t]);

  return (
    <div className="space-y-2">
      <div className="flex flex-wrap items-center gap-2">
        <Button
          variant="outline"
          size="sm"
          onClick={handleVerify}
          disabled={verifying || !fields.apiKey.trim()}
        >
          {verifying ? (
            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
          ) : (
            <ShieldCheck className="mr-2 h-4 w-4" />
          )}
          {t("quickStart.verify", { defaultValue: "验证 Key" })}
        </Button>
        {message && (
          <span
            className={cn(
              "text-xs",
              ok
                ? "text-green-600 dark:text-green-400"
                : "text-red-600 dark:text-red-400",
            )}
          >
            {message}
          </span>
        )}
      </div>
      <p className="text-xs text-muted-foreground">
        {t("quickStart.verifyHint", {
          defaultValue:
            "Anthropic 协议验证会消耗约 1 token；OpenAI 兼容网关验证成功后可展开模型列表",
        })}
      </p>
      {models.length > 0 && (
        <div className="rounded-md border border-border">
          <button
            type="button"
            className="flex w-full items-center gap-2 px-3 py-2 text-left text-xs font-medium"
            onClick={() => setModelsOpen((v) => !v)}
          >
            {modelsOpen ? (
              <ChevronDown className="h-3.5 w-3.5" />
            ) : (
              <ChevronRight className="h-3.5 w-3.5" />
            )}
            {t("quickStart.modelsList", {
              count: models.length,
              defaultValue: `可用模型（${models.length}）`,
            })}
          </button>
          {modelsOpen && (
            <ul className="max-h-40 overflow-y-auto border-t border-border px-3 py-2 text-xs font-mono text-muted-foreground">
              {models.slice(0, 50).map((m) => (
                <li key={m.id} className="truncate py-0.5">
                  {m.id}
                </li>
              ))}
              {models.length > 50 && (
                <li className="py-1 text-muted-foreground/80">…</li>
              )}
            </ul>
          )}
        </div>
      )}
    </div>
  );
}
