import { useState } from "react";
import { useTranslation } from "react-i18next";
import { CheckCircle2, Plus, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import type { PoolKeyMeta } from "@/lib/api/simpleConnect";
import { PoolHealthPanel } from "./PoolHealthPanel";
import { BackupAuditPanel } from "./BackupAuditPanel";
import { PanelShell, SectionHeader, SC_INNER } from "./ui";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";
import { ChevronDown, KeyRound, Settings2 } from "lucide-react";

interface KeyPoolPanelProps {
  supplierId: string;
  poolEnabled: boolean;
  keys: PoolKeyMeta[];
  primaryKeyHint?: string | null;
  requireKeyVerify?: boolean;
  deeplinkImportEnabled?: boolean;
  onPoolEnabledChange: (enabled: boolean) => void;
  onRequireKeyVerifyChange?: (enabled: boolean) => void;
  onDeeplinkImportEnabledChange?: (enabled: boolean) => void;
  onKeysChange: (keys: PoolKeyMeta[]) => void;
  onStoreKey: (keyId: string, secret: string) => Promise<string | void>;
  onRemoveKey: (keyId: string) => Promise<void>;
}

export function KeyPoolPanel({
  supplierId,
  poolEnabled,
  keys,
  primaryKeyHint,
  requireKeyVerify = true,
  deeplinkImportEnabled = true,
  onPoolEnabledChange,
  onRequireKeyVerifyChange,
  onDeeplinkImportEnabledChange,
  onKeysChange,
  onStoreKey,
  onRemoveKey,
}: KeyPoolPanelProps) {
  const { t } = useTranslation();
  const [primaryKey, setPrimaryKey] = useState("");
  const [extraKey, setExtraKey] = useState("");
  const [saving, setSaving] = useState(false);
  const [savedHint, setSavedHint] = useState<string | null>(primaryKeyHint ?? null);

  const handleSavePrimary = async () => {
    if (!primaryKey.trim()) return;
    setSaving(true);
    try {
      const hint = await onStoreKey("primary", primaryKey.trim());
      setPrimaryKey("");
      if (typeof hint === "string") setSavedHint(hint);
      const exists = keys.some((k) => k.id === "primary");
      if (!exists) {
        onKeysChange([
          ...keys,
          {
            id: "primary",
            label: t("simpleConnect.pool.primary", { defaultValue: "主 Key" }),
            weight: 1,
            enabled: true,
          },
        ]);
      }
      toast.success(
        t("simpleConnect.keySavedToast", {
          defaultValue: "API Key 已安全存入 Keychain",
        }),
      );
    } catch (e) {
      toast.error(String(e));
    } finally {
      setSaving(false);
    }
  };

  const handleAddExtra = async () => {
    if (!extraKey.trim()) return;
    const id = `k${Date.now()}`;
    setSaving(true);
    try {
      await onStoreKey(id, extraKey.trim());
      setExtraKey("");
      onKeysChange([
        ...keys,
        {
          id,
          label: t("simpleConnect.pool.extra", { defaultValue: "备用 Key" }),
          weight: 1,
          enabled: true,
        },
      ]);
      toast.success(
        t("simpleConnect.poolKeyAdded", { defaultValue: "备用 Key 已添加" }),
      );
    } catch (e) {
      toast.error(String(e));
    } finally {
      setSaving(false);
    }
  };

  const handleRemove = async (keyId: string) => {
    if (keyId === "primary") return;
    await onRemoveKey(keyId);
    onKeysChange(keys.filter((k) => k.id !== keyId));
  };

  const updateKeyMeta = (keyId: string, patch: Partial<PoolKeyMeta>) => {
    onKeysChange(
      keys.map((k) => (k.id === keyId ? { ...k, ...patch } : k)),
    );
  };

  const displayHint = savedHint ?? primaryKeyHint;

  return (
    <PanelShell>
      <SectionHeader
        icon={KeyRound}
        title={t("simpleConnect.step2", { defaultValue: "密钥" })}
        description={t("simpleConnect.pool.step2Hint", {
          defaultValue:
            "输入 API Key 并保存，CLI 将通过本地代理安全访问",
        })}
      />

      <div className="space-y-2">
        <div className="flex items-center justify-between gap-2">
          <Label htmlFor="sc-primary-key">
            {t("simpleConnect.pool.primaryKey", { defaultValue: "API Key" })}
          </Label>
          {displayHint && (
            <Badge variant="outline" className="font-mono text-[10px] font-normal">
              {displayHint}
            </Badge>
          )}
        </div>
        <div className="flex gap-2">
          <Input
            id="sc-primary-key"
            type="password"
            autoComplete="off"
            placeholder={t("simpleConnect.pool.keyPlaceholder", {
              defaultValue: "sk-...",
            })}
            value={primaryKey}
            onChange={(e) => setPrimaryKey(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") void handleSavePrimary();
            }}
          />
          <Button
            type="button"
            disabled={saving || !primaryKey.trim()}
            onClick={() => void handleSavePrimary()}
          >
            {t("common.save", { defaultValue: "保存" })}
          </Button>
        </div>
        {displayHint && !primaryKey && (
          <p className="flex items-center gap-1.5 text-xs text-emerald-600 dark:text-emerald-400">
            <CheckCircle2 className="h-3.5 w-3.5" />
            {t("simpleConnect.keyReadyHint", {
              defaultValue: "主 Key 已就绪，可进入下一步",
            })}
          </p>
        )}
      </div>

      <Collapsible defaultOpen={false}>
        <CollapsibleTrigger className="flex w-full items-center justify-between rounded-lg border border-border/40 px-3 py-2.5 text-sm hover:bg-muted/30 transition-colors">
          <span className="flex items-center gap-2 font-medium text-muted-foreground">
            <Settings2 className="h-4 w-4" />
            {t("simpleConnect.advanced.title", { defaultValue: "高级" })}
          </span>
          <ChevronDown className="h-4 w-4 text-muted-foreground transition-transform [[data-state=open]_&]:rotate-180" />
        </CollapsibleTrigger>
        <CollapsibleContent className="space-y-4 pt-4 data-[state=closed]:animate-out data-[state=open]:animate-in">
          {/* 密钥池开关 */}
          <div className="flex items-center justify-between gap-4">
            <div>
              <Label className="text-sm font-medium">
                {t("simpleConnect.pool.enable", { defaultValue: "开启密钥池" })}
              </Label>
              <p className="text-xs text-muted-foreground mt-1">
                {t("simpleConnect.pool.enableHint", {
                  defaultValue:
                    "多 Key 加权轮询 + 429 阶梯冷却；CLI 始终写 local token，真实 Key 仅存 Keychain",
                })}
              </p>
            </div>
            <Switch checked={poolEnabled} onCheckedChange={onPoolEnabledChange} />
          </div>

          {poolEnabled && (
            <>
              {/* 备用 Key 管理 */}
              <div className="space-y-2">
                <Label>
                  {t("simpleConnect.pool.extraKey", {
                    defaultValue: "添加备用 Key",
                  })}
                </Label>
                <div className="flex gap-2">
                  <Input
                    type="password"
                    autoComplete="off"
                    placeholder={t("simpleConnect.pool.keyPlaceholder", {
                      defaultValue: "sk-...",
                    })}
                    value={extraKey}
                    onChange={(e) => setExtraKey(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") void handleAddExtra();
                    }}
                  />
                  <Button
                    type="button"
                    variant="outline"
                    disabled={saving || !extraKey.trim()}
                    onClick={() => void handleAddExtra()}
                  >
                    <Plus className="h-4 w-4" />
                  </Button>
                </div>
              </div>

              {keys.length > 0 && (
                <ul className="space-y-2">
                  {keys.map((k) => (
                    <li
                      key={k.id}
                      className="flex flex-wrap items-center gap-3 rounded-lg border border-border/50 bg-background/60 px-3 py-2.5"
                    >
                      <div className="min-w-0 flex-1">
                        <p className="text-sm font-medium">
                          {k.label}
                          {k.id === "primary" && (
                            <span className="text-muted-foreground font-normal">
                              {" "}
                              ({t("simpleConnect.pool.required", { defaultValue: "必需" })})
                            </span>
                          )}
                        </p>
                        <p className="text-[11px] text-muted-foreground font-mono truncate">
                          {k.id}
                        </p>
                      </div>
                      <div className="flex items-center gap-2">
                        <Label className="text-[11px] text-muted-foreground sr-only">
                          weight
                        </Label>
                        <Input
                          type="number"
                          min={1}
                          max={99}
                          className="h-8 w-16 text-center"
                          value={k.weight}
                          onChange={(e) =>
                            updateKeyMeta(k.id, {
                              weight: Math.max(1, Number(e.target.value) || 1),
                            })
                          }
                        />
                        <Switch
                          checked={k.enabled}
                          onCheckedChange={(enabled) =>
                            updateKeyMeta(k.id, { enabled })
                          }
                          aria-label={k.label}
                        />
                      </div>
                      {k.id !== "primary" && (
                        <Button
                          type="button"
                          variant="ghost"
                          size="icon"
                          className="shrink-0"
                          onClick={() => void handleRemove(k.id)}
                        >
                          <Trash2 className="h-4 w-4" />
                        </Button>
                      )}
                    </li>
                  ))}
                </ul>
              )}

              {/* 密钥池运行态 */}
              <div className={`${SC_INNER} p-4`}>
                <PoolHealthPanel enabled pollMs={2500} embedded />
              </div>
            </>
          )}

          <div className="flex items-center justify-between gap-4">
            <div>
              <Label className="text-sm">
                {t("simpleConnect.advanced.verifyKey", {
                  defaultValue: "保存前校验 Key",
                })}
              </Label>
              <p className="text-[11px] text-muted-foreground mt-0.5">
                {t("simpleConnect.advanced.verifyKeyHint", {
                  defaultValue: "调用 /v1/models 验证；关闭后仅高级用户手动导入",
                })}
              </p>
            </div>
            <Switch
              checked={requireKeyVerify}
              onCheckedChange={(v) => onRequireKeyVerifyChange?.(v)}
            />
          </div>
          <div className="flex items-center justify-between gap-4">
            <div>
              <Label className="text-sm">
                {t("simpleConnect.advanced.deeplinkImport", {
                  defaultValue: "允许 URL 导入 Key",
                })}
              </Label>
              <p className="text-[11px] text-muted-foreground mt-0.5">
                {t("simpleConnect.advanced.deeplinkImportHint", {
                  defaultValue: "beeapi:// / OpenSunstar://simple-connect/import",
                })}
              </p>
            </div>
            <Switch
              checked={deeplinkImportEnabled}
              onCheckedChange={(v) => onDeeplinkImportEnabledChange?.(v)}
            />
          </div>
          <BackupAuditPanel embedded />
        </CollapsibleContent>
      </Collapsible>

      <p className="text-[11px] text-muted-foreground font-mono">
        {t("simpleConnect.pool.supplierHint", {
          supplier: supplierId,
          defaultValue: "Keychain: simple-connect/{{supplier}}/…",
        })}
      </p>
    </PanelShell>
  );
}
