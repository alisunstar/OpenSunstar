import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { Sparkles, ChevronRight, ChevronLeft, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

const ONBOARDING_KEY = "os_dashboard_onboarding_done";

interface Step {
  emoji: string;
  titleKey: string;
  titleDefault: string;
  descKey: string;
  descDefault: string;
  /** 步骤附带动作按钮时传入点击回调。 */
  actionLabel?: string;
}

function buildSteps(t: ReturnType<typeof useTranslation>["t"]): Step[] {
  return [
    {
      emoji: "📁",
      titleKey: "onboarding.step1.title",
      titleDefault: "添加项目",
      descKey: "onboarding.step1.desc",
      descDefault:
        "所有功能围绕项目展开。点击侧边栏「我的项目 → 添加项目」，把你的代码仓库加进来，OpenSunstar 会自动扫描 AI 配置状态。",
      actionLabel: t("sidebar.addProject", { defaultValue: "添加项目" }),
    },
    {
      emoji: "🚀",
      titleKey: "onboarding.step2.title",
      titleDefault: "项目驾驶舱 — 今天有没有事",
      descKey: "onboarding.step2.desc",
      descDefault:
        "「今日告警」只回答一个问题：今天有没有事。没事就显示「今天没事」—— 没事就是最好的消息。「项目看板」放聚合指标与资产矩阵，巡视内容不占开机第一眼。",
    },
    {
      emoji: "✨",
      titleKey: "onboarding.step3.title",
      titleDefault: "AI资产配置 — 为项目落地资产",
      descKey: "onboarding.step3.desc",
      descDefault:
        "在「项目配置 → AI资产配置」里为项目关联 MCP 服务器、Skills、Prompts 等资产。侧栏「Agent 配置」管全局库，这里管它们在项目上的关联。",
    },
    {
      emoji: "🔧",
      titleKey: "onboarding.step4.title",
      titleDefault: "工作流编排 — 建立可执行流程",
      descKey: "onboarding.step4.desc",
      descDefault:
        "在「项目配置 → 工作流编排」里为项目配置阶段、预设与变更执行方案。方法论识别、预设编排、自定义编排、设计合约——四个独立维度，按需选用，无先后依赖。",
    },
  ];
}

interface DashboardOnboardingProps {
  onAddProject?: () => void;
}

export function DashboardOnboarding({
  onAddProject,
}: DashboardOnboardingProps) {
  const { t } = useTranslation();
  const [visible, setVisible] = useState(false);
  const [step, setStep] = useState(0);

  const steps = buildSteps(t);

  useEffect(() => {
    try {
      const done = localStorage.getItem(ONBOARDING_KEY);
      if (!done) setVisible(true);
    } catch {
      // localStorage unavailable — show onboarding by default
      setVisible(true);
    }
  }, []);

  const dismiss = () => {
    setVisible(false);
    try {
      localStorage.setItem(ONBOARDING_KEY, "1");
    } catch {
      // ignore
    }
  };

  if (!visible) return null;

  const current = steps[step];
  const isFirst = step === 0;
  const isLast = step === steps.length - 1;

  const handleStepAction = () => {
    if (step === 0 && onAddProject) {
      onAddProject();
    }
  };

  return (
    <div className="rounded-xl border border-primary/30 bg-primary/5 p-4 space-y-3 relative">
      {/* Close button */}
      <button
        type="button"
        className="absolute right-3 top-3 text-muted-foreground/60 hover:text-foreground transition-colors"
        onClick={dismiss}
        aria-label={t("onboarding.dismiss", { defaultValue: "关闭引导" })}
      >
        <X className="w-4 h-4" />
      </button>

      {/* Header */}
      <div className="flex items-center gap-2">
        <Sparkles className="w-4 h-4 text-primary shrink-0" />
        <h3 className="text-sm font-semibold text-foreground">
          {t("onboarding.title", { defaultValue: "快速上手" })}
        </h3>
        <span className="text-[10px] text-muted-foreground tabular-nums">
          {step + 1}/{steps.length}
        </span>
      </div>

      {/* Current step */}
      <div className="space-y-1">
        <p className="text-sm font-medium text-foreground">
          <span className="mr-1.5">{current.emoji}</span>
          {t(current.titleKey, { defaultValue: current.titleDefault })}
        </p>
        <p className="text-xs text-muted-foreground leading-relaxed">
          {t(current.descKey, { defaultValue: current.descDefault })}
        </p>
      </div>

      {/* Step action button (only step 0: 添加项目) */}
      {current.actionLabel && step === 0 && (
        <Button
          variant="default"
          size="sm"
          className="h-7 text-xs"
          onClick={handleStepAction}
        >
          {current.actionLabel}
        </Button>
      )}

      {/* Navigation */}
      <div className="flex items-center justify-between">
        <Button
          variant="ghost"
          size="sm"
          className="h-7 text-xs"
          disabled={isFirst}
          onClick={() => setStep((s) => s - 1)}
        >
          <ChevronLeft className="w-3.5 h-3.5 mr-0.5" />
          {t("onboarding.prev", { defaultValue: "上一步" })}
        </Button>

        <div className="flex items-center gap-1.5">
          {steps.map((_, i) => (
            <span
              key={i}
              className={cn(
                "w-1.5 h-1.5 rounded-full transition-colors",
                i === step ? "bg-primary" : "bg-muted-foreground/30",
              )}
            />
          ))}
        </div>

        {isLast ? (
          <Button
            variant="default"
            size="sm"
            className="h-7 text-xs"
            onClick={dismiss}
          >
            {t("onboarding.done", { defaultValue: "知道了" })}
          </Button>
        ) : (
          <Button
            variant="ghost"
            size="sm"
            className="h-7 text-xs"
            onClick={() => setStep((s) => s + 1)}
          >
            {t("onboarding.next", { defaultValue: "下一步" })}
            <ChevronRight className="w-3.5 h-3.5 ml-0.5" />
          </Button>
        )}
      </div>
    </div>
  );
}
