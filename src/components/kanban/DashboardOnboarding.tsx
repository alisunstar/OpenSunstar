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
    titleDefault: "配置状态 — 一眼看全局",
    descKey: "onboarding.step1.desc",
    descDefault:
      "顶部汇总所有项目的 AI 配置状态：正常、需关注、异常、未扫描会分开显示。",
  },
  {
    emoji: "🔍",
    titleKey: "onboarding.step2.title",
    titleDefault: "需处理项目 — 快速定位",
    descKey: "onboarding.step2.desc",
    // 按钮文案在 §2.4 已从「查看修复」改成「修复漂移」（`health.action.repair`）。
    // 引导词用引号直接抄按钮上的字，按钮改名它就得跟着改，否则用户照着找不到。
    descDefault:
      "需要关注的项目会列在下方，附带人可读的原因说明。点击「修复漂移」或「配置资产」即可直达操作。",
  },
  /**
   * 这块面板不在今日工作台上，而引导条只在今日工作台渲染 —— 所以这段话必须
   * 说清去哪儿找，否则用户在当前页面上找一块不存在的东西。
   *
   * 去处几经变动：§3.3 先把它从今日工作台挪到项目看板，最终定在「AI 资产
   * 总览」（配置在各项目里落地了多少，正是这块面板回答的问题）。权威事实是
   * `workspaceTabLayout.test.tsx` —— 那里断言它只在 assetsMatrix 这个 Tab 出现。
   * 名字也从「治理总览」改成了「配置生效率」（§2.5）。
   */
  {
    emoji: "📊",
    titleKey: "onboarding.step3.title",
    titleDefault: "配置生效率 — 在「AI 资产总览」",
    descKey: "onboarding.step3.desc",
    descDefault:
      "配置不一致项目、生效率、已生效项等详细数据，切到「AI 资产总览」查看。将鼠标悬停在卡片上可查看说明。",
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
