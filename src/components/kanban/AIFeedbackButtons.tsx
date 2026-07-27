/**
 * AI 洞察 / NL 问答反馈按钮 — 有用 / 无用
 */

import { useState, useCallback } from "react";
import { ThumbsUp, ThumbsDown } from "lucide-react";
import { useTranslation } from "react-i18next";
import { submitInsightFeedback, submitAIQueryFeedback } from "@/api/aiInsight";
import { useAICostOptional } from "@/contexts/AICostContext";

interface AIFeedbackButtonsProps {
  /** 洞察反馈：项目 id */
  projectId?: string;
  /** 洞察反馈：类型 */
  insightType?: string;
  /** NL 问答反馈：ai_query_log.id */
  queryLogId?: number;
  className?: string;
  onSubmitted?: () => void;
}

export function AIFeedbackButtons({
  projectId,
  insightType,
  queryLogId,
  className = "",
  onSubmitted,
}: AIFeedbackButtonsProps) {
  const { t } = useTranslation();
  const costCtx = useAICostOptional();
  const [feedback, setFeedback] = useState<"useful" | "not_useful" | null>(
    null,
  );
  const [submitting, setSubmitting] = useState(false);

  const handleClick = useCallback(
    async (value: "useful" | "not_useful") => {
      if (submitting || feedback === value) return;
      setSubmitting(true);

      let ok = false;
      if (queryLogId != null) {
        ok = await submitAIQueryFeedback(queryLogId, value);
      } else if (projectId && insightType) {
        ok = await submitInsightFeedback(projectId, insightType, value);
      }

      if (ok) {
        setFeedback(value);
        costCtx?.bumpRefresh();
        onSubmitted?.();
      }
      setSubmitting(false);
    },
    [
      projectId,
      insightType,
      queryLogId,
      feedback,
      submitting,
      costCtx,
      onSubmitted,
    ],
  );

  if (!queryLogId && (!projectId || !insightType)) {
    return null;
  }

  const btnBase =
    "inline-flex items-center gap-0.5 rounded px-1.5 py-0.5 text-xs transition-colors " +
    "hover:bg-muted/50 disabled:opacity-40";

  return (
    <span
      className={`inline-flex items-center gap-1 opacity-60 hover:opacity-100 ${className}`}
      role="group"
      aria-label={t("ai.feedback.groupLabel", { defaultValue: "AI 反馈" })}
    >
      {/* 这两个按钮只有图标，`aria-label` 是读屏器唯一能念出来的东西 ——
          它漏翻译比可见文案漏翻译更隐蔽：界面看上去完全正常。 */}
      <button
        type="button"
        className={`${btnBase} ${feedback === "useful" ? "bg-emerald-500/20 text-emerald-400" : "text-zinc-400"}`}
        onClick={() => handleClick("useful")}
        disabled={submitting}
        title={t("ai.feedback.useful", { defaultValue: "有用" })}
        aria-label={t("ai.feedback.markUseful", { defaultValue: "标记为有用" })}
        aria-pressed={feedback === "useful"}
      >
        <ThumbsUp size={13} />
      </button>
      <button
        type="button"
        className={`${btnBase} ${feedback === "not_useful" ? "bg-red-500/20 text-red-400" : "text-zinc-400"}`}
        onClick={() => handleClick("not_useful")}
        disabled={submitting}
        title={t("ai.feedback.notUseful", { defaultValue: "无用" })}
        aria-label={t("ai.feedback.markNotUseful", {
          defaultValue: "标记为无用",
        })}
        aria-pressed={feedback === "not_useful"}
      >
        <ThumbsDown size={13} />
      </button>
    </span>
  );
}
