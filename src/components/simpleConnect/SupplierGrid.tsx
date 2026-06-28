import { ExternalLink, Globe } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { cn } from "@/lib/utils";
import type { SupplierProfile } from "@/lib/api/simpleConnect";
import { SUPPLIER_ACCENTS } from "./constants";

interface SupplierGridProps {
  suppliers: SupplierProfile[];
  selectedId: string;
  customBase: string;
  onSelect: (id: string) => void;
  onCustomBaseChange: (value: string) => void;
}

export function SupplierGrid({
  suppliers,
  selectedId,
  customBase,
  onSelect,
  onCustomBaseChange,
}: SupplierGridProps) {
  const { t } = useTranslation();

  return (
    <div className="space-y-4">
      <div className="grid gap-3 sm:grid-cols-2">
        {suppliers.map((supplier) => {
          const selected = selectedId === supplier.id;
          return (
            <button
              key={supplier.id}
              type="button"
              onClick={() => onSelect(supplier.id)}
              className={cn(
                "group relative rounded-xl border p-4 text-left transition-all",
                "bg-gradient-to-br hover:shadow-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                SUPPLIER_ACCENTS[supplier.id] ?? SUPPLIER_ACCENTS.custom,
                selected && "ring-2 ring-primary shadow-sm",
              )}
            >
              <div className="flex items-start justify-between gap-2">
                <div>
                  <p className="font-semibold text-sm">{supplier.name}</p>
                  {supplier.default_model && (
                    <p className="text-xs text-muted-foreground mt-1 truncate">
                      {supplier.default_model}
                    </p>
                  )}
                  {supplier.website && (
                    <a
                      href={supplier.website}
                      target="_blank"
                      rel="noreferrer"
                      className="inline-flex items-center gap-1 text-[11px] text-primary mt-1 hover:underline"
                      onClick={(e) => e.stopPropagation()}
                    >
                      {t("simpleConnect.openWebsite", { defaultValue: "官网" })}
                      <ExternalLink className="h-3 w-3" />
                    </a>
                  )}
                </div>
              </div>
            </button>
          );
        })}

        <button
          type="button"
          onClick={() => onSelect("custom")}
          className={cn(
            "rounded-xl border p-4 text-left transition-all bg-gradient-to-br",
            SUPPLIER_ACCENTS.custom,
            selectedId === "custom" && "ring-2 ring-primary shadow-sm",
          )}
        >
          <div className="flex items-center gap-2">
            <Globe className="h-4 w-4 text-muted-foreground" />
            <p className="font-semibold text-sm">
              {t("simpleConnect.customSupplier", {
                defaultValue: "自定义 OpenAI 兼容",
              })}
            </p>
          </div>
          <p className="text-xs text-muted-foreground mt-1">
            {t("simpleConnect.customSupplierHint", {
              defaultValue: "任意 OpenAI-compatible Base URL",
            })}
          </p>
        </button>
      </div>

      {selectedId === "custom" && (
        <div className="space-y-2 animate-in fade-in slide-in-from-top-1 duration-200">
          <Label htmlFor="sc-custom-base">
            {t("simpleConnect.customBase", { defaultValue: "API Base URL" })}
          </Label>
          <Input
            id="sc-custom-base"
            placeholder="https://api.example.com/v1"
            value={customBase}
            onChange={(e) => onCustomBaseChange(e.target.value)}
          />
        </div>
      )}
    </div>
  );
}
