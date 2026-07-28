/**
 * 打开项目详情抽屉的意图。
 *
 * 这里原来还有一个 `tab: "overview" | "aiAssets"` —— 抽屉曾经是两个 Tab，第二个
 * 装着蓝图 / 编排 / 就绪度 / 资产勾选。那一整块已经升格成侧栏一级页
 * 「项目资产配置」（`PageView` 的 `projectAiConfig`），抽屉只剩概览一种形态，
 * 于是这个字段没有了可选项，也就不再是「意图」。
 */
export interface ProjectDetailIntent {
  projectId: string;
  /** Increment to re-open the same project. */
  key: number;
}
