//! AI Prompt 模板
//!
//! 根据项目上下文构建提示词，用于不同类型的 AI 洞察生成。

use super::types::{ChatMessage, ProjectContextInput};

/// 构建项目摘要的 Prompt
///
/// 目标: 生成一句简洁的项目状态描述（不超过 50 字中文）
pub fn build_summary_prompt(ctx: &ProjectContextInput) -> Vec<ChatMessage> {
    let system = "你是一个项目分析助手。根据项目指标生成一句简洁的状态摘要。要求：不超过50个中文字，包含关键信息（活跃度、规模、健康度），语气专业但易懂。不要使用markdown格式。";

    let user = format_project_context(ctx);

    vec![
        ChatMessage {
            role: "system".to_string(),
            content: system.to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: format!("{user}\n\n请用一句话描述这个项目的当前状态。"),
        },
    ]
}

/// 构建健康度分析的 Prompt
///
/// 目标: 基于规则评分 + AI 增强分析，给出改进建议
pub fn build_health_prompt(ctx: &ProjectContextInput) -> Vec<ChatMessage> {
    let system = "你是一个项目健康度分析师。根据项目指标给出简短的健康分析（2-3句话，不超过100字）。聚焦于：1）当前最大的风险点 2）最值得肯定的方面 3）一个具体可行的改进建议。不要使用markdown格式。";

    let user = format_project_context(ctx);

    vec![
        ChatMessage {
            role: "system".to_string(),
            content: system.to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: format!("{user}\n\n请分析这个项目的健康状况并给出建议。"),
        },
    ]
}

/// 构建项目组合概览的 Prompt
///
/// 目标: 从全局视角总结多项目的整体状况
pub fn build_portfolio_prompt(projects: &[ProjectContextInput]) -> Vec<ChatMessage> {
    let system = "你是一个项目总监助理。根据多个项目的指标数据，生成一段简短的组合概览（不超过150字）。关注：整体活跃度、风险项目数量、值得关注的异常。不要使用markdown格式。";

    let project_summaries: Vec<String> = projects
        .iter()
        .enumerate()
        .map(|(i, ctx)| {
            let code = ctx
                .code_lines
                .as_ref()
                .map(|c| format!("{}行代码", c.code_lines))
                .unwrap_or_else(|| "未知规模".to_string());
            let activity = if ctx.commit_count_30d >= 40 {
                "非常活跃"
            } else if ctx.commit_count_30d >= 11 {
                "活跃"
            } else if ctx.commit_count_30d >= 1 {
                "一般"
            } else {
                "沉寂"
            };
            format!(
                "项目{}: {} [{}] {} | 30天提交{}",
                i + 1,
                ctx.project_name,
                ctx.stage,
                code,
                ctx.commit_count_30d
            )
            .replace("沉寂", &format!("沉寂({activity})"))
            .replace("一般", &format!("一般({activity})"))
            .replace("活跃", &format!("活跃({activity})"))
            .replace("非常活跃", &format!("非常活跃({activity})"))
        })
        .collect();

    let user = format!(
        "以下是 {} 个项目的概况：\n{}\n\n请生成一段简短的组合概览。",
        projects.len(),
        project_summaries.join("\n")
    );

    vec![
        ChatMessage {
            role: "system".to_string(),
            content: system.to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: user,
        },
    ]
}

/// 构建项目进度估算的 Prompt
///
/// 目标: 基于代码指标和 Git 活动估算 MVP 完成度
pub fn build_progress_prompt(ctx: &ProjectContextInput) -> Vec<ChatMessage> {
    let system = "你是一个项目管理专家。根据项目指标估算 MVP（最小可行产品）的完成百分比。返回格式严格为 JSON: {\"progress\": <0-100整数>, \"summary\": \"<50字以内的进度描述>\"}。不要输出任何其他内容。";

    let user = format_project_context(ctx);

    vec![
        ChatMessage {
            role: "system".to_string(),
            content: system.to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: format!("{user}\n\n请估算这个项目的 MVP 完成度，返回 JSON 格式。"),
        },
    ]
}

// ── Phase 2 Prompt 模板 ──────────────────────────

/// 构建风险分析的 Prompt
///
/// 目标: 识别多维度风险并给出评估和建议，输出结构化 JSON
pub fn build_risk_prompt(ctx: &ProjectContextInput) -> Vec<ChatMessage> {
    let system = r#"你是一个项目风险分析专家。根据项目数据，识别潜在风险并给出评估和建议。

你必须严格返回以下 JSON 格式，不要输出任何其他内容：
{
  "risks": [
    {
      "risk_type": "activity|concentration|tech_debt|schedule",
      "level": "high|medium|low",
      "evidence": "具体数据支撑（一句话）",
      "suggestion": "可操作的改进建议（一句话）"
    }
  ],
  "overall_risk": "high|medium|low",
  "summary": "一句话风险概述（不超过60字）"
}

分析维度：
- activity: 提交频率下降、开发活跃度不足
- concentration: 单人贡献占比过高（bus factor 风险）
- tech_debt: 代码规模异常、注释比例过低、语言过于分散
- schedule: 进度与活跃度不匹配、版本长期未更新

只列出确实存在的风险（最多4条），不要凑数。如果项目状态良好，返回空数组和 "low" 等级。"#;

    let user = format_project_context(ctx);

    vec![
        ChatMessage {
            role: "system".to_string(),
            content: system.to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: format!("{user}\n\n请分析这个项目的风险状况，返回 JSON 格式。"),
        },
    ]
}

/// 构建提交趋势分析的 Prompt
///
/// 目标: 对 12 周提交趋势生成 1-2 句中文解读
pub fn build_trend_prompt(project_name: &str, weekly_commits: &[u32]) -> Vec<ChatMessage> {
    let system = "你是一个开发趋势分析师。根据项目最近 12 周的每周提交数据，用 1-2 句话（不超过 80 字）描述开发趋势。关注：整体趋势（上升/下降/平稳）、明显的峰值或低谷、可能的原因推测。不要使用 markdown 格式。";

    // 构建周数据描述（从旧到新）
    let week_labels: Vec<String> = weekly_commits
        .iter()
        .enumerate()
        .map(|(i, &count)| {
            let week_num = weekly_commits.len() - i; // W12(最旧) → W1(最新)
            format!("W{week_num}: {count}次")
        })
        .collect();

    let total: u32 = weekly_commits.iter().sum();
    let avg = if weekly_commits.is_empty() {
        0.0
    } else {
        total as f64 / weekly_commits.len() as f64
    };

    let user = format!(
        "项目: {project_name}\n最近12周每周提交数（从旧到新）:\n{}\n总计: {total}次, 周均: {avg:.1}次\n\n请描述这个项目的开发趋势。",
        week_labels.join(", ")
    );

    vec![
        ChatMessage {
            role: "system".to_string(),
            content: system.to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: user,
        },
    ]
}

/// 构建自然语言查询的 Prompt
///
/// 目标: 基于所有项目的上下文数据，回答用户的自然语言问题
pub fn build_nl_query_prompt(projects: &[ProjectContextInput], query: &str) -> Vec<ChatMessage> {
    let system = "你是一个项目管理助手，负责根据项目数据回答用户的问题。回答要求：1）基于实际数据 2）简洁明了（不超过200字）3）如果数据不足以回答，诚实说明 4）适当引用具体数字。不要使用markdown格式，直接以自然语言回答。";

    // 构建项目数据摘要（限制大小，最多 20 个项目）
    let project_summaries: Vec<String> = projects
        .iter()
        .take(20)
        .enumerate()
        .map(|(i, ctx)| {
            let code = ctx
                .code_lines
                .as_ref()
                .map(|c| format!("{}行/{}文件", c.code_lines, c.files))
                .unwrap_or_else(|| "未知".to_string());
            let contribs = if ctx.contributors.is_empty() {
                "无".to_string()
            } else {
                ctx.contributors
                    .iter()
                    .take(3)
                    .map(|c| format!("{}({})", c.name, c.commits))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let version = ctx
                .package_version
                .as_deref()
                .unwrap_or("无版本号");
            format!(
                "[{i}] {name} | 阶段:{stage} | 代码:{code} | 30天提交:{commits} | 贡献者:{contribs} | 版本:{version}",
                name = ctx.project_name,
                stage = ctx.stage,
                commits = ctx.commit_count_30d,
            )
        })
        .collect();

    let user = format!(
        "以下是 {} 个项目的数据：\n{}\n\n用户问题：{query}",
        projects.len(),
        project_summaries.join("\n")
    );

    vec![
        ChatMessage {
            role: "system".to_string(),
            content: system.to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: user,
        },
    ]
}

// ── Phase 3 Prompt 模板 ──────────────────────────

/// 构建智能周报的 Prompt
///
/// 目标: 基于所有项目数据生成结构化 Markdown 周报
pub fn build_weekly_report_prompt(projects: &[ProjectContextInput]) -> Vec<ChatMessage> {
    let system = r#"你是一个项目总监助理，负责根据以下项目数据生成一份简洁专业的中文周报。

周报格式要求（Markdown）：
## 本周项目概览

（2-3句话总结整体状况：活跃项目数、总提交数、值得关注的变化）

## 各项目进展

（每个项目 1-2 句话，按阶段分组：MVP / 快速迭代 / 稳定维护）

## 风险提醒

（需要重点关注的项目及原因，如无风险则写"本周无异常"）

## 下周展望

（基于趋势给出 1-2 条建议）

要求：
- 总字数控制在 300-500 字
- 引用具体数据（提交数、代码量等）
- 语气专业但易懂
- 不要使用代码块包裹整个报告"#;

    // 构建项目数据摘要
    let project_summaries: Vec<String> = projects
        .iter()
        .enumerate()
        .map(|(i, ctx)| {
            let code = ctx
                .code_lines
                .as_ref()
                .map(|c| format!("{}行/{}文件", c.code_lines, c.files))
                .unwrap_or_else(|| "未知".to_string());
            let contribs = if ctx.contributors.is_empty() {
                "无".to_string()
            } else {
                ctx.contributors
                    .iter()
                    .take(3)
                    .map(|c| format!("{}({})", c.name, c.commits))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            let version = ctx.package_version.as_deref().unwrap_or("无");
            // 最近 4 周趋势
            let recent_4w: Vec<u32> = ctx.weekly_commits.iter().rev().take(4).copied().collect();
            let trend = if recent_4w.len() >= 2 {
                let first_half: u32 = recent_4w[recent_4w.len()/2..].iter().sum();
                let second_half: u32 = recent_4w[..recent_4w.len()/2].iter().sum();
                if second_half > first_half * 2 { "↑上升" }
                else if first_half > second_half * 2 { "↓下降" }
                else { "→平稳" }
            } else { "—无数据" };

            let weekly_current = ctx
                .weekly_commits
                .last()
                .copied()
                .unwrap_or(ctx.commit_count_7d);
            format!(
                "[{i}] {name} | 阶段:{stage} | 代码:{code} | 本周提交:{weekly_current} | 近7天:{commits7} | 近30天:{commits30} | 趋势:{trend} | 贡献者:{contribs} | 版本:{version}",
                name = ctx.project_name,
                stage = ctx.stage,
                commits7 = ctx.commit_count_7d,
                commits30 = ctx.commit_count_30d,
            )
        })
        .collect();

    let total_commits_7d: u32 = projects.iter().map(|p| p.commit_count_7d).sum();

    let user = format!(
        "共 {} 个项目，近7天总提交 {} 次（本周维度与看板卡片一致）。\n\n{}\n\n请生成周报。",
        projects.len(),
        total_commits_7d,
        project_summaries.join("\n")
    );

    vec![
        ChatMessage {
            role: "system".to_string(),
            content: system.to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: user,
        },
    ]
}

/// 构建 Agent 配置就绪度建议的 Prompt
///
/// 目标: 基于缺失的配置项，生成一句自然语言改进建议（不超过 60 字）
pub fn build_agent_readiness_prompt(missing_items: &[String]) -> Vec<ChatMessage> {
    let system = "你是一个 AI 开发工具配置顾问。根据项目当前缺失的 AI Agent 配置项，用一句话（不超过60个中文字）给出最具操作性的改进建议。语气专业、简洁，直接说明应该配置什么以及为什么。不要使用 markdown 格式。";

    let missing_list = missing_items
        .iter()
        .enumerate()
        .map(|(i, item)| format!("  {}. {}", i + 1, item))
        .collect::<Vec<_>>()
        .join("\n");

    let user = format!(
        "该项目的 AI Agent 配置检查中，以下项目尚未完成：\n{missing_list}\n\n请给出一句改进建议。"
    );

    vec![
        ChatMessage {
            role: "system".to_string(),
            content: system.to_string(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: user,
        },
    ]
}

/// 格式化项目上下文为人类可读文本
fn format_project_context(ctx: &ProjectContextInput) -> String {
    let mut lines = Vec::new();

    lines.push(format!("项目名: {}", ctx.project_name));
    lines.push(format!("阶段: {}", stage_label(&ctx.stage)));
    lines.push(format!("路径: {}", ctx.project_path));

    if let Some(ref code) = ctx.code_lines {
        lines.push(format!(
            "代码规模: {} 行代码 / {} 个文件 (注释 {} 行, 空行 {} 行)",
            code.code_lines, code.files, code.comment_lines, code.blank_lines
        ));
        if !code.top_languages.is_empty() {
            lines.push(format!("主要语言: {}", code.top_languages.join(", ")));
        }
    }

    if let Some(ref git) = ctx.git_info {
        if git.is_repo {
            if let Some(ref branch) = git.branch {
                lines.push(format!("当前分支: {branch}"));
            }
            if let Some(ref date) = git.last_commit_date {
                lines.push(format!("最后提交时间: {date}"));
            }
            if let Some(ref msg) = git.last_commit_message {
                lines.push(format!("最后提交信息: {msg}"));
            }
        } else {
            lines.push("Git: 非 Git 仓库".to_string());
        }
    }

    lines.push(format!(
        "近7天提交数: {} | 近30天提交数: {}",
        ctx.commit_count_7d, ctx.commit_count_30d
    ));

    if !ctx.weekly_commits.is_empty() {
        let total_12w: u32 = ctx.weekly_commits.iter().sum();
        lines.push(format!("近12周总提交数: {total_12w}"));
    }

    if !ctx.contributors.is_empty() {
        let names: Vec<String> = ctx
            .contributors
            .iter()
            .take(5)
            .map(|c| format!("{}({}次)", c.name, c.commits))
            .collect();
        lines.push(format!("贡献者: {}", names.join(", ")));
    }

    if let Some(ref ver) = ctx.package_version {
        lines.push(format!("版本号: {ver}"));
    }

    if let Some(progress) = ctx.mvp_progress {
        lines.push(format!("MVP 进度: {progress}%"));
    }

    lines.join("\n")
}

fn stage_label(stage: &str) -> &str {
    match stage {
        "mvp" => "MVP（最小可行产品）",
        "rapid" => "快速迭代",
        "stable" => "稳定维护",
        other => other,
    }
}
