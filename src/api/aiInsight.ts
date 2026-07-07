/**
 * AI 洞察 API — 调用 Tauri Rust 后端
 *
 * 桥接前端 localStorage AI 配置与 Rust 后端 AI 能力，
 * 为项目看板提供 AI 摘要、健康评分、成本统计等能力。
 */

import { invoke } from "@tauri-apps/api/core";

// ── 类型（对齐 Rust 后端 ai::types）──────────────

export interface AIProviderConfig {
  provider: string;
  api_key: string;
  api_url: string;
  model: string;
}

export interface AIInsightResult {
  content: string;
  model_used: string;
  tokens_used: number;
  cost_estimate: number;
  is_cached: boolean;
  created_at: number;
}

export interface AIHealthResult {
  score: number;
  breakdown: HealthBreakdown;
  ai_analysis: string | null;
  is_cached: boolean;
}

export interface HealthBreakdown {
  activity_score: number;
  contributor_score: number;
  code_scale_score: number;
  regularity_score: number;
  version_score: number;
}

export interface AICostSummary {
  total_cost: number;
  total_tokens: number;
  insight_count: number;
  by_type: Record<string, number>;
  by_type_details: CostByTypeDetail[];
  nl_query_count: number;
  period_days: number;
}

export interface CostByTypeDetail {
  insight_type: string;
  count: number;
  total_cost: number;
  total_tokens: number;
}

export interface AIRoiReport {
  period_days: number;
  totals: AIRoiTotals;
  by_type: CostByTypeDetail[];
  by_project: ProjectRoiEntry[];
  trends: RoiTrendBucket[];
  narrative: string;
}

export interface AIRoiTotals {
  cost: number;
  tokens: number;
  api_calls: number;
  insights_generated: number;
  risks_found: number;
  nl_answers: number;
  useful_feedback: number;
  not_useful_feedback: number;
}

export interface ProjectRoiEntry {
  project_id: string;
  project_name: string;
  cost: number;
  tokens: number;
  insight_count: number;
  risk_count: number;
  useful_count: number;
  top_risks: string[];
}

export interface RoiTrendBucket {
  bucket_start: number;
  cost: number;
  tokens: number;
  api_calls: number;
  nl_answers: number;
}

export interface ProjectProgressResult {
  progress: number;
  summary: string;
}

// ── Phase 2 类型 ──────────────────────────────

export interface AIRiskResult {
  risks: RiskItem[];
  overall_risk: "high" | "medium" | "low";
  summary: string;
}

export interface RiskItem {
  risk_type: "activity" | "concentration" | "tech_debt" | "schedule";
  level: "high" | "medium" | "low";
  evidence: string;
  suggestion: string;
}

export interface NLQueryResult {
  answer: string;
  tokens_used: number;
  cost_estimate: number;
  query_log_id?: number | null;
}

// ── Phase 3 类型 ──────────────────────────────

export interface WeeklyReportResult {
  content: string;
  tokens_used: number;
  cost_estimate: number;
  is_cached: boolean;
}

export interface CostByTypeEntry {
  insight_type: string;
  count: number;
  total_cost: number;
  total_tokens?: number;
}

// ── F-P2-1 类型 ───────────────────────────────

export interface AgentReadinessResult {
  score: number;
  /** 满分，默认 100 */
  max_score?: number;
  details: AgentReadinessItem[];
  llm_suggestion: string | null;
  is_cached: boolean;
  /** Unix 时间戳（秒） */
  evaluated_at?: number | null;
  /** 计分所依据的目标 CLI */
  target_app?: string | null;
}

export type ReadinessItemStatus =
  | "ready"
  | "partial"
  | "global_only"
  | "detected_only"
  | "missing";

export interface AgentReadinessItem {
  check_name: string;
  label: string;
  weight: number;
  score: number;
  detail: string;
  status?: ReadinessItemStatus | null;
  /** configured | unconfigured */
  configured_state?: string | null;
  /** effective | drifted | unchecked | not_applicable */
  effective_state?: string | null;
  effective_detail?: string | null;
  effective_scanned_at?: number | null;
  live_path?: string | null;
}

export interface EffectiveItemState {
  check_name: string;
  configured_state: string;
  effective_state: string;
  effective_detail?: string | null;
  live_path?: string | null;
}

export interface EffectiveScanResult {
  scanned_at: number;
  target_app: string;
  items: EffectiveItemState[];
}

export interface RepairAssetDriftResult {
  check_name: string;
  before_state: string;
  after_state: string;
  repaired: boolean;
  effective_detail?: string | null;
  live_path?: string | null;
  scanned_at: number;
}

export interface RepairProjectDriftResult {
  repaired_count: number;
  still_drifted_count: number;
  items: RepairAssetDriftResult[];
  scanned_at: number;
}

export interface ProjectContextInput {
  project_name: string;
  project_path: string;
  stage: string;
  code_lines: CodeLinesSummary | null;
  git_info: GitInfoSummary | null;
  /** 近 7 天提交数（看板/周报统一窗口） */
  commit_count_7d: number;
  /** 近 30 天提交数（健康评分规则仍参考更长窗口） */
  commit_count_30d: number;
  weekly_commits: number[];
  contributors: ContributorSummary[];
  package_version: string | null;
  mvp_progress: number | null;
}

export interface CodeLinesSummary {
  total_lines: number;
  code_lines: number;
  comment_lines: number;
  blank_lines: number;
  files: number;
  top_languages: string[];
}

export interface GitInfoSummary {
  is_repo: boolean;
  branch: string | null;
  remote_url: string | null;
  last_commit_date: string | null;
  last_commit_message: string | null;
}

export interface ContributorSummary {
  name: string;
  commits: number;
}

// ── 配置构建 ──────────────────────────────────

/**
 * 从 Keychain + SQLite 构建 Rust 后端所需的 AIProviderConfig。
 * 返回 null 表示未配置任何 AI Key。
 */
export async function buildProviderConfig(): Promise<AIProviderConfig | null> {
  return invoke<AIProviderConfig | null>("build_ai_insight_provider_config");
}

// ── 项目上下文构建 ──────────────────────────────

/**
 * 从看板扫描数据构建 ProjectContextInput，供 Rust 后端生成 AI 洞察。
 *
 * @param project 项目基本信息
 * @param stage 阶段标识
 * @param codeMetrics tokei 扫描结果（来自 countProjectCodeLines）
 * @param gitInfo Git 仓库信息（来自 detectProjectGitInfo）
 * @param commitCount7d 近 7 天提交数（看板/周报统一窗口）
 * @param commitCount30d 近 30 天提交数
 * @param weeklyCommits 最近 12 周每周提交数
 * @param contributors 贡献者列表
 * @param packageVersion package.json 版本号
 * @param mvpProgress MVP 进度 (0-100)
 */
export function buildProjectContext(
  project: { name: string; path: string },
  stage: string,
  codeMetrics: import("./codeMetrics").CodeLineResult | null,
  gitInfo: import("./projectGit").ProjectGitInfo | null,
  commitCount7d: number,
  commitCount30d: number,
  weeklyCommits: number[],
  contributors: import("./codeMetrics").Contributor[],
  packageVersion: string | null,
  mvpProgress?: number,
): ProjectContextInput {
  return {
    project_name: project.name,
    project_path: project.path,
    stage,
    code_lines: codeMetrics
      ? {
          total_lines: codeMetrics.total_lines,
          code_lines: codeMetrics.code_lines,
          comment_lines: codeMetrics.comment_lines,
          blank_lines: codeMetrics.blank_lines,
          files: codeMetrics.files,
          top_languages: codeMetrics.languages
            .slice(0, 5)
            .map((l) => l.language),
        }
      : null,
    git_info: gitInfo
      ? {
          is_repo: gitInfo.is_repo,
          branch: gitInfo.branch ?? null,
          remote_url: gitInfo.remote_url ?? null,
          last_commit_date: gitInfo.last_commit_date ?? null,
          last_commit_message: gitInfo.last_commit_message ?? null,
        }
      : null,
    commit_count_7d: commitCount7d,
    commit_count_30d: commitCount30d,
    weekly_commits: weeklyCommits,
    contributors: contributors.map((c) => ({
      name: c.name,
      commits: c.commits,
    })),
    package_version: packageVersion,
    mvp_progress: mvpProgress ?? null,
  };
}

// ── Tauri 命令封装 ──────────────────────────────

function warn(msg: string, e: unknown): void {
  console.warn(`[aiInsight] ${msg}:`, e instanceof Error ? e.message : String(e));
}

/** 获取 AI 项目洞察 */
export async function getAIInsight(
  projectId: string,
  insightType: string,
  config: AIProviderConfig,
  context: ProjectContextInput,
  forceRefresh = false,
): Promise<AIInsightResult | null> {
  try {
    return await invoke<AIInsightResult>("get_ai_insight", {
      projectId,
      insightType,
      providerConfig: config,
      projectContext: context,
      forceRefresh,
    });
  } catch (e) {
    warn(`getAIInsight(${insightType}) failed`, e);
    return null;
  }
}

/** 获取项目健康评分 */
export async function getAIHealthScore(
  projectId: string,
  config: AIProviderConfig,
  context: ProjectContextInput,
  forceRefresh = false,
): Promise<AIHealthResult | null> {
  try {
    return await invoke<AIHealthResult>("get_ai_health_score", {
      projectId,
      providerConfig: config,
      projectContext: context,
      forceRefresh,
    });
  } catch (e) {
    warn("getAIHealthScore failed", e);
    return null;
  }
}

/** 获取 AI 调用成本汇总 */
export async function getAICostSummary(
  rangeDays: number,
): Promise<AICostSummary | null> {
  try {
    return await invoke<AICostSummary>("get_ai_cost_summary", {
      rangeDays,
    });
  } catch (e) {
    warn("getAICostSummary failed", e);
    return null;
  }
}

/** 获取 AI 成本-价值 ROI 报告 */
export async function getAIRoiReport(
  rangeDays: number,
): Promise<AIRoiReport | null> {
  try {
    return await invoke<AIRoiReport>("get_ai_roi_report", {
      rangeDays,
    });
  } catch (e) {
    warn("getAIRoiReport failed", e);
    return null;
  }
}

/** 估算项目 MVP 进度（覆盖 codeMetrics.ts 中的存根） */
export async function estimateProjectProgressAI(
  root: string,
  config: AIProviderConfig,
): Promise<ProjectProgressResult | null> {
  try {
    return await invoke<ProjectProgressResult>("estimate_project_progress", {
      root,
      providerConfig: config,
    });
  } catch (e) {
    warn("estimateProjectProgress failed", e);
    return null;
  }
}

// ── Phase 2 命令封装 ──────────────────────────────

/** 获取 AI 风险分析 */
export async function getAIRiskAnalysis(
  projectId: string,
  config: AIProviderConfig,
  context: ProjectContextInput,
  forceRefresh = false,
): Promise<AIRiskResult | null> {
  try {
    return await invoke<AIRiskResult>("get_ai_risk_analysis", {
      projectId,
      providerConfig: config,
      projectContext: context,
      forceRefresh,
    });
  } catch (e) {
    warn("getAIRiskAnalysis failed", e);
    return null;
  }
}

/** 自然语言查询项目数据 */
export async function queryProjectsNL(
  config: AIProviderConfig,
  projectsContext: ProjectContextInput[],
  query: string,
): Promise<NLQueryResult | null> {
  try {
    return await invoke<NLQueryResult>("query_projects_nl", {
      providerConfig: config,
      projectsContext,
      query,
    });
  } catch (e) {
    warn("queryProjectsNL failed", e);
    return null;
  }
}

// ── Phase 3 命令封装 ──────────────────────────────

/** 生成项目组合智能周报 */
export async function generateWeeklyReport(
  config: AIProviderConfig,
  projectsContext: ProjectContextInput[],
): Promise<WeeklyReportResult | null> {
  try {
    return await invoke<WeeklyReportResult>("generate_weekly_report", {
      providerConfig: config,
      projectsContext,
    });
  } catch (e) {
    warn("generateWeeklyReport failed", e);
    return null;
  }
}

// ── F-P2-1 命令封装 ──────────────────────────────

/** 获取 Agent 配置就绪度评分（满分 100，按目标 CLI 动态调分） */
export async function getAgentReadinessScore(
  projectPath: string,
  config?: AIProviderConfig | null,
  forceRefresh = false,
  targetApp?: string | null,
  scanEffective = false,
): Promise<AgentReadinessResult | null> {
  try {
    return await invoke<AgentReadinessResult>("get_agent_readiness_score", {
      projectPath,
      providerConfig: config ?? null,
      forceRefresh,
      targetApp: targetApp ?? null,
      scanEffective,
    });
  } catch (e) {
    warn("getAgentReadinessScore failed", e);
    return null;
  }
}

/** 仅扫描 AI 资产生效态（库 vs 磁盘），不调用 LLM */
export async function scanProjectEffectiveState(
  projectPath: string,
  targetApp?: string | null,
): Promise<EffectiveScanResult | null> {
  try {
    return await invoke<EffectiveScanResult>("scan_project_effective_state", {
      projectPath,
      targetApp: targetApp ?? null,
    });
  } catch (e) {
    warn("scanProjectEffectiveState failed", e);
    return null;
  }
}

/** 漂移一键修复：写回单类资产并复扫验证 */
export async function repairAssetDrift(
  projectPath: string,
  checkName: string,
  targetApp?: string | null,
): Promise<RepairAssetDriftResult | null> {
  try {
    return await invoke<RepairAssetDriftResult>("repair_asset_drift", {
      projectPath,
      checkName,
      targetApp: targetApp ?? null,
    });
  } catch (e) {
    warn("repairAssetDrift failed", e);
    return null;
  }
}

/** 修复项目内全部漂移项 */
export async function repairProjectDrift(
  projectPath: string,
  targetApp?: string | null,
): Promise<RepairProjectDriftResult | null> {
  try {
    return await invoke<RepairProjectDriftResult>("repair_project_drift", {
      projectPath,
      targetApp: targetApp ?? null,
    });
  } catch (e) {
    warn("repairProjectDrift failed", e);
    return null;
  }
}

/** 提交 AI 洞察的用户反馈（useful / not_useful） */
export async function submitInsightFeedback(
  projectId: string,
  insightType: string,
  feedback: "useful" | "not_useful",
): Promise<boolean> {
  try {
    return await invoke<boolean>("submit_insight_feedback", {
      projectId,
      insightType,
      feedback,
    });
  } catch (e) {
    warn("submitInsightFeedback failed", e);
    return false;
  }
}

/** 提交 NL 问答的用户反馈 */
export async function submitAIQueryFeedback(
  queryLogId: number,
  feedback: "useful" | "not_useful",
): Promise<boolean> {
  try {
    return await invoke<boolean>("submit_ai_query_feedback", {
      queryLogId,
      feedback,
    });
  } catch (e) {
    warn("submitAIQueryFeedback failed", e);
    return false;
  }
}
