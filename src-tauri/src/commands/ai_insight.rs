//! AI 洞察 Tauri 命令
//!
//! 项目看板 AI 能力的后端入口：洞察生成、健康评分、成本统计。

use tauri::State;

use crate::ai::agent_readiness::{
    compute_readiness_items, detect_repo_mcp_file, ReadinessCheckInput,
    AGENT_READINESS_MAX_SCORE,
};
use crate::ai::asset_effective_state::{
    merge_effective_into_details, scan_effective_states, EffectiveScanContext, EffectiveScanResult,
    RepairAssetDriftResult, RepairPreviewItem, RepairPreviewResult, RepairProjectDriftResult,
    DRIFTED,
};
use crate::ai::client::{estimate_cost, AIClient};
use crate::ai::project_id::{resolve_canonical_project_id, PORTFOLIO_PROJECT_ID};
use crate::ai::prompts;
use crate::ai::types::{
    AIHealthResult, AIInsightResult, AICostSummary, AIProviderConfig, AIRiskResult, RiskItem,
    AgentReadinessResult,
    ChatMessage, HealthBreakdown, InsightType, NLQueryResult,
    ProjectContextInput, ProjectProgressResult, WeeklyReportResult,
    CostByTypeDetail, AIRoiReport,
};
use crate::database::{AIInsightRow, AICostLogRow, AIQueryLogRow};
use crate::database::Database;
use crate::store::AppState;

// ── 内部辅助 ──────────────────────────────────

/// 计算输入数据的 SHA-256 哈希（用于缓存失效判断）
fn compute_input_hash(ctx: &ProjectContextInput) -> String {
    let json = serde_json::to_string(ctx).unwrap_or_default();
    use sha2::{Sha256, Digest};
    let hash = Sha256::digest(json.as_bytes());
    to_hex(&hash)
}

/// 字节切片转 hex 字符串
fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// 检查缓存并返回有效结果
fn check_cache(
    db: &Database,
    project_id: &str,
    insight_type: &str,
    input_hash: &str,
    force_refresh: bool,
) -> Option<AIInsightRow> {
    if force_refresh {
        return None;
    }
    let now = chrono::Utc::now().timestamp();
    match db.get_ai_insight(project_id, insight_type) {
        Ok(Some(row)) if row.expires_at > now && row.input_hash == input_hash => {
            Some(row)
        }
        _ => None,
    }
}

/// 保存 AI 洞察结果到缓存 + 成本日志
fn save_insight_and_cost(
    db: &Database,
    project_id: &str,
    project_path: Option<&str>,
    insight_type: &str,
    content: &str,
    model_used: &str,
    prompt_tokens: u32,
    completion_tokens: u32,
    cost: f64,
    input_hash: &str,
    ttl_seconds: i64,
    provider_name: &str,
) -> Result<(), String> {
    let canonical_id = resolve_canonical_project_id(db, project_id, project_path);
    let now = chrono::Utc::now().timestamp();
    let total_tokens = (prompt_tokens + completion_tokens) as i64;

    // 保存缓存
    let row = AIInsightRow {
        id: 0, // AUTOINCREMENT
        project_id: canonical_id.clone(),
        insight_type: insight_type.to_string(),
        content: content.to_string(),
        model_used: Some(model_used.to_string()),
        tokens_used: total_tokens,
        cost_estimate: cost,
        created_at: now,
        expires_at: now + ttl_seconds,
        input_hash: input_hash.to_string(),
    };
    db.upsert_ai_insight(&row).map_err(|e| e.to_string())?;

    // 记录成本
    let cost_log = AICostLogRow {
        insight_type: insight_type.to_string(),
        project_id: Some(canonical_id),
        model: Some(model_used.to_string()),
        provider: Some(provider_name.to_string()),
        prompt_tokens: prompt_tokens as i64,
        completion_tokens: completion_tokens as i64,
        cost,
        created_at: now,
    };
    db.insert_ai_cost_log(&cost_log).map_err(|e| e.to_string())?;

    Ok(())
}

/// 基于规则的 Project 健康评分（不依赖 AI）
fn compute_rule_health(ctx: &ProjectContextInput) -> HealthBreakdown {
    // 活跃度得分 (0-30): 基于 30 天提交数
    let activity_score = match ctx.commit_count_30d {
        0 => 0,
        1..=5 => 8,
        6..=15 => 16,
        16..=30 => 24,
        _ => 30,
    };

    // 贡献者得分 (0-20): 基于贡献者数量
    let contrib_count = ctx.contributors.len();
    let contributor_score = match contrib_count {
        0 => 0,
        1 => 8,
        2..=3 => 14,
        _ => 20,
    };

    // 代码规模得分 (0-20): 基于代码行数和文件数
    let code_scale_score = if let Some(ref code) = ctx.code_lines {
        let lines_score = match code.code_lines {
            0..=100 => 4,
            101..=1000 => 10,
            1001..=10000 => 16,
            _ => 20,
        };
        let files_score = match code.files {
            0..=5 => 0,
            6..=20 => 2,
            21..=100 => 4,
            _ => 6,
        };
        // 合并但不超过 20
        std::cmp::min(lines_score + files_score / 2, 20)
    } else {
        5 // 无数据给一个基础分
    };

    // 规律性得分 (0-15): 基于 12 周提交的均匀度
    let regularity_score = if ctx.weekly_commits.is_empty() {
        0
    } else {
        let total: u32 = ctx.weekly_commits.iter().sum();
        if total == 0 {
            0
        } else {
            let _avg = total as f64 / ctx.weekly_commits.len() as f64;
            let active_weeks = ctx.weekly_commits.iter().filter(|&&c| c > 0).count();
            let consistency = active_weeks as f64 / ctx.weekly_commits.len() as f64;
            // 一致性越高得分越高
            let score = (consistency * 15.0) as u32;
            std::cmp::min(score, 15)
        }
    };

    // 版本管理得分 (0-15)
    let version_score = {
        let mut score = 0u32;
        if ctx.git_info.as_ref().map(|g| g.is_repo).unwrap_or(false) {
            score += 5; // 有 Git 仓库
            if ctx.git_info.as_ref().and_then(|g| g.branch.as_ref()).is_some() {
                score += 3; // 有分支信息
            }
            if ctx.git_info.as_ref().and_then(|g| g.remote_url.as_ref()).is_some() {
                score += 3; // 有远程仓库
            }
        }
        if ctx.package_version.is_some() {
            score += 4; // 有版本号
        }
        std::cmp::min(score, 15)
    };

    HealthBreakdown {
        activity_score,
        contributor_score,
        code_scale_score,
        regularity_score,
        version_score,
    }
}

// ── Tauri 命令 ────────────────────────────────

/// 获取 AI 项目洞察（摘要/建议等），支持缓存
#[tauri::command]
pub async fn get_ai_insight(
    state: State<'_, AppState>,
    project_id: String,
    insight_type: String,
    provider_config: AIProviderConfig,
    project_context: ProjectContextInput,
    force_refresh: bool,
) -> Result<AIInsightResult, String> {
    let db = state.db.clone();
    let input_hash = compute_input_hash(&project_context);
    let itype = &insight_type;
    let canonical_id = resolve_canonical_project_id(
        &db,
        &project_id,
        Some(&project_context.project_path),
    );

    // 1. 检查缓存
    if let Some(cached) = check_cache(&db, &canonical_id, itype, &input_hash, force_refresh) {
        return Ok(AIInsightResult {
            content: cached.content,
            model_used: cached.model_used.unwrap_or_default(),
            tokens_used: cached.tokens_used as u32,
            cost_estimate: 0.0, // 缓存命中无额外成本
            is_cached: true,
            created_at: cached.created_at,
        });
    }

    // 2. 构建 Prompt
    let messages: Vec<ChatMessage> = match itype.as_str() {
        "summary" => prompts::build_summary_prompt(&project_context),
        "portfolio_summary" => prompts::build_portfolio_prompt(&[project_context.clone()]),
        "stage_suggestion" => prompts::build_summary_prompt(&project_context), // 复用摘要 prompt
        "trend_analysis" => prompts::build_trend_prompt(
            &project_context.project_name,
            &project_context.weekly_commits,
        ),
        _ => prompts::build_summary_prompt(&project_context),
    };

    // 3. 调用 AI（异步 HTTP）
    let response = AIClient::chat_completion(&provider_config, messages, Some(256)).await?;
    let content = response
        .choices
        .first()
        .map(|c| c.message.content.trim().to_string())
        .unwrap_or_else(|| "暂无分析".to_string());

    // 4. 计算成本和保存
    let (prompt_tokens, completion_tokens) = response
        .usage
        .as_ref()
        .map(|u| (u.prompt_tokens, u.completion_tokens))
        .unwrap_or((0, 0));
    let total_tokens = prompt_tokens + completion_tokens;
    let cost = estimate_cost(&provider_config.model, prompt_tokens, completion_tokens);
    let ttl = InsightType::from_str(itype)
        .map(|t| t.ttl_seconds())
        .unwrap_or(4 * 3600);
    let model_used = provider_config.model.clone();
    let now = chrono::Utc::now().timestamp();

    // 保存缓存 + 成本日志（同步 DB 操作，锁短暂）
    let _ = save_insight_and_cost(
        &db,
        &canonical_id,
        Some(&project_context.project_path),
        itype,
        &content,
        &model_used,
        prompt_tokens,
        completion_tokens,
        cost,
        &input_hash,
        ttl,
        &provider_config.provider,
    );

    Ok(AIInsightResult {
        content,
        model_used,
        tokens_used: total_tokens,
        cost_estimate: cost,
        is_cached: false,
        created_at: now,
    })
}

/// 获取项目健康评分（规则 + 可选 AI 增强分析）
#[tauri::command]
pub async fn get_ai_health_score(
    state: State<'_, AppState>,
    project_id: String,
    provider_config: AIProviderConfig,
    project_context: ProjectContextInput,
    force_refresh: bool,
) -> Result<AIHealthResult, String> {
    let db = state.db.clone();

    // 1. 规则评分（始终可用，不需要 AI）
    let breakdown = compute_rule_health(&project_context);
    let score = breakdown.activity_score
        + breakdown.contributor_score
        + breakdown.code_scale_score
        + breakdown.regularity_score
        + breakdown.version_score;

    // 2. 检查 AI 分析缓存
    let input_hash = compute_input_hash(&project_context);
    let canonical_id = resolve_canonical_project_id(
        &db,
        &project_id,
        Some(&project_context.project_path),
    );
    if let Some(cached) = check_cache(&db, &canonical_id, "health", &input_hash, force_refresh) {
        return Ok(AIHealthResult {
            score,
            breakdown,
            ai_analysis: Some(cached.content),
            is_cached: true,
        });
    }

    // 3. 调用 AI 获取增强分析
    let messages = prompts::build_health_prompt(&project_context);
    let ai_analysis = match AIClient::chat_completion(&provider_config, messages, Some(300)).await {
        Ok(response) => {
            let content = response
                .choices
                .first()
                .map(|c| c.message.content.trim().to_string())
                .unwrap_or_default();

            // 保存缓存
            let (pt, ct) = response
                .usage
                .as_ref()
                .map(|u| (u.prompt_tokens, u.completion_tokens))
                .unwrap_or((0, 0));
            let cost = estimate_cost(&provider_config.model, pt, ct);
            let _ = save_insight_and_cost(
                &db,
                &canonical_id,
                Some(&project_context.project_path),
                "health",
                &content,
                &provider_config.model,
                pt,
                ct,
                cost,
                &input_hash,
                InsightType::Health.ttl_seconds(),
                &provider_config.provider,
            );

            Some(content)
        }
        Err(e) => {
            log::warn!("AI 健康分析调用失败，仅返回规则评分: {e}");
            None
        }
    };

    Ok(AIHealthResult {
        score,
        breakdown,
        ai_analysis,
        is_cached: false,
    })
}

/// 获取 AI 调用成本汇总
#[tauri::command]
pub async fn get_ai_cost_summary(
    state: State<'_, AppState>,
    range_days: u32,
) -> Result<AICostSummary, String> {
    let db = state.db.clone();
    let since = chrono::Utc::now().timestamp() - (range_days as i64 * 86400);

    let summary = db.get_ai_cost_summary(since).map_err(|e| e.to_string())?;
    let by_type_entries = db.get_ai_cost_by_type(since).unwrap_or_default();
    let nl_query_count = db.count_nl_queries(since).unwrap_or(0);

    let mut by_type = std::collections::HashMap::new();
    let mut by_type_details = Vec::new();
    for entry in &by_type_entries {
        by_type.insert(entry.insight_type.clone(), entry.count as u32);
        by_type_details.push(CostByTypeDetail {
            insight_type: entry.insight_type.clone(),
            count: entry.count as u32,
            total_cost: entry.total_cost,
            total_tokens: entry.total_tokens as u64,
        });
    }

    Ok(AICostSummary {
        total_cost: summary.total_cost,
        total_tokens: (summary.total_prompt_tokens + summary.total_completion_tokens) as u64,
        insight_count: summary.insight_count as u32,
        by_type,
        by_type_details,
        nl_query_count,
        period_days: range_days,
    })
}

/// 获取 AI 成本-价值 ROI 报告（Phase 3）
#[tauri::command]
pub async fn get_ai_roi_report(
    state: State<'_, AppState>,
    range_days: u32,
) -> Result<AIRoiReport, String> {
    let db = state.db.clone();
    let since = chrono::Utc::now().timestamp() - (range_days as i64 * 86400);
    db.get_ai_roi_report(since, range_days)
        .map_err(|e| e.to_string())
}

// ── Phase 3: 智能周报命令 ─────────────────────────

/// 生成项目组合智能周报（Markdown 格式）
#[tauri::command]
pub async fn generate_weekly_report(
    state: State<'_, AppState>,
    provider_config: AIProviderConfig,
    projects_context: Vec<ProjectContextInput>,
) -> Result<WeeklyReportResult, String> {
    if projects_context.is_empty() {
        return Ok(WeeklyReportResult {
            content: "# 周报\n\n暂无项目数据，请先在看板中添加项目。".to_string(),
            tokens_used: 0,
            cost_estimate: 0.0,
            is_cached: false,
        });
    }

    let db = state.db.clone();
    // 使用组合级 project_id，类型为 portfolio_summary
    let project_id = PORTFOLIO_PROJECT_ID;
    let insight_type = "portfolio_summary";

    // 构建输入哈希用于缓存判断
    let combined_hash = {
        let json = serde_json::to_string(&projects_context).unwrap_or_default();
        use sha2::{Sha256, Digest};
        let hash = Sha256::digest(json.as_bytes());
        to_hex(&hash)
    };

    // 检查缓存
    if let Some(cached) = check_cache(&db, project_id, insight_type, &combined_hash, false) {
        return Ok(WeeklyReportResult {
            content: cached.content,
            tokens_used: cached.tokens_used as u32,
            cost_estimate: 0.0,
            is_cached: true,
        });
    }

    // 调用 AI
    let messages = prompts::build_weekly_report_prompt(&projects_context);
    let response = AIClient::chat_completion(&provider_config, messages, Some(2000)).await?;

    let content = response
        .choices
        .first()
        .map(|c| c.message.content.trim().to_string())
        .unwrap_or_else(|| "生成周报失败，请重试。".to_string());

    let (pt, ct) = response
        .usage
        .as_ref()
        .map(|u| (u.prompt_tokens, u.completion_tokens))
        .unwrap_or((0, 0));
    let total_tokens = pt + ct;
    let cost = estimate_cost(&provider_config.model, pt, ct);

    // 保存缓存 + 成本日志
    let _ = save_insight_and_cost(
        &db,
        PORTFOLIO_PROJECT_ID,
        None,
        insight_type,
        &content,
        &provider_config.model,
        pt,
        ct,
        cost,
        &combined_hash,
        InsightType::PortfolioSummary.ttl_seconds(),
        &provider_config.provider,
    );

    Ok(WeeklyReportResult {
        content,
        tokens_used: total_tokens,
        cost_estimate: cost,
        is_cached: false,
    })
}

/// 估算项目 MVP 进度（实现 codeMetrics.ts 中已有的存根）
#[tauri::command]
pub async fn estimate_project_progress(
    state: State<'_, AppState>,
    root: String,
    provider_config: AIProviderConfig,
) -> Result<ProjectProgressResult, String> {
    let db = state.db.clone();
    let project_id = resolve_canonical_project_id(
        &db,
        &format!("path_{}", sha2_short(&root)),
        Some(root.trim()),
    );

    // 收集项目指标
    let root_path = std::path::Path::new(root.trim());
    let code_lines = crate::project_metrics::count_code_lines(root_path).ok();
    let git_info = crate::project_metrics::detect_git_info(root_path).ok();
    let commit_30d = crate::project_metrics::git_commit_count_last_n_days(root_path, 30);
    let commit_7d = crate::project_metrics::git_commit_count_last_n_days(root_path, 7);
    let weekly = crate::project_metrics::git_weekly_commit_counts(root_path);
    let contributors: Vec<crate::ai::types::ContributorSummary> =
        crate::project_metrics::git_contributors(root_path)
            .into_iter()
            .map(|c| crate::ai::types::ContributorSummary {
                name: c.name,
                commits: c.commits,
            })
            .collect();
    let version = crate::project_metrics::read_package_version(root_path).ok().flatten();

    let ctx = ProjectContextInput {
        project_name: root_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| root.clone()),
        project_path: root,
        stage: "mvp".to_string(),
        code_lines: code_lines.map(|c| crate::ai::types::CodeLinesSummary {
            total_lines: c.total_lines,
            code_lines: c.code_lines,
            comment_lines: c.comment_lines,
            blank_lines: c.blank_lines,
            files: c.files,
            top_languages: c.languages.iter().take(5).map(|l| l.language.clone()).collect(),
        }),
        git_info: git_info.map(|g| crate::ai::types::GitInfoSummary {
            is_repo: g.is_repo,
            branch: g.branch,
            remote_url: g.remote_url,
            last_commit_date: g.last_commit_date,
            last_commit_message: g.last_commit_message,
        }),
        commit_count_30d: commit_30d,
        commit_count_7d: commit_7d,
        weekly_commits: weekly,
        contributors,
        package_version: version,
        mvp_progress: None,
    };

    // 检查缓存
    let input_hash = compute_input_hash(&ctx);
    if let Some(cached) = check_cache(&db, &project_id, "progress", &input_hash, false) {
        if let Ok(parsed) = serde_json::from_str::<ProjectProgressResult>(&cached.content) {
            return Ok(parsed);
        }
    }

    // 调用 AI
    let messages = prompts::build_progress_prompt(&ctx);
    match AIClient::chat_completion(&provider_config, messages, Some(200)).await {
        Ok(response) => {
            let content = response
                .choices
                .first()
                .map(|c| c.message.content.trim().to_string())
                .unwrap_or_default();

            // 尝试解析 JSON 响应
            let result = serde_json::from_str::<ProjectProgressResult>(&content)
                .unwrap_or(ProjectProgressResult {
                    progress: 50,
                    summary: content.clone(),
                });

            // 保存缓存
            let (pt, ct) = response
                .usage
                .as_ref()
                .map(|u| (u.prompt_tokens, u.completion_tokens))
                .unwrap_or((0, 0));
            let cost = estimate_cost(&provider_config.model, pt, ct);
            let serialized = serde_json::to_string(&result).unwrap_or_default();
            let _ = save_insight_and_cost(
                &db,
                &project_id,
                Some(&ctx.project_path),
                "progress",
                &serialized,
                &provider_config.model,
                pt,
                ct,
                cost,
                &input_hash,
                InsightType::Summary.ttl_seconds(),
                &provider_config.provider,
            );

            Ok(result)
        }
        Err(e) => {
            log::warn!("AI 进度估算失败: {e}");
            // 降级: 基于规则估算
            Ok(heuristic_progress(&ctx))
        }
    }
}

/// 基于规则的进度估算（AI 不可用时的降级方案）
fn heuristic_progress(ctx: &ProjectContextInput) -> ProjectProgressResult {
    let mut progress = 0u32;
    let mut signals = Vec::new();

    // 有代码 = 基础进度
    if let Some(ref code) = ctx.code_lines {
        if code.code_lines > 0 {
            progress += 15;
            signals.push("已有代码基础");
        }
        if code.code_lines > 1000 {
            progress += 15;
            signals.push("代码规模初具");
        }
        if code.code_lines > 10000 {
            progress += 10;
        }
    }

    // 有 Git = 版本管理
    if ctx.git_info.as_ref().map(|g| g.is_repo).unwrap_or(false) {
        progress += 10;
        signals.push("已启用版本管理");
    }

    // 有提交活跃度
    if ctx.commit_count_30d > 0 {
        progress += 10;
        signals.push("近期有活跃开发");
    }
    if ctx.commit_count_30d > 20 {
        progress += 10;
    }

    // 有贡献者
    if ctx.contributors.len() > 1 {
        progress += 5;
    }

    // 有版本号
    if ctx.package_version.is_some() {
        progress += 5;
        signals.push("已有版本号");
    }

    let progress = std::cmp::min(progress, 95);
    let summary = if signals.is_empty() {
        "项目处于初始阶段".to_string()
    } else {
        format!("基于指标估算: {}", signals.join("、"))
    };

    ProjectProgressResult { progress, summary }
}

// ── Phase 2: 风险分析命令 ─────────────────────────

/// 获取 AI 风险分析，支持缓存和规则降级
#[tauri::command]
pub async fn get_ai_risk_analysis(
    state: State<'_, AppState>,
    project_id: String,
    provider_config: AIProviderConfig,
    project_context: ProjectContextInput,
    force_refresh: bool,
) -> Result<AIRiskResult, String> {
    let db = state.db.clone();
    let input_hash = compute_input_hash(&project_context);
    let canonical_id = resolve_canonical_project_id(
        &db,
        &project_id,
        Some(&project_context.project_path),
    );

    // 1. 检查缓存
    if let Some(cached) = check_cache(&db, &canonical_id, "risk_analysis", &input_hash, force_refresh)
    {
        if let Ok(parsed) = serde_json::from_str::<AIRiskResult>(&cached.content) {
            return Ok(parsed);
        }
    }

    // 2. 调用 AI
    let messages = prompts::build_risk_prompt(&project_context);
    match AIClient::chat_completion(&provider_config, messages, Some(800)).await {
        Ok(response) => {
            let content = response
                .choices
                .first()
                .map(|c| c.message.content.trim().to_string())
                .unwrap_or_default();

            // 尝试解析 JSON
            let result = serde_json::from_str::<AIRiskResult>(&content).unwrap_or_else(|_| {
                // JSON 解析失败 → 降级为规则评估
                rule_based_risk(&project_context)
            });

            // 保存缓存
            let (pt, ct) = response
                .usage
                .as_ref()
                .map(|u| (u.prompt_tokens, u.completion_tokens))
                .unwrap_or((0, 0));
            let cost = estimate_cost(&provider_config.model, pt, ct);
            let serialized = serde_json::to_string(&result).unwrap_or_default();
            let _ = save_insight_and_cost(
                &db,
                &canonical_id,
                Some(&project_context.project_path),
                "risk_analysis",
                &serialized,
                &provider_config.model,
                pt,
                ct,
                cost,
                &input_hash,
                InsightType::RiskAnalysis.ttl_seconds(),
                &provider_config.provider,
            );

            Ok(result)
        }
        Err(e) => {
            log::warn!("AI 风险分析调用失败，降级为规则评估: {e}");
            Ok(rule_based_risk(&project_context))
        }
    }
}

/// 基于规则的风险评估（AI 不可用或 JSON 解析失败时的降级方案）
fn rule_based_risk(ctx: &ProjectContextInput) -> AIRiskResult {
    let mut risks = Vec::new();

    // 活跃度风险
    if ctx.commit_count_30d == 0 {
        risks.push(RiskItem {
            risk_type: "activity".to_string(),
            level: "high".to_string(),
            evidence: "近30天无任何提交".to_string(),
            suggestion: "确认项目是否已停止维护，或考虑归档".to_string(),
        });
    } else if ctx.commit_count_30d < 5 {
        risks.push(RiskItem {
            risk_type: "activity".to_string(),
            level: "medium".to_string(),
            evidence: format!("近30天仅{}次提交，活跃度偏低", ctx.commit_count_30d),
            suggestion: "检查是否有阻塞因素或资源不足".to_string(),
        });
    }

    // 集中度风险（bus factor）
    if ctx.contributors.len() == 1 {
        let c = &ctx.contributors[0];
        risks.push(RiskItem {
            risk_type: "concentration".to_string(),
            level: "high".to_string(),
            evidence: format!("仅1位贡献者({}), bus factor=1", c.name),
            suggestion: "考虑引入更多贡献者以降低人员风险".to_string(),
        });
    } else if !ctx.contributors.is_empty() {
        let total: u32 = ctx.contributors.iter().map(|c| c.commits).sum();
        if total > 0 {
            let top = ctx.contributors.iter().map(|c| c.commits).max().unwrap_or(0);
            let ratio = top as f64 / total as f64;
            if ratio > 0.7 {
                risks.push(RiskItem {
                    risk_type: "concentration".to_string(),
                    level: "medium".to_string(),
                    evidence: format!("主要贡献者占比 {:.0}%，集中度偏高", ratio * 100.0),
                    suggestion: "鼓励知识分享和代码审查以分散风险".to_string(),
                });
            }
        }
    }

    // 进度风险
    if let Some(progress) = ctx.mvp_progress {
        if progress < 30 && ctx.commit_count_30d < 5 {
            risks.push(RiskItem {
                risk_type: "schedule".to_string(),
                level: "medium".to_string(),
                evidence: format!("MVP 进度仅 {progress}%，且近期活跃度低"),
                suggestion: "评估是否需要调整里程碑或增加资源投入".to_string(),
            });
        }
    }

    let overall_risk = if risks.iter().any(|r| r.level == "high") {
        "high"
    } else if risks.iter().any(|r| r.level == "medium") {
        "medium"
    } else {
        "low"
    };

    let summary = if risks.is_empty() {
        "项目状态良好，暂未发现明显风险".to_string()
    } else {
        format!("发现 {} 个风险项，总体等级: {}", risks.len(), overall_label(overall_risk))
    };

    AIRiskResult {
        risks,
        overall_risk: overall_risk.to_string(),
        summary,
    }
}

fn overall_label(level: &str) -> &str {
    match level {
        "high" => "高",
        "medium" => "中",
        _ => "低",
    }
}

// ── Phase 2: 自然语言查询命令 ─────────────────────

/// 自然语言查询项目数据（不缓存，每次实时调用）
#[tauri::command]
pub async fn query_projects_nl(
    state: State<'_, AppState>,
    provider_config: AIProviderConfig,
    projects_context: Vec<ProjectContextInput>,
    query: String,
) -> Result<NLQueryResult, String> {
    if projects_context.is_empty() {
        return Ok(NLQueryResult {
            answer: "当前没有可分析的项目数据，请先在看板中添加项目。".to_string(),
            tokens_used: 0,
            cost_estimate: 0.0,
            query_log_id: None,
        });
    }

    let db = state.db.clone();
    let messages = prompts::build_nl_query_prompt(&projects_context, &query);
    let response = AIClient::chat_completion(&provider_config, messages, Some(500)).await?;

    let answer = response
        .choices
        .first()
        .map(|c| c.message.content.trim().to_string())
        .unwrap_or_else(|| "抱歉，我无法理解这个问题。".to_string());

    let (pt, ct) = response
        .usage
        .as_ref()
        .map(|u| (u.prompt_tokens, u.completion_tokens))
        .unwrap_or((0, 0));
    let cost = estimate_cost(&provider_config.model, pt, ct);
    let now = chrono::Utc::now().timestamp();
    let answer_preview = if answer.len() > 200 {
        format!("{}…", &answer[..200])
    } else {
        answer.clone()
    };

    let cost_log = AICostLogRow {
        insight_type: "nl_query".to_string(),
        project_id: Some(PORTFOLIO_PROJECT_ID.to_string()),
        model: Some(provider_config.model.clone()),
        provider: Some(provider_config.provider.clone()),
        prompt_tokens: pt as i64,
        completion_tokens: ct as i64,
        cost,
        created_at: now,
    };
    db.insert_ai_cost_log(&cost_log)
        .map_err(|e| e.to_string())?;

    let query_log = AIQueryLogRow {
        query_text: query,
        answer_preview,
        prompt_tokens: pt as i64,
        completion_tokens: ct as i64,
        cost,
        model: Some(provider_config.model.clone()),
        provider: Some(provider_config.provider.clone()),
        created_at: now,
    };
    let query_log_id = db
        .insert_ai_query_log(&query_log)
        .map_err(|e| e.to_string())?;

    Ok(NLQueryResult {
        answer,
        tokens_used: pt + ct,
        cost_estimate: cost,
        query_log_id: Some(query_log_id),
    })
}

/// 简短 SHA 用于路径标识
fn sha2_short(s: &str) -> String {
    use sha2::{Sha256, Digest};
    let hash = Sha256::digest(s.as_bytes());
    to_hex(&hash[..8])
}

// ── F-P2-1: Agent 配置就绪度 + 反馈闭环 ──────────────

/// 检测项目目录下是否存在常见 Agent 提示词文件
fn detect_prompt_files(project_path: &str) -> Vec<String> {
    let base = std::path::Path::new(project_path);
    let candidates = ["CLAUDE.md", "AGENTS.md", "GEMINI.md"];
    candidates
        .iter()
        .filter(|f| base.join(f).is_file())
        .map(|f| f.to_string())
        .collect()
}

struct ProjectReadinessContext {
    sqlite_id: Option<String>,
    effective_target_app: Option<String>,
    details: Vec<crate::ai::types::AgentReadinessItem>,
}

fn build_project_readiness_context(
    db: &Database,
    project_path: &str,
    target_app: Option<String>,
) -> ProjectReadinessContext {
    let sqlite_id = db.get_project_id_by_path(project_path).ok().flatten();
    let project_target_app = sqlite_id
        .as_deref()
        .and_then(|id| db.get_project(id).ok().flatten())
        .and_then(|p| p.target_app.clone());
    let effective_target_app = project_target_app.or(target_app);

    let mcp_count = sqlite_id
        .as_deref()
        .and_then(|id| db.count_enabled_project_mcp(id).ok())
        .unwrap_or(0);
    let skills_count = sqlite_id
        .as_deref()
        .and_then(|id| db.count_enabled_project_skills(id).ok())
        .unwrap_or(0);
    let db_prompt_count = sqlite_id
        .as_deref()
        .and_then(|id| db.count_enabled_project_prompts(id).ok())
        .unwrap_or(0);
    let prompt_files = detect_prompt_files(project_path);
    let commands_count = sqlite_id
        .as_deref()
        .and_then(|id| db.count_enabled_project_assets(id, "command").ok())
        .unwrap_or(0);
    let hooks_count = sqlite_id
        .as_deref()
        .and_then(|id| db.count_enabled_project_assets(id, "hook").ok())
        .unwrap_or(0);
    let ignore_project_count = sqlite_id
        .as_deref()
        .and_then(|id| db.count_enabled_project_assets(id, "ignore").ok())
        .unwrap_or(0);
    let permissions_project_count = sqlite_id
        .as_deref()
        .and_then(|id| db.count_enabled_project_assets(id, "permission").ok())
        .unwrap_or(0);
    let subagents_count = sqlite_id
        .as_deref()
        .and_then(|id| db.count_enabled_project_assets(id, "subagent").ok())
        .unwrap_or(0);
    let ignore_global_count = db.count_global_ignore_rules().unwrap_or(0);
    let permissions_global_count = db.count_global_permissions().unwrap_or(0);

    let max_legacy_ts = sqlite_id
        .as_deref()
        .and_then(|id| db.max_project_config_updated_at(id).ok().flatten());
    let max_links_ts = sqlite_id
        .as_deref()
        .and_then(|id| db.max_project_asset_links_updated_at(id).ok().flatten());
    let max_ts = match (max_legacy_ts, max_links_ts) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    };
    let ninety_days_ago = chrono::Utc::now().timestamp() - 7_776_000;
    let recent_update_within_90d = matches!(max_ts, Some(ts) if ts > ninety_days_ago);

    let (_, details) = compute_readiness_items(&ReadinessCheckInput {
        mcp_project_count: mcp_count,
        has_repo_mcp: detect_repo_mcp_file(project_path),
        skills_count,
        prompt_db_count: db_prompt_count,
        prompt_files,
        commands_count,
        hooks_count,
        ignore_project_count,
        ignore_global_count,
        permissions_project_count,
        permissions_global_count,
        subagents_count,
        recent_update_within_90d,
        target_app: effective_target_app.clone(),
    });

    ProjectReadinessContext {
        sqlite_id,
        effective_target_app,
        details,
    }
}

fn scan_project_effective_for_details(
    state: &AppState,
    project_path: &str,
    sqlite_id: Option<&str>,
    target_app: Option<&str>,
    details: &[crate::ai::types::AgentReadinessItem],
) -> EffectiveScanResult {
    scan_effective_states(
        state,
        details,
        target_app,
        EffectiveScanContext {
            project_path: Some(project_path),
            project_id: sqlite_id,
        },
    )
}

fn effective_item_for_check<'a>(
    scan: &'a EffectiveScanResult,
    check_name: &str,
) -> Option<&'a crate::ai::asset_effective_state::EffectiveItemState> {
    scan.items.iter().find(|i| i.check_name == check_name)
}

/// 就绪度评分的输入指纹：纳入项目关联配置与全局规则，配置变更后自动失效缓存
fn compute_agent_readiness_input_hash(
    db: &Database,
    project_path: &str,
    sqlite_id: Option<&str>,
    target_app: Option<&str>,
) -> String {
    use serde::Serialize;
    use sha2::{Digest, Sha256};

    #[derive(Serialize)]
    struct ReadinessHashInput<'a> {
        project_path: &'a str,
        mcp_count: u32,
        skills_count: u32,
        prompt_db_count: u32,
        prompt_files: Vec<String>,
        commands_count: u32,
        hooks_count: u32,
        ignore_project_count: u32,
        permissions_project_count: u32,
        subagents_count: u32,
        ignore_global_count: u32,
        perm_global_count: u32,
        max_config_ts: Option<i64>,
        max_asset_links_ts: Option<i64>,
        target_app: Option<String>,
    }

    let mcp_count = sqlite_id
        .and_then(|id| db.count_enabled_project_mcp(id).ok())
        .unwrap_or(0);
    let skills_count = sqlite_id
        .and_then(|id| db.count_enabled_project_skills(id).ok())
        .unwrap_or(0);
    let prompt_db_count = sqlite_id
        .and_then(|id| db.count_enabled_project_prompts(id).ok())
        .unwrap_or(0);
    let prompt_files = detect_prompt_files(project_path);
    let commands_count = sqlite_id
        .and_then(|id| db.count_enabled_project_assets(id, "command").ok())
        .unwrap_or(0);
    let hooks_count = sqlite_id
        .and_then(|id| db.count_enabled_project_assets(id, "hook").ok())
        .unwrap_or(0);
    let ignore_project_count = sqlite_id
        .and_then(|id| db.count_enabled_project_assets(id, "ignore").ok())
        .unwrap_or(0);
    let permissions_project_count = sqlite_id
        .and_then(|id| db.count_enabled_project_assets(id, "permission").ok())
        .unwrap_or(0);
    let subagents_count = sqlite_id
        .and_then(|id| db.count_enabled_project_assets(id, "subagent").ok())
        .unwrap_or(0);
    let ignore_global_count = db.count_global_ignore_rules().unwrap_or(0);
    let perm_global_count = db.count_global_permissions().unwrap_or(0);
    let max_config_ts = sqlite_id
        .and_then(|id| db.max_project_config_updated_at(id).ok().flatten());
    let max_asset_links_ts = sqlite_id
        .and_then(|id| db.max_project_asset_links_updated_at(id).ok().flatten());

    let payload = ReadinessHashInput {
        project_path,
        mcp_count,
        skills_count,
        prompt_db_count,
        prompt_files,
        commands_count,
        hooks_count,
        ignore_project_count,
        permissions_project_count,
        subagents_count,
        ignore_global_count,
        perm_global_count,
        max_config_ts,
        max_asset_links_ts,
        target_app: target_app.map(|s| s.to_string()),
    };
    let json = serde_json::to_string(&payload).unwrap_or_default();
    to_hex(&Sha256::digest(json.as_bytes()))
}

/// 获取 Agent 配置就绪度评分（满分 100，9 项检查：8 类资产 + 近 90 天更新）
///
/// 通过 project_path 桥接 Kanban localStorage ID 和 SQLite projects 表，
/// 查询 junction 表获取 MCP/Skills/Prompts 关联状态。
/// `scan_effective=true` 时附加生效态诊断（库内容 vs CLI 磁盘文件）。
#[tauri::command]
pub async fn get_agent_readiness_score(
    state: State<'_, AppState>,
    project_path: String,
    provider_config: Option<AIProviderConfig>,
    force_refresh: Option<bool>,
    target_app: Option<String>,
    scan_effective: Option<bool>,
) -> Result<AgentReadinessResult, String> {
    let db = state.db.clone();
    let project_id = resolve_canonical_project_id(
        &db,
        &format!("path_{}", sha2_short(&project_path)),
        Some(&project_path),
    );
    let insight_type = InsightType::AgentReadiness.as_str();
    let force = force_refresh.unwrap_or(false) || scan_effective.unwrap_or(false);
    let do_scan = scan_effective.unwrap_or(false);

    // 通过 path 桥接查找 SQLite project_id（用于 junction 表与 hash）
    let sqlite_id = db.get_project_id_by_path(&project_path).ok().flatten();
    let sqlite_id_ref = sqlite_id.as_deref();

    let project_target_app = sqlite_id_ref
        .and_then(|id| db.get_project(id).ok().flatten())
        .and_then(|p| p.target_app.clone());
    let effective_target_app = project_target_app.clone().or(target_app.clone());

    let input_hash = compute_agent_readiness_input_hash(
        &db,
        &project_path,
        sqlite_id_ref,
        effective_target_app.as_deref(),
    );

    // 1. 检查缓存
    if let Some(cached) = check_cache(&db, &project_id, insight_type, &input_hash, force) {
        if let Ok(mut parsed) = serde_json::from_str::<AgentReadinessResult>(&cached.content) {
            parsed.is_cached = true;
            if parsed.evaluated_at.is_none() {
                parsed.evaluated_at = Some(cached.created_at);
            }
            if do_scan {
                let scan = scan_effective_states(
                    &state,
                    &parsed.details,
                    effective_target_app.as_deref(),
                    EffectiveScanContext {
                        project_path: Some(&project_path),
                        project_id: sqlite_id_ref,
                    },
                );
                merge_effective_into_details(&mut parsed.details, &scan);
                parsed.is_cached = false;
            }
            return Ok(parsed);
        }
    }

    // 2. 逐项评分（100 分模型）
    let mcp_count = sqlite_id
        .as_deref()
        .and_then(|id| db.count_enabled_project_mcp(id).ok())
        .unwrap_or(0);
    let skills_count = sqlite_id
        .as_deref()
        .and_then(|id| db.count_enabled_project_skills(id).ok())
        .unwrap_or(0);
    let db_prompt_count = sqlite_id
        .as_deref()
        .and_then(|id| db.count_enabled_project_prompts(id).ok())
        .unwrap_or(0);
    let prompt_files = detect_prompt_files(&project_path);
    let commands_count = sqlite_id
        .as_deref()
        .and_then(|id| db.count_enabled_project_assets(id, "command").ok())
        .unwrap_or(0);
    let hooks_count = sqlite_id
        .as_deref()
        .and_then(|id| db.count_enabled_project_assets(id, "hook").ok())
        .unwrap_or(0);
    let ignore_project_count = sqlite_id
        .as_deref()
        .and_then(|id| db.count_enabled_project_assets(id, "ignore").ok())
        .unwrap_or(0);
    let permissions_project_count = sqlite_id
        .as_deref()
        .and_then(|id| db.count_enabled_project_assets(id, "permission").ok())
        .unwrap_or(0);
    let subagents_count = sqlite_id
        .as_deref()
        .and_then(|id| db.count_enabled_project_assets(id, "subagent").ok())
        .unwrap_or(0);
    let ignore_global_count = db.count_global_ignore_rules().unwrap_or(0);
    let permissions_global_count = db.count_global_permissions().unwrap_or(0);

    let max_legacy_ts = sqlite_id
        .as_deref()
        .and_then(|id| db.max_project_config_updated_at(id).ok().flatten());
    let max_links_ts = sqlite_id
        .as_deref()
        .and_then(|id| db.max_project_asset_links_updated_at(id).ok().flatten());
    let max_ts = match (max_legacy_ts, max_links_ts) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    };
    let ninety_days_ago = chrono::Utc::now().timestamp() - 7_776_000;
    let recent_update_within_90d = matches!(max_ts, Some(ts) if ts > ninety_days_ago);

    let (total_score, mut details) = compute_readiness_items(&ReadinessCheckInput {
        mcp_project_count: mcp_count,
        has_repo_mcp: detect_repo_mcp_file(&project_path),
        skills_count,
        prompt_db_count: db_prompt_count,
        prompt_files,
        commands_count,
        hooks_count,
        ignore_project_count,
        ignore_global_count,
        permissions_project_count,
        permissions_global_count,
        subagents_count,
        recent_update_within_90d,
        target_app: effective_target_app.clone(),
    });

    if do_scan {
        let scan = scan_effective_states(
            &state,
            &details,
            effective_target_app.as_deref(),
            EffectiveScanContext {
                project_path: Some(&project_path),
                project_id: sqlite_id_ref,
            },
        );
        merge_effective_into_details(&mut details, &scan);
    }

    // 4. 可选: LLM 补充建议（仅当有 AI 配置且存在缺失项时）
    let llm_suggestion = if let Some(ref config) = provider_config {
        let missing: Vec<String> = details
            .iter()
            .filter(|d| d.score == 0)
            .map(|d| d.label.clone())
            .collect();
        if !missing.is_empty() {
            let messages = prompts::build_agent_readiness_prompt(&missing);
            match AIClient::chat_completion(config, messages, Some(128)).await {
                Ok(resp) => {
                    let text = resp
                        .choices
                        .first()
                        .map(|c| c.message.content.trim().to_string())
                        .unwrap_or_default();

                    // 记录成本
                    let (pt, ct) = resp
                        .usage
                        .as_ref()
                        .map(|u| (u.prompt_tokens, u.completion_tokens))
                        .unwrap_or((0, 0));
                    let cost = estimate_cost(&config.model, pt, ct);
                    if cost > 0.0 {
                        let now = chrono::Utc::now().timestamp();
                        let cost_log = AICostLogRow {
                            insight_type: "agent_readiness".to_string(),
                            project_id: Some(project_id.clone()),
                            model: Some(config.model.clone()),
                            provider: Some(config.provider.clone()),
                            prompt_tokens: pt as i64,
                            completion_tokens: ct as i64,
                            cost,
                            created_at: now,
                        };
                        let _ = db.insert_ai_cost_log(&cost_log);
                    }

                    if text.is_empty() { None } else { Some(text) }
                }
                Err(e) => {
                    log::warn!("Agent 就绪度 LLM 建议生成失败: {e}");
                    None
                }
            }
        } else {
            None // 全部满分，无需建议
        }
    } else {
        None // 无 AI 配置
    };

    let now = chrono::Utc::now().timestamp();
    let result = AgentReadinessResult {
        score: total_score,
        max_score: AGENT_READINESS_MAX_SCORE,
        details,
        llm_suggestion,
        is_cached: false,
        evaluated_at: Some(now),
        target_app: effective_target_app.clone(),
    };

    // 5. 缓存结果（24h TTL；不缓存生效态字段，避免磁盘漂移导致陈旧）
    let mut cache_payload = result.clone();
    for item in &mut cache_payload.details {
        item.configured_state = None;
        item.effective_state = None;
        item.effective_detail = None;
        item.effective_scanned_at = None;
        item.live_path = None;
    }
    let serialized = serde_json::to_string(&cache_payload).unwrap_or_default();
    let cache_row = AIInsightRow {
        id: 0,
        project_id: project_id.clone(),
        insight_type: insight_type.to_string(),
        content: serialized,
        model_used: provider_config.as_ref().map(|c| c.model.clone()),
        tokens_used: 0, // LLM 建议的 token 已在成本日志中单独记录
        cost_estimate: 0.0,
        created_at: now,
        expires_at: now + InsightType::AgentReadiness.ttl_seconds(),
        input_hash,
    };
    let _ = db.upsert_ai_insight(&cache_row);

    crate::services::project_artifacts::export_readiness_artifacts(
        &db,
        &project_path,
        target_app.as_deref(),
        &result,
    );

    Ok(result)
}

/// 仅扫描项目 AI 资产生效态（库内容 vs CLI 磁盘文件），不触发 LLM 建议
#[tauri::command]
pub async fn scan_project_effective_state(
    state: State<'_, AppState>,
    project_path: String,
    target_app: Option<String>,
) -> Result<EffectiveScanResult, String> {
    let ctx = build_project_readiness_context(&state.db, &project_path, target_app);
    Ok(scan_project_effective_for_details(
        &state,
        &project_path,
        ctx.sqlite_id.as_deref(),
        ctx.effective_target_app.as_deref(),
        &ctx.details,
    ))
}

/// 漂移一键修复：按检查项写回项目级配置并复扫验证（P0-B）
fn repair_asset_drift_inner(
    state: &AppState,
    project_path: &str,
    check_name: &str,
    target_app: Option<String>,
) -> Result<RepairAssetDriftResult, String> {
    let ctx = build_project_readiness_context(&state.db, project_path, target_app);
    let before_scan = scan_project_effective_for_details(
        state,
        project_path,
        ctx.sqlite_id.as_deref(),
        ctx.effective_target_app.as_deref(),
        &ctx.details,
    );
    let before = effective_item_for_check(&before_scan, check_name)
        .ok_or_else(|| format!("未知检查项: {check_name}"))?;

    if before.effective_state != DRIFTED {
        return Ok(RepairAssetDriftResult {
            check_name: check_name.to_string(),
            before_state: before.effective_state.clone(),
            after_state: before.effective_state.clone(),
            repaired: false,
            effective_detail: before.effective_detail.clone(),
            live_path: before.live_path.clone(),
            scanned_at: before_scan.scanned_at,
        });
    }

    crate::services::project_config_sync::sync_asset_for_project_path(
        state,
        project_path,
        check_name,
    )
    .map_err(|e| e.to_string())?;

    if let Some(ref project_id) = ctx.sqlite_id {
        crate::ai::readiness_cache::invalidate_agent_readiness_for_project(
            &state.db,
            project_id,
            Some(project_path),
        );
        crate::services::project_artifacts::refresh_baseline_snapshot_for_project_id(
            &state.db,
            project_id,
            None,
        );
        if check_name == "skills_configured" {
            crate::services::project_artifacts::refresh_skill_registry_for_project_id(
                &state.db,
                project_id,
            );
        }
    }

    let after_scan = scan_project_effective_for_details(
        state,
        project_path,
        ctx.sqlite_id.as_deref(),
        ctx.effective_target_app.as_deref(),
        &ctx.details,
    );
    let after = effective_item_for_check(&after_scan, check_name)
        .ok_or_else(|| format!("未知检查项: {check_name}"))?;

    Ok(RepairAssetDriftResult {
        check_name: check_name.to_string(),
        before_state: before.effective_state.clone(),
        after_state: after.effective_state.clone(),
        repaired: after.effective_state != DRIFTED,
        effective_detail: after.effective_detail.clone(),
        live_path: after.live_path.clone(),
        scanned_at: after_scan.scanned_at,
    })
}

#[tauri::command]
pub async fn repair_asset_drift(
    state: State<'_, AppState>,
    project_path: String,
    check_name: String,
    target_app: Option<String>,
) -> Result<RepairAssetDriftResult, String> {
    repair_asset_drift_inner(&state, &project_path, &check_name, target_app)
}

/// 修复项目内漂移项（支持选择性修复：传入 check_names 仅修复指定项，不传则修复全部）
#[tauri::command]
pub async fn repair_project_drift(
    state: State<'_, AppState>,
    project_path: String,
    target_app: Option<String>,
    check_names: Option<Vec<String>>,
) -> Result<RepairProjectDriftResult, String> {
    let ctx = build_project_readiness_context(&state.db, &project_path, target_app.clone());
    let initial_scan = scan_project_effective_for_details(
        &state,
        &project_path,
        ctx.sqlite_id.as_deref(),
        ctx.effective_target_app.as_deref(),
        &ctx.details,
    );
    let all_drifted: Vec<String> = initial_scan
        .items
        .iter()
        .filter(|i| i.effective_state == DRIFTED)
        .map(|i| i.check_name.clone())
        .collect();

    // 如果传入了 check_names，仅修复这些项；否则修复全部
    let drifted = match check_names {
        Some(names) if !names.is_empty() => names
            .into_iter()
            .filter(|n| all_drifted.contains(n))
            .collect(),
        _ => all_drifted,
    };

    let mut items = Vec::with_capacity(drifted.len());
    for check_name in drifted {
        let result = repair_asset_drift_inner(
            &state,
            &project_path,
            &check_name,
            ctx.effective_target_app.clone(),
        )?;
        items.push(result);
    }

    let final_scan = scan_project_effective_for_details(
        &state,
        &project_path,
        ctx.sqlite_id.as_deref(),
        ctx.effective_target_app.as_deref(),
        &ctx.details,
    );
    let still_drifted_count = final_scan
        .items
        .iter()
        .filter(|i| i.effective_state == DRIFTED)
        .count() as u32;

    Ok(RepairProjectDriftResult {
        repaired_count: items.iter().filter(|i| i.repaired).count() as u32,
        still_drifted_count,
        items,
        scanned_at: final_scan.scanned_at,
    })
}

/// 安全类资产（影响文件暴露和命令执行）
const SAFETY_CRITICAL_CHECKS: &[&str] = &[
    "ignore_rules",
    "permissions",
    "hooks_configured",
];

fn check_label(check_name: &str) -> String {
    match check_name {
        "mcp_enabled" => "MCP 服务器",
        "prompt_files" => "Prompt 文件",
        "commands_configured" => "Commands",
        "hooks_configured" => "Hooks（钩子）",
        "permissions" => "Permissions（权限）",
        "skills_configured" => "Skills",
        "subagents_configured" => "Subagents",
        "ignore_rules" => "Ignore 规则",
        "recent_updates" => "维护度",
        _ => check_name,
    }
    .to_string()
}

/// 预览项目漂移修复：返回漂移项列表及当前文件内容摘要，供前端展示确认
#[tauri::command]
pub async fn preview_repair_project_drift(
    state: State<'_, AppState>,
    project_path: String,
    target_app: Option<String>,
) -> Result<RepairPreviewResult, String> {
    let ctx = build_project_readiness_context(&state.db, &project_path, target_app);
    let scan = scan_project_effective_for_details(
        &state,
        &project_path,
        ctx.sqlite_id.as_deref(),
        ctx.effective_target_app.as_deref(),
        &ctx.details,
    );

    let drifted_items: Vec<_> = scan
        .items
        .iter()
        .filter(|i| i.effective_state == DRIFTED)
        .collect();

    let total = drifted_items.len() as u32;

    let items: Vec<RepairPreviewItem> = drifted_items
        .into_iter()
        .map(|item| {
            let content = item
                .live_path
                .as_ref()
                .and_then(|p| {
                    let path = std::path::Path::new(p);
                    if path.is_file() {
                        std::fs::read_to_string(path).ok()
                    } else {
                        None
                    }
                })
                .unwrap_or_default();

            let truncated = if content.len() > 800 {
                let safe_end = content
                    .char_indices()
                    .take_while(|(i, _)| *i < 800)
                    .last()
                    .map(|(i, c)| i + c.len_utf8())
                    .unwrap_or(0);
                format!("{}…", &content[..safe_end])
            } else {
                content
            };

            let is_safety = SAFETY_CRITICAL_CHECKS.contains(&item.check_name.as_str());

            RepairPreviewItem {
                check_name: item.check_name.clone(),
                label: check_label(&item.check_name),
                live_path: item.live_path.clone(),
                effective_detail: item.effective_detail.clone(),
                current_content: truncated,
                is_safety_critical: is_safety,
            }
        })
        .collect();

    Ok(RepairPreviewResult {
        items,
        total_drifted: total,
    })
}

/// 提交 AI 洞察的用户反馈（有用 / 无用）
#[tauri::command]
pub async fn submit_insight_feedback(
    state: State<'_, AppState>,
    project_id: String,
    insight_type: String,
    feedback: String,
) -> Result<bool, String> {
    let db = state.db.clone();
    if feedback != "useful" && feedback != "not_useful" {
        return Err("feedback 值必须为 'useful' 或 'not_useful'".to_string());
    }
    let canonical_id = resolve_canonical_project_id(&db, &project_id, None);
    db.update_insight_feedback(&canonical_id, &insight_type, &feedback)
        .map_err(|e| e.to_string())
}

/// 提交 NL 问答的用户反馈
#[tauri::command]
pub async fn submit_ai_query_feedback(
    state: State<'_, AppState>,
    query_log_id: i64,
    feedback: String,
) -> Result<bool, String> {
    let db = state.db.clone();
    if feedback != "useful" && feedback != "not_useful" {
        return Err("feedback 值必须为 'useful' 或 'not_useful'".to_string());
    }
    db.update_query_feedback(query_log_id, &feedback)
        .map_err(|e| e.to_string())
}
