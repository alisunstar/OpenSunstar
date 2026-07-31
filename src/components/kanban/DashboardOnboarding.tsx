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
}

const STEPS: Step[] = [
  {
    emoji: "🟢",
    titleKey: "onboarding.step1.title",
    titleDefault: "告警制首屏 — 今天有没有事",
    descKey: "onboarding.step1.desc",
    descDefault:
      "今日工作台只回答一个问题：今天有没有事。没事就显示「今天没事」—— 没事就是最好的消息，别给没病的人看体检报告。",
  },
  {
    emoji: "🔴🟡🔧",
    titleKey: "onboarding.step2.title",
    titleDefault: "命 / 钱 / 事 — 三类告警",
    descKey: "onboarding.step2.desc",
    descDefault:
      "命（红）= 供应商故障转移，会丢上下文；钱（黄）= 预算超限 critical/emergency；事（蓝）= 配置缺口或漂移。命永远排最前。",
  },
  {
    emoji: "🛠️",
    titleKey: "onboarding.step3.title",
    titleDefault: "动作按钮 — 一次点击落到能修的页面",
    descKey: "onboarding.step3.desc",
    descDefault:
      "每条告警卡都带动作按钮：「去修复」「去配置」「调整预算」。一次点击直接落在「项目资产配置」对应区，不用再翻 Tab。",
  },
  {
    emoji: "📊",
    titleKey: "onboarding.step4.title",
    titleDefault: "巡视类内容 — 切到「项目看板」",
    descKey: "onboarding.step4.desc",
    descDefault:
      "聚合指标卡、阶段分布、健康清单、资产矩阵都在「项目看板」Tab。今日只放需要动手的，巡视内容不占开机第一眼。",
  },
];

export function DashboardOnboarding() {
  const { t } = useTranslation();
  const [visible, setVisible] = useState(false);
  const [step, setStep] = useState(0);

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

  const current = STEPS[step];
  const isFirst = step === 0;
  const isLast = step === STEPS.length - 1;

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
          {t("onboarding.title", { defaultValue: "欢迎使用工作区" })}
        </h3>
        <span className="text-[10px] text-muted-foreground tabular-nums">
          {step + 1}/{STEPS.length}
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
          {STEPS.map((_, i) => (
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
