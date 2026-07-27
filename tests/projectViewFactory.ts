import type { Project } from "@/types/project";
import type { ProjectView } from "@/types/projectView";

/**
 * 造一个 `ProjectView`（审查报告 §6.1 的聚合对象）。
 *
 * 默认值刻意全是「什么都没扫到」：`undefined` 一路铺满，只有
 * `aiSummaryLoading` 是 `false`（没在转圈是事实，不是猜测）。测试只覆盖自己
 * 关心的那几个字段，其余保持缺失 —— 这样任何「缺失被当成 0」的回归都会在
 * 断言里现形，而不是被工厂里的默认值悄悄补上。
 */
export function makeProjectView(
  project: Project,
  patch: Partial<ProjectView> = {},
): ProjectView {
  return {
    id: project.id,
    project,
    stage: "mvp",
    progress: undefined,
    codeLines: undefined,
    version: undefined,
    gitInfo: undefined,
    commits7d: undefined,
    commits30d: undefined,
    contributors: undefined,
    weeklyCommits: undefined,
    aiSummary: undefined,
    aiSummaryLoading: false,
    aiHealthScore: undefined,
    aiTrendInsight: null,
    context: null,
    readiness: undefined,
    assets: undefined,
    ...patch,
  };
}
