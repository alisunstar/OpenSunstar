import {
  AGENT_READINESS_MAX,
  readinessMaxScore,
} from "@/lib/readinessConstants";

/** 组件里 `useTranslation()` 拿到的 `t`，只用得到这一个签名。 */
type Translate = (key: string, opts?: Record<string, unknown>) => string;

/**
 * 组合视图上一共只有两个 0-100 的分数，它们来自两个毫不相干的地方：
 *
 * | kind | 来自 | 含义 |
 * | --- | --- | --- |
 * | `agentReadiness` | 后端配置扫描（`agent_readiness.rs`） | 这个项目的 AI 配置落地了多少 |
 * | `aiHealth` | AI 分析（`usePortfolioAIAnalysis`） | 这个项目的工程健康度 |
 *
 * 两者在项目卡片上**并排出现**，此前只靠「圆点 vs 盾牌」区分，且各处自己拼
 * tooltip 文案（有的写「健康评分: 88/100」，有的写「Agent 配置就绪 42/100」，
 * 有的干脆什么都不写）—— 审查报告 §5.2「一个分数，三个名字，旁边还有一个不同
 * 的分数」。
 *
 * 名字放在这里而不是各组件里，是为了让「改名」变成一处修改。§2.5 之后还要再
 * 改一轮命名，那时这张表是唯一的落点。
 */
export type ProjectScoreKind = "agentReadiness" | "aiHealth";

export const PROJECT_SCORE_META: Record<
  ProjectScoreKind,
  { i18nKey: string; label: string; max: number }
> = {
  agentReadiness: {
    i18nKey: "kanban.readiness.title",
    label: "Agent 配置就绪",
    max: AGENT_READINESS_MAX,
  },
  aiHealth: {
    i18nKey: "kanban.score.health",
    label: "健康评分",
    max: 100,
  },
};

/** 分数的规范名。所有展示这个分数的地方都必须由此取名。 */
export function projectScoreLabel(
  kind: ProjectScoreKind,
  t: Translate,
): string {
  const meta = PROJECT_SCORE_META[kind];
  return t(meta.i18nKey, { defaultValue: meta.label });
}

/**
 * 分数的可读说明，同时充当 tooltip 与无障碍名：`Agent 配置就绪 42/100`。
 *
 * 裸数字对读屏器等于没有信息 —— 念出来就是「四十二」，既不知道满分多少，
 * 也不知道说的是哪个分数。
 *
 * @param maxScore 就绪分兼容旧缓存的 80 分制；不传按该 kind 的满分算。
 * @param hint 追加的操作提示，例如「点击查看详情」。
 */
export function projectScoreTitle(
  kind: ProjectScoreKind,
  score: number,
  t: Translate,
  options: { maxScore?: number | null; hint?: string } = {},
): string {
  const meta = PROJECT_SCORE_META[kind];
  const max =
    kind === "agentReadiness"
      ? readinessMaxScore(options.maxScore)
      : (options.maxScore ?? meta.max);
  const base = `${projectScoreLabel(kind, t)} ${score}/${max}`;
  return options.hint ? `${base} · ${options.hint}` : base;
}
