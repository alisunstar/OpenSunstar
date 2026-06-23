//! AI 模块共享类型

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── AI 提供方配置（前端传入）──────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIProviderConfig {
    /// 供应商标识: "deepseek" | "glm" | "custom"
    pub provider: String,
    /// API 密钥
    pub api_key: String,
    /// API 端点 URL（OpenAI 兼容格式）
    pub api_url: String,
    /// 模型名称
    pub model: String,
}

// ── 项目上下文输入（前端从扫描数据构建）──────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContextInput {
    pub project_name: String,
    pub project_path: String,
    pub stage: String,
    /// 代码行数统计摘要（来自 tokei）
    pub code_lines: Option<CodeLinesSummary>,
    /// Git 仓库信息摘要
    pub git_info: Option<GitInfoSummary>,
    /// 近 30 天提交数
    pub commit_count_30d: u32,
    /// 近 7 天提交数（看板/周报统一窗口）
    #[serde(default)]
    pub commit_count_7d: u32,
    /// 最近 12 周每周提交数（从旧到新）
    pub weekly_commits: Vec<u32>,
    /// 贡献者列表
    pub contributors: Vec<ContributorSummary>,
    /// package.json 版本号
    pub package_version: Option<String>,
    /// MVP 阶段进度 (0-100)
    pub mvp_progress: Option<u32>,
}

/// 代码行数摘要（从 CodeLineResult 简化，支持 Deserialize）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeLinesSummary {
    pub total_lines: usize,
    pub code_lines: usize,
    pub comment_lines: usize,
    pub blank_lines: usize,
    pub files: usize,
    pub top_languages: Vec<String>,
}

/// Git 信息摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitInfoSummary {
    pub is_repo: bool,
    pub branch: Option<String>,
    pub remote_url: Option<String>,
    pub last_commit_date: Option<String>,
    pub last_commit_message: Option<String>,
}

/// 贡献者摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributorSummary {
    pub name: String,
    pub commits: u32,
}

// ── AI 返回结果 ─────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIInsightResult {
    /// AI 生成的内容文本
    pub content: String,
    /// 实际使用的模型
    pub model_used: String,
    /// 总 token 用量
    pub tokens_used: u32,
    /// 预估成本（CNY）
    pub cost_estimate: f64,
    /// 是否来自缓存
    pub is_cached: bool,
    /// 创建时间戳
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIHealthResult {
    /// 综合健康评分 (0-100)
    pub score: u32,
    /// 评分维度明细
    pub breakdown: HealthBreakdown,
    /// AI 增强分析文本（需要 AI Key）
    pub ai_analysis: Option<String>,
    /// 是否来自缓存
    pub is_cached: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthBreakdown {
    /// 活跃度得分 (0-30): 基于提交频率
    pub activity_score: u32,
    /// 贡献者得分 (0-20): 基于贡献者数量和多样性
    pub contributor_score: u32,
    /// 代码规模得分 (0-20): 基于代码行数和文件数
    pub code_scale_score: u32,
    /// 规律性得分 (0-15): 基于提交的周间分布均匀度
    pub regularity_score: u32,
    /// 版本管理得分 (0-15): 基于是否有版本号、分支策略
    pub version_score: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AICostSummary {
    /// 总成本（CNY）
    pub total_cost: f64,
    /// 总 token 数
    pub total_tokens: u64,
    /// 洞察生成次数（cost_log 行数）
    pub insight_count: u32,
    /// 按类型统计（次数，兼容旧前端）
    pub by_type: HashMap<String, u32>,
    /// 按类型统计（含金额与 token）
    pub by_type_details: Vec<CostByTypeDetail>,
    /// NL 问答次数
    pub nl_query_count: u32,
    /// 统计周期（天）
    pub period_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectProgressResult {
    /// 进度百分比 (0-100)
    pub progress: u32,
    /// AI 生成的进度摘要
    pub summary: String,
}

// ── Phase 2: 风险分析类型 ──────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIRiskResult {
    /// 风险项列表
    pub risks: Vec<RiskItem>,
    /// 总体风险等级: "high" | "medium" | "low"
    pub overall_risk: String,
    /// 一句话风险概述
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskItem {
    /// 风险类型: "activity" | "concentration" | "tech_debt" | "schedule"
    pub risk_type: String,
    /// 风险等级: "high" | "medium" | "low"
    pub level: String,
    /// 具体证据
    pub evidence: String,
    /// 改进建议
    pub suggestion: String,
}

// ── Phase 2: 自然语言查询类型 ─────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NLQueryResult {
    /// AI 生成的回答
    pub answer: String,
    /// 总 token 用量
    pub tokens_used: u32,
    /// 预估成本（CNY）
    pub cost_estimate: f64,
    /// ai_query_log 行 id（用于反馈）
    #[serde(default)]
    pub query_log_id: Option<i64>,
}

// ── Phase 3: 周报与成本分组类型 ───────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeeklyReportResult {
    /// Markdown 格式周报内容
    pub content: String,
    /// 总 token 用量
    pub tokens_used: u32,
    /// 预估成本（CNY）
    pub cost_estimate: f64,
    /// 是否来自缓存
    pub is_cached: bool,
}

/// 按 insight_type 分组的成本统计条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostByTypeEntry {
    pub insight_type: String,
    pub count: i64,
    pub total_cost: f64,
    pub total_tokens: i64,
}

/// 前端展示用的按类型成本明细
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostByTypeDetail {
    pub insight_type: String,
    pub count: u32,
    pub total_cost: f64,
    pub total_tokens: u64,
}

// ── AI ROI 报告（Phase 3 成本-价值追踪）────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIRoiReport {
    pub period_days: u32,
    pub totals: AIRoiTotals,
    pub by_type: Vec<CostByTypeDetail>,
    pub by_project: Vec<ProjectRoiEntry>,
    pub trends: Vec<RoiTrendBucket>,
    pub narrative: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIRoiTotals {
    pub cost: f64,
    pub tokens: u64,
    pub api_calls: u32,
    pub insights_generated: u32,
    pub risks_found: u32,
    pub nl_answers: u32,
    pub useful_feedback: u32,
    pub not_useful_feedback: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRoiEntry {
    pub project_id: String,
    pub project_name: String,
    pub cost: f64,
    pub tokens: u64,
    pub insight_count: u32,
    pub risk_count: u32,
    pub useful_count: u32,
    pub top_risks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoiTrendBucket {
    pub bucket_start: i64,
    pub cost: f64,
    pub tokens: u64,
    pub api_calls: u32,
    pub nl_answers: u32,
}

// ── Agent 配置就绪度（F-P2-1）────────────────────

/// Agent 配置就绪度评分结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentReadinessResult {
    pub score: u32,
    pub details: Vec<AgentReadinessItem>,
    pub llm_suggestion: Option<String>,
    pub is_cached: bool,
    /// Unix 时间戳（秒），评分计算或缓存写入时刻
    #[serde(default)]
    pub evaluated_at: Option<i64>,
}

/// Agent 就绪度单项检查
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentReadinessItem {
    pub check_name: String,
    pub label: String,
    pub weight: u32,
    pub score: u32,
    pub detail: String,
}

// ── OpenAI 兼容 API 类型 ──────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatResponse {
    pub choices: Vec<ChatChoice>,
    pub usage: Option<ChatUsage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatChoice {
    pub message: ChatMessage,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ChatUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

// ── 洞察类型与 TTL ────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InsightType {
    Summary,
    Health,
    PortfolioSummary,
    StageSuggestion,
    /// Phase 2: 风险分析
    RiskAnalysis,
    /// Phase 2: 趋势分析
    TrendAnalysis,
    /// F-P2-1: Agent 配置就绪度
    AgentReadiness,
}

impl InsightType {
    /// 缓存有效期（秒）
    pub fn ttl_seconds(&self) -> i64 {
        match self {
            InsightType::Summary => 4 * 3600,          // 4 小时
            InsightType::Health => 2 * 3600,            // 2 小时
            InsightType::PortfolioSummary => 4 * 3600,  // 4 小时
            InsightType::StageSuggestion => 8 * 3600,   // 8 小时
            InsightType::RiskAnalysis => 8 * 3600,      // 8 小时
            InsightType::TrendAnalysis => 4 * 3600,     // 4 小时
            InsightType::AgentReadiness => 24 * 3600,   // 24 小时
        }
    }

    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            InsightType::Summary => "summary",
            InsightType::Health => "health",
            InsightType::PortfolioSummary => "portfolio_summary",
            InsightType::StageSuggestion => "stage_suggestion",
            InsightType::RiskAnalysis => "risk_analysis",
            InsightType::TrendAnalysis => "trend_analysis",
            InsightType::AgentReadiness => "agent_readiness",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "summary" => Some(InsightType::Summary),
            "health" => Some(InsightType::Health),
            "portfolio_summary" => Some(InsightType::PortfolioSummary),
            "stage_suggestion" => Some(InsightType::StageSuggestion),
            "risk_analysis" => Some(InsightType::RiskAnalysis),
            "trend_analysis" => Some(InsightType::TrendAnalysis),
            "agent_readiness" => Some(InsightType::AgentReadiness),
            _ => None,
        }
    }
}
