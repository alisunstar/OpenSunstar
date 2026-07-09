//! CLI 专用 API 层：为 `os` 二进制提供治理链入口
//!
//! 封装 `commands/ai_insight.rs` 中的私有编排逻辑，
//! 暴露干净的 pub 函数供 CLI 直接调用，避免修改命令层可见性。

use crate::ai::agent_readiness::{
    compute_readiness_items, detect_repo_mcp_file, ReadinessCheckInput,
    AGENT_READINESS_MAX_SCORE,
};
use crate::ai::asset_effective_state::{
    scan_effective_states, EffectiveScanContext, EffectiveScanResult, RepairAssetDriftResult,
    RepairProjectDriftResult, DRIFTED,
};
use crate::ai::types::AgentReadinessItem;
use crate::app_config::AppType;
use crate::database::Database;
use crate::services::ProviderService;
use crate::store::AppState;
use std::sync::Arc;

// ── 内部辅助（与 commands/ai_insight.rs 同构，避免跨模块暴露私有函数） ──

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
    details: Vec<AgentReadinessItem>,
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
    details: &[AgentReadinessItem],
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
    let before = before_scan
        .items
        .iter()
        .find(|i| i.check_name == check_name)
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
    let after = after_scan
        .items
        .iter()
        .find(|i| i.check_name == check_name)
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

// ── 公开 CLI 入口 ──────────────────────────────────

/// CLI: `os drift check` — 扫描项目资产生效态
pub fn cli_drift_check(
    state: &AppState,
    project_path: &str,
    target_app: Option<String>,
) -> Result<EffectiveScanResult, String> {
    let ctx = build_project_readiness_context(&state.db, project_path, target_app);
    Ok(scan_project_effective_for_details(
        state,
        project_path,
        ctx.sqlite_id.as_deref(),
        ctx.effective_target_app.as_deref(),
        &ctx.details,
    ))
}

/// CLI: 刷新治理缓存（就绪度缓存失效），供 `os drift check --refresh` 使用
pub fn cli_invalidate_readiness_cache(state: &AppState, project_path: &str) {
    let project_id = state.db.get_project_id_by_path(project_path).ok().flatten();
    if let Some(ref pid) = project_id {
        crate::ai::readiness_cache::invalidate_agent_readiness_for_project(
            &state.db,
            pid,
            Some(project_path),
        );
    }
}

/// CLI: `os drift repair` — 修复单项漂移
pub fn cli_drift_repair(
    state: &AppState,
    project_path: &str,
    check_name: &str,
    target_app: Option<String>,
) -> Result<RepairAssetDriftResult, String> {
    repair_asset_drift_inner(state, project_path, check_name, target_app)
}

/// CLI: `os drift repair --all` — 修复项目内全部漂移项
pub fn cli_drift_repair_all(
    state: &AppState,
    project_path: &str,
    target_app: Option<String>,
) -> Result<RepairProjectDriftResult, String> {
    let ctx = build_project_readiness_context(&state.db, project_path, target_app.clone());
    let initial_scan = scan_project_effective_for_details(
        state,
        project_path,
        ctx.sqlite_id.as_deref(),
        ctx.effective_target_app.as_deref(),
        &ctx.details,
    );
    let drifted: Vec<String> = initial_scan
        .items
        .iter()
        .filter(|i| i.effective_state == DRIFTED)
        .map(|i| i.check_name.clone())
        .collect();

    let mut items = Vec::with_capacity(drifted.len());
    for check_name in drifted {
        let result = repair_asset_drift_inner(
            state,
            project_path,
            &check_name,
            ctx.effective_target_app.clone(),
        )?;
        items.push(result);
    }

    let final_scan = scan_project_effective_for_details(
        state,
        project_path,
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

/// CLI: `os readiness score` — Agent 就绪度评分
pub fn cli_readiness_score(
    state: &AppState,
    project_path: &str,
    target_app: Option<String>,
) -> Result<ReadinessScoreOutput, String> {
    let ctx = build_project_readiness_context(&state.db, project_path, target_app.clone());

    // 运行生效态扫描，合并到 details 中
    let scan = scan_project_effective_for_details(
        state,
        project_path,
        ctx.sqlite_id.as_deref(),
        ctx.effective_target_app.as_deref(),
        &ctx.details,
    );

    let mut details = ctx.details;
    crate::ai::asset_effective_state::merge_effective_into_details(&mut details, &scan);

    let total_score: u32 = details.iter().map(|d| d.score).sum();

    Ok(ReadinessScoreOutput {
        project_path: project_path.to_string(),
        score: total_score,
        max_score: AGENT_READINESS_MAX_SCORE,
        target_app: ctx.effective_target_app.unwrap_or_else(|| "claude".to_string()),
        details,
        drift_items: scan
            .items
            .iter()
            .filter(|i| i.effective_state == DRIFTED)
            .map(|i| i.check_name.clone())
            .collect(),
        scanned_at: scan.scanned_at,
    })
}

/// CLI readiness 评分输出
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReadinessScoreOutput {
    pub project_path: String,
    pub score: u32,
    pub max_score: u32,
    pub target_app: String,
    pub details: Vec<AgentReadinessItem>,
    pub drift_items: Vec<String>,
    pub scanned_at: i64,
}

// ── Phase B: 编排层 CLI 入口 ─────────────────────────

/// CLI: `os flow list` — 列出工作流模块（内置 + 项目级）
pub fn cli_flow_list(
    project_path: Option<&str>,
) -> Result<Vec<crate::services::flow_orchestrator::WorkflowModule>, String> {
    crate::services::flow_orchestrator::list_workflow_modules(project_path).map_err(|e| e.to_string())
}

/// CLI: `os flow list --presets` — 列出工作流预设摘要
pub fn cli_flow_presets(
    project_path: Option<&str>,
) -> Result<Vec<crate::services::flow_orchestrator::WorkflowPresetSummary>, String> {
    crate::services::flow_orchestrator::list_workflow_presets(project_path).map_err(|e| e.to_string())
}

/// CLI: `os flow get <id>` — 获取完整 preset 定义
pub fn cli_flow_preset_get(
    id: &str,
    project_path: Option<&str>,
) -> Result<crate::services::flow_orchestrator::WorkflowPreset, String> {
    crate::services::flow_orchestrator::get_workflow_preset(id, project_path)
        .map_err(|e| e.to_string())
}

/// CLI: `os flow scan` — 扫描项目 .specs/ 目录与工作流索引
pub fn cli_flow_scan(
    project_path: &str,
    preset_id: Option<&str>,
    project_type: Option<&str>,
) -> Result<crate::services::flow_orchestrator::SpecsWorkflowIndex, String> {
    crate::services::flow_orchestrator::scan_project_specs_workflow(
        project_path,
        preset_id,
        project_type,
    )
    .map_err(|e| e.to_string())
}

/// CLI: `os flow validate` — 阶段门禁校验（R2.7 上游工件检查）
pub fn cli_flow_validate(
    project_path: &str,
    preset_id: &str,
    project_type: &str,
    change_id: &str,
    target_stage: &str,
) -> Result<crate::services::flow_orchestrator::StageGateResult, String> {
    crate::services::flow_orchestrator::validate_workflow_stage_gate(
        project_path,
        preset_id,
        project_type,
        change_id,
        target_stage,
    )
    .map_err(|e| e.to_string())
}

/// CLI: `os flow export` — 导出 workflow.profile.json
pub fn cli_flow_export(
    project_path: &str,
    preset_id: &str,
    project_type: &str,
) -> Result<crate::services::flow_orchestrator::WorkflowProfile, String> {
    crate::services::flow_orchestrator::export_project_workflow_profile(
        project_path,
        preset_id,
        project_type,
        None,
        None,
        None,
    )
    .map_err(|e| e.to_string())
}

/// CLI: `os flow config` — 导出 flow-config.yaml（含 R9.6 安全阀）
pub fn cli_flow_config(
    project_path: &str,
    preset_id: &str,
    project_type: &str,
) -> Result<crate::services::flow_orchestrator::FlowConfig, String> {
    crate::services::flow_orchestrator::export_flow_config(
        project_path,
        preset_id,
        project_type,
        None,
        None,
    )
    .map_err(|e| e.to_string())
}

/// CLI: `os flow graph` — 从 preset 构建阶段 DAG
pub fn cli_flow_graph(
    preset_id: &str,
    project_path: Option<&str>,
) -> Result<crate::services::recipe_composer::StageGraph, String> {
    let preset =
        crate::services::flow_orchestrator::get_workflow_preset(preset_id, project_path)?;
    Ok(crate::services::recipe_composer::build_stage_graph(&preset))
}

/// CLI: `os recipe list` — 列出已保存 recipe 名称
pub fn cli_recipe_list(project_path: &str) -> Result<Vec<String>, String> {
    crate::services::recipe_composer::list_saved_recipes(project_path).map_err(|e| e.to_string())
}

/// CLI: `os recipe read <name>` — 读取已保存 recipe 的完整 YAML+MD 内容
pub fn cli_recipe_read(project_path: &str, name: &str) -> Result<String, String> {
    crate::services::recipe_composer::read_saved_recipe(project_path, name)
        .map_err(|e| e.to_string())
}

/// CLI: `os recipe delete <name>` — 删除已保存 recipe
pub fn cli_recipe_delete(project_path: &str, name: &str) -> Result<(), String> {
    crate::services::recipe_composer::delete_saved_recipe(project_path, name)
        .map_err(|e| e.to_string())
}

/// CLI: `os recipe preview` — 根据参数组合 recipe 并生成 YAML+MD 混合文档预览
///
/// 内部先调用 `compose_recipe`，再调用 `generate_recipe_hybrid` 输出预览文本。
pub fn cli_recipe_preview(
    params: &crate::services::recipe_composer::RecipeComposeParams,
) -> Result<String, String> {
    let preset = crate::services::flow_orchestrator::get_workflow_preset(
        &params.preset_id,
        None,
    )?;
    let modules = crate::services::flow_orchestrator::list_workflow_modules(None)?;
    let recipe = crate::services::recipe_composer::compose_recipe(&preset, params, &modules)?;
    crate::services::recipe_composer::generate_recipe_hybrid(&recipe).map_err(|e| e.to_string())
}

/// CLI: `os recipe install` — 安装 recipe 模板脚手架到项目（.specs/ + STATE.md）
///
/// `recipe_content` 为 YAML+MD 混合文档（由 `cli_recipe_preview` 生成），
/// 通过 `parse_recipe_frontmatter` 还原为 `CompositionRecipe`，`change_id` 为空时自动生成。
pub fn cli_recipe_install(
    project_path: &str,
    recipe_content: &str,
    change_id: Option<&str>,
) -> Result<crate::services::recipe_composer::InstallResult, String> {
    let recipe = crate::services::recipe_composer::parse_recipe_frontmatter(recipe_content)?;
    let default_cid;
    let cid: &str = match change_id {
        Some(s) => s,
        None => {
            // 简易默认：recipe-name-<epoch 秒>
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            default_cid = format!("{}-{}", recipe.name.to_lowercase(), ts);
            &default_cid
        }
    };
    crate::services::recipe_composer::install_recipe(project_path, &recipe, cid)
        .map_err(|e| e.to_string())
}

/// CLI: `os recipe plan` — 安装前预检（dry-run），返回文件列表与审计摘要
pub fn cli_recipe_plan(
    project_path: &str,
    recipe_content: &str,
    change_id: &str,
) -> Result<crate::services::recipe_composer::RecipeInstallPlan, String> {
    let recipe = crate::services::recipe_composer::parse_recipe_frontmatter(recipe_content)?;
    crate::services::recipe_composer::preview_recipe_install_plan(
        project_path,
        &recipe,
        change_id,
    )
    .map_err(|e| e.to_string())
}

/// CLI: `os design list` — 列出内置品牌模板 (id, name)
pub fn cli_design_list() -> Vec<(String, String)> {
    crate::services::design_contract::list_design_templates()
}

/// CLI: `os design get <id>` — 获取内置模板完整合约
pub fn cli_design_get(
    template_id: &str,
) -> Result<crate::services::design_contract::DesignContract, String> {
    crate::services::design_contract::get_design_template(template_id)
        .ok_or_else(|| format!("模板不存在: {template_id}"))
}

/// CLI: `os design compose` — 从参数/模板组合出 DesignContract
pub fn cli_design_compose(
    params: &crate::services::design_contract::DesignContractParams,
) -> Result<crate::services::design_contract::DesignContract, String> {
    crate::services::design_contract::compose_design_contract(params).map_err(|e| e.to_string())
}

/// CLI: `os design md` — 生成 Google DESIGN.md 兼容的 Markdown
pub fn cli_design_md(
    contract: &crate::services::design_contract::DesignContract,
) -> Result<String, String> {
    crate::services::design_contract::generate_design_md(contract).map_err(|e| e.to_string())
}

/// CLI: `os design dtcg` — 生成 W3C DTCG JSON（design tokens 工具链格式）
pub fn cli_design_dtcg(
    contract: &crate::services::design_contract::DesignContract,
) -> Result<String, String> {
    crate::services::design_contract::generate_dtchg_json(contract).map_err(|e| e.to_string())
}

/// CLI: `os design export` — 导出 DESIGN.md 与 .opensunstar/contract/ 归档
pub fn cli_design_export(
    project_path: &str,
    contract: &crate::services::design_contract::DesignContract,
) -> Result<String, String> {
    crate::services::design_contract::export_design_contract(project_path, contract)
        .map_err(|e| e.to_string())
}

/// CLI: `os design install` — 安装 DESIGN.md + design-tokens.json 到项目
pub fn cli_design_install(
    project_path: &str,
    contract: &crate::services::design_contract::DesignContract,
) -> Result<crate::services::design_contract::DesignInstallResult, String> {
    crate::services::design_contract::install_design_contract(project_path, contract)
        .map_err(|e| e.to_string())
}

/// CLI: `os design import` — 从本地 DESIGN.md 文件导入
pub fn cli_design_import(
    file_path: &str,
) -> Result<crate::services::design_contract::ImportResult, String> {
    crate::services::design_contract::import_design_from_file(file_path)
        .map_err(|e| e.to_string())
}

/// CLI: `os design plan` — 安装前预检（dry-run）
pub fn cli_design_plan(
    project_path: &str,
    contract: &crate::services::design_contract::DesignContract,
) -> Result<crate::services::design_contract::DesignInstallPlan, String> {
    crate::services::design_contract::preview_install_plan(project_path, contract)
        .map_err(|e| e.to_string())
}

/// CLI: `os sdd list` — 列出 7 个 SDD 框架描述符
pub fn cli_sdd_list(
    state: &AppState,
) -> Result<Vec<crate::services::sdd::SddDescriptorSummary>, String> {
    let db_arc: std::sync::Arc<Database> = state.db.clone().into();
    crate::services::sdd::list_descriptors(&db_arc).map_err(|e| e.to_string())
}

/// CLI: `os sdd detect <path>` — 对单个项目路径执行只读探测
pub fn cli_sdd_detect(
    project_path: &str,
) -> Vec<crate::services::sdd::SddDetectionResult> {
    crate::services::sdd::detect_project(project_path)
}

/// CLI: `os sdd detect --all` — 对数据库内所有项目执行探测
pub fn cli_sdd_detect_all(
    state: &AppState,
) -> Result<
    std::collections::HashMap<String, Vec<crate::services::sdd::SddDetectionResult>>,
    String,
> {
    let db_arc: std::sync::Arc<Database> = state.db.clone().into();
    crate::services::sdd::detect_all_projects(&db_arc).map_err(|e| e.to_string())
}

/// CLI: `os sdd saved` — 读取已持久化的全部探测结果
pub fn cli_sdd_saved(
    state: &AppState,
) -> Result<
    std::collections::HashMap<String, Vec<crate::services::sdd::SddDetectionResult>>,
    String,
> {
    let db_arc: std::sync::Arc<Database> = state.db.clone().into();
    crate::services::sdd::get_all_saved_detections(&db_arc).map_err(|e| e.to_string())
}

/// CLI: `os sdd recommend` — 根据探测结果推荐 workflow preset
pub fn cli_sdd_recommend(
    results: &[crate::services::sdd::SddDetectionResult],
) -> Option<String> {
    crate::services::sdd::recommend_preset_from_detections(results)
}

/// CLI: `os blueprint list` — 列出内置项目蓝图
pub fn cli_blueprint_list() -> Result<Vec<crate::services::blueprint::Blueprint>, String> {
    crate::services::blueprint::list_blueprints().map_err(|e| e.to_string())
}

/// CLI: `os blueprint preview` — 应用蓝图的预检（不修改数据库）
pub fn cli_blueprint_preview(
    state: &AppState,
    project_id: &str,
    blueprint_id: &str,
) -> Result<crate::services::blueprint::BlueprintApplyPreview, String> {
    crate::services::blueprint::preview_apply_blueprint(state, project_id, blueprint_id)
        .map_err(|e| e.to_string())
}

/// CLI: `os blueprint apply` — 实际应用蓝图到项目（写入链接与元数据）
pub fn cli_blueprint_apply(
    state: &AppState,
    project_id: &str,
    blueprint_id: &str,
) -> Result<crate::services::blueprint::BlueprintApplyPreview, String> {
    crate::services::blueprint::apply_blueprint_to_project(state, project_id, blueprint_id)
        .map_err(|e| e.to_string())
}

// ── Phase C: 数据/配置层 CLI 入口 ─────────────────────

/// CLI: `os project list` — 列出全部项目
pub fn cli_project_list(state: &AppState) -> Result<Vec<crate::database::Project>, String> {
    state.db.get_all_projects().map_err(|e| e.to_string())
}

/// CLI: `os project get <id>` — 按 id 查询单个项目
pub fn cli_project_get(
    state: &AppState,
    project_id: &str,
) -> Result<Option<crate::database::Project>, String> {
    state.db.get_project(project_id).map_err(|e| e.to_string())
}

/// 项目统一上下文：桥接 DB (project_id) 与文件系统 (project_path)，
/// 聚合编排状态（flow profile / recipe / design contract / specs）+ 资产计数。
#[derive(serde::Serialize, Debug)]
pub struct ProjectContext {
    pub project: crate::database::Project,
    pub asset_counts: crate::database::ProjectAllAssetCounts,
    pub workspace_exists: bool,
    pub has_flow_profile: bool,
    pub has_flow_config: bool,
    pub has_design_contract: bool,
    pub recipe_count: usize,
    pub contract_count: usize,
    pub specs_exists: bool,
    pub active_change_id: Option<String>,
    pub total_artifact_completeness: Option<u8>,
}

/// CLI: `os project status` — 聚合项目全景上下文
pub fn cli_project_context(
    state: &AppState,
    project_path: &str,
) -> Result<ProjectContext, String> {
    use std::path::Path;
    let dot = Path::new(project_path).join(".opensunstar");

    // 1. DB: project metadata
    let project = state
        .db
        .get_project_by_path(project_path)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("项目未注册: {project_path}"))?;

    // 2. DB: asset counts
    let asset_counts = state
        .db
        .get_project_all_asset_counts(&project.id)
        .map_err(|e| e.to_string())?;

    // 3. Filesystem: orchestration state
    let workspace_exists = dot.is_dir();
    let has_flow_profile = dot.join("workflow.profile.json").is_file();
    let has_flow_config = dot.join("flow-config.yaml").is_file();
    let has_design_contract = Path::new(project_path).join("DESIGN.md").is_file();

    let recipe_count = std::fs::read_dir(dot.join("recipe"))
        .map(|d| d.count())
        .unwrap_or(0);
    let contract_count = std::fs::read_dir(dot.join("contract"))
        .map(|d| d.count())
        .unwrap_or(0);
    let specs_exists = Path::new(project_path).join(".specs").is_dir();

    // 4. Active change ID from STATE.md
    let active_change_id = std::fs::read_to_string(Path::new(project_path).join("STATE.md"))
        .ok()
        .and_then(|content| {
            content
                .lines()
                .find(|l| l.starts_with("change_id:"))
                .map(|l| l.trim_start_matches("change_id:").trim().to_string())
        });

    // 5. Total artifact completeness from flow scan (best-effort)
    let total_artifact_completeness = if specs_exists {
        crate::services::flow_orchestrator::scan_project_specs_workflow(
            project_path, None, None,
        )
        .ok()
        .map(|idx| {
            if idx.changes.is_empty() {
                0u8
            } else {
                let sum: u32 = idx.changes.iter().map(|c| c.artifact_completeness as u32).sum();
                (sum / idx.changes.len() as u32) as u8
            }
        })
    } else {
        None
    };

    Ok(ProjectContext {
        project,
        asset_counts,
        workspace_exists,
        has_flow_profile,
        has_flow_config,
        has_design_contract,
        recipe_count,
        contract_count,
        specs_exists,
        active_change_id,
        total_artifact_completeness,
    })
}

/// CLI: `os provider list --app <app>` — 列出某 CLI 下全部供应商
pub fn cli_provider_list(
    state: &AppState,
    app: &str,
) -> Result<indexmap::IndexMap<String, crate::provider::Provider>, String> {
    state.db.get_all_providers(app).map_err(|e| e.to_string())
}

/// CLI: `os provider current --app <app>` — 查询当前激活的供应商 id
pub fn cli_provider_current(
    state: &AppState,
    app: &str,
) -> Result<Option<String>, String> {
    state.db.get_current_provider(app).map_err(|e| e.to_string())
}

/// CLI 首次初始化结果（`os config bootstrap` / 自动 bootstrap）
#[derive(Debug, serde::Serialize)]
pub struct CliBootstrapResult {
    pub created: bool,
    pub config_dir: String,
    pub db_path: String,
    pub imported_apps: Vec<String>,
    pub seeded_official_providers: usize,
    pub seeded_skill_repos: usize,
}

/// 初始化 OpenSunstar 数据目录与 SQLite（无需 GUI）。
///
/// 对齐 GUI 启动编排：建库 → 导入 live 配置为 default provider → seed 官方预设。
pub fn cli_bootstrap_database() -> Result<CliBootstrapResult, String> {
    let config_dir = crate::config::get_app_config_dir();
    let db_path = config_dir.join("OpenSunstar.db");
    let created = !db_path.exists();

    let db = Database::init().map_err(|e| e.to_string())?;
    let state = AppState::new(Arc::new(db));

    let mut imported_apps = Vec::new();
    for app_type in AppType::all().filter(|t| !t.is_additive_mode()) {
        let should_import = ProviderService::should_import_default_config_on_startup(
            &state,
            &app_type,
        )
        .unwrap_or(false);
        if !should_import {
            continue;
        }
        if ProviderService::import_default_config(&state, app_type.clone()).unwrap_or(false) {
            imported_apps.push(app_type.as_str().to_string());
        }
    }

    let seeded_official_providers = state
        .db
        .init_default_official_providers()
        .map_err(|e| e.to_string())?;
    let seeded_skill_repos = state
        .db
        .init_default_skill_repos()
        .map_err(|e| e.to_string())?;

    Ok(CliBootstrapResult {
        created,
        config_dir: config_dir.display().to_string(),
        db_path: db_path.display().to_string(),
        imported_apps,
        seeded_official_providers,
        seeded_skill_repos,
    })
}

/// 确保数据库存在；若不存在则自动 bootstrap（CLI 独立运行入口）。
pub fn cli_ensure_database() -> Result<Database, String> {
    let db_path = crate::config::get_app_config_dir().join("OpenSunstar.db");
    if !db_path.exists() {
        cli_bootstrap_database()?;
    }
    Database::init().map_err(|e| e.to_string())
}

/// CLI: `os provider switch --app <app> <id>` — 切换激活供应商（含 live 配置写入）
pub fn cli_provider_switch(
    state: &AppState,
    app: &str,
    id: &str,
) -> Result<crate::services::provider::SwitchResult, String> {
    let app_type: AppType = app.parse().map_err(|e: crate::error::AppError| e.to_string())?;
    ProviderService::switch(state, app_type, id).map_err(|e| e.to_string())
}

/// CLI: `os provider verify` — 校验 API Key 有效性（异步 HTTP）
///
/// `verify_key` 为 async fn；此处在 CLI 二进制内构造一次性 tokio runtime 同步执行，
/// 保持对外接口同步以便与 clap 命令层直接对接。
pub fn cli_provider_verify(
    base_url: &str,
    api_key: &str,
    protocol: crate::services::provider::VerifyProtocol,
) -> Result<crate::services::provider::VerifyKeyResult, String> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
    rt.block_on(crate::services::provider::verify_key(
        base_url, api_key, protocol,
    ))
    .map_err(|e| e.to_string())
}

/// CLI: `os mcp list` — 列出全部 MCP 服务器
pub fn cli_mcp_list(
    state: &AppState,
) -> Result<indexmap::IndexMap<String, crate::app_config::McpServer>, String> {
    state.db.get_all_mcp_servers().map_err(|e| e.to_string())
}

/// CLI: `os skill list` — 列出全部已安装 Skills
pub fn cli_skill_list(
    state: &AppState,
) -> Result<indexmap::IndexMap<String, crate::app_config::InstalledSkill>, String> {
    state.db.get_all_installed_skills().map_err(|e| e.to_string())
}

/// CLI: `os asset list --project <id> [--type <asset_type>]`
pub fn cli_asset_list(
    state: &AppState,
    project_id: &str,
    asset_type: Option<&str>,
) -> Result<Vec<crate::database::ProjectAssetLink>, String> {
    state
        .db
        .get_project_asset_links(project_id, asset_type)
        .map_err(|e| e.to_string())
}

/// CLI: `os asset counts --project <id>` — 8 类资产聚合计数
pub fn cli_asset_counts(
    state: &AppState,
    project_id: &str,
) -> Result<crate::database::ProjectAllAssetCounts, String> {
    state
        .db
        .get_project_all_asset_counts(project_id)
        .map_err(|e| e.to_string())
}

/// CLI: `os config export <path>` — 导出数据库为 SQL 文件
pub fn cli_config_export(
    state: &AppState,
    output_path: &std::path::Path,
) -> Result<(), String> {
    state.db.export_sql(output_path).map_err(|e| e.to_string())
}

/// CLI: `os config import <path>` — 从 SQL 文件导入数据库，返回备份 id
pub fn cli_config_import(
    state: &AppState,
    input_path: &std::path::Path,
) -> Result<String, String> {
    state.db.import_sql(input_path).map_err(|e| e.to_string())
}
