import { Check } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";

interface WizardStepperProps {
  step: number;
  onStepClick?: (step: number) => void;
}

const STEPS = [
  { id: 1, labelKey: "simpleConnect.step1", fallback: "供应商" },
  { id: 2, labelKey: "simpleConnect.step2", fallback: "密钥" },
  { id: 3, labelKey: "simpleConnect.step3", fallback: "应用" },
] as const;

export function WizardStepper({ step, onStepClick }: WizardStepperProps) {
  const { t } = useTranslation();

  return (
    <nav
      aria-label={t("simpleConnect.wizardNav", { defaultValue: "快速接入步骤" })}
      className="w-full"
    >
      <ol className="flex items-center gap-2">
        {STEPS.map((item, index) => {
          const done = step > item.id;
          const active = step === item.id;
          const clickable = done && onStepClick;

          return (
            <li key={item.id} className="flex flex-1 items-center gap-2 min-w-0">
              <button
                type="button"
                disabled={!clickable}
                onClick={() => clickable && onStepClick(item.id)}
                className={cn(
                  "flex items-center gap-2 min-w-0 text-left transition-colors",
                  clickable && "hover:opacity-80 cursor-pointer",
                  !clickable && "cursor-default",
                )}
              >
                <span
                  className={cn(
                    "flex h-8 w-8 shrink-0 items-center justify-center rounded-full border text-xs font-semibold transition-colors",
                    done && "border-emerald-500/50 bg-emerald-500/15 text-emerald-600 dark:text-emerald-400",
                    active && "border-primary bg-primary text-primary-foreground",
                    !done && !active && "border-border/60 bg-muted/30 text-muted-foreground",
                  )}
                >
                  {done ? <Check className="h-4 w-4" /> : item.id}
                </span>
                <span
                  className={cn(
                    "hidden sm:block truncate text-sm",
                    active && "font-semibold text-foreground",
                    done && "text-foreground",
                    !done && !active && "text-muted-foreground",
                  )}
                >
                  {t(item.labelKey, { defaultValue: item.fallback })}
                </span>
              </button>
              {index < STEPS.length - 1 && (
                <div
                  className={cn(
                    "h-px flex-1 min-w-[12px] rounded-full",
                    step > item.id ? "bg-emerald-500/40" : "bg-border/60",
                  )}
                />
              )}
            </li>
          );
        })}
      </ol>
    </nav>
  );
}
