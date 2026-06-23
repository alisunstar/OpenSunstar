import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";
import type { StageKey } from "@/hooks/useProjectStages";

interface StagePickerProps {
  value: StageKey;
  onChange: (stage: StageKey) => void;
}

const STAGE_OPTIONS: {
  key: StageKey;
  labelKey: string;
  labelDefault: string;
  descKey: string;
  descDefault: string;
  tone: "purple" | "green" | "blue";
}[] = [
  {
    key: "mvp",
    labelKey: "board.stage.mvp.label",
    labelDefault: "MVP 阶段",
    descKey: "board.stage.mvp.desc",
    descDefault: "项目处于早期开发，尚未上线",
    tone: "purple",
  },
  {
    key: "rapid",
    labelKey: "board.stage.rapid.label",
    labelDefault: "快速迭代",
    descKey: "board.stage.rapid.desc",
    descDefault: "项目已上线，正在快速迭代",
    tone: "green",
  },
  {
    key: "stable",
    labelKey: "board.stage.stable.label",
    labelDefault: "稳定维护",
    descKey: "board.stage.stable.desc",
    descDefault: "项目进入稳定期，慢迭代维护",
    tone: "blue",
  },
];

const TONE_MAP: Record<string, string> = {
  purple:
    "bg-purple-500/10 text-purple-600 dark:text-purple-400 border-purple-500/20",
  green:
    "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border-emerald-500/20",
  blue: "bg-blue-500/10 text-blue-600 dark:text-blue-400 border-blue-500/20",
};

export function StagePicker({ value, onChange }: StagePickerProps) {
  const { t } = useTranslation();

  return (
    <div className="flex gap-2" role="radiogroup">
      {STAGE_OPTIONS.map((opt) => {
        const active = value === opt.key;
        return (
          <button
            key={opt.key}
            type="button"
            role="radio"
            aria-checked={active}
            onClick={() => onChange(opt.key)}
            className={cn(
              "flex-1 px-4 py-3 rounded-xl border text-left transition-all duration-150",
              active
                ? `${TONE_MAP[opt.tone]} border-current/30`
                : "border-border bg-muted/30 text-muted-foreground hover:bg-muted/50 hover:text-foreground",
            )}
          >
            <div className="text-sm font-semibold">
              {t(opt.labelKey, { defaultValue: opt.labelDefault })}
            </div>
            <div className="text-xs mt-0.5 opacity-70">
              {t(opt.descKey, { defaultValue: "" })}
            </div>
          </button>
        );
      })}
    </div>
  );
}

export function StageBadge({
  stage,
  className,
}: {
  stage: StageKey;
  className?: string;
}) {
  const { t } = useTranslation();

  const labels: Record<StageKey, string> = {
    mvp: t("kanban.stage.mvp.badge", { defaultValue: "MVP" }),
    rapid: t("kanban.stage.rapid.badge", { defaultValue: "已上线" }),
    stable: t("kanban.stage.stable.badge", { defaultValue: "稳定维护" }),
  };

  const tones: Record<StageKey, string> = {
    mvp: "bg-purple-500/15 text-purple-600 dark:text-purple-400",
    rapid: "bg-emerald-500/15 text-emerald-600 dark:text-emerald-400",
    stable: "bg-blue-500/15 text-blue-600 dark:text-blue-400",
  };

  return (
    <span
      className={cn(
        "inline-flex items-center px-2 py-0.5 rounded-md text-xs font-medium",
        tones[stage],
        className,
      )}
    >
      {labels[stage]}
    </span>
  );
}
