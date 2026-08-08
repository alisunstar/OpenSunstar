//! 部署执行器（Git MVP M3+M4）
//!
//! 将 DeploymentPlan 中的写入步骤（Create/Update/Remove）执行到项目目录，
//! 并产出文件级回执（post-write SHA-256 验证）。
//!
//! 写入策略（冻结文档 D1：复用现有适配器路径）：
//! - Prompt/Rule → marker_merge 受管区域注入（section_id = "team:{asset_id}"）
//! - Skill → 目录复制（team 包 → .claude/skills/ 或 .codex/skills/）
//! - Ignore → 文件写入（整体替换，team 为唯一 writer）
//! - Permission → JSON/TOML 合并（MVP 简化：整体写入受管区域）
//!
//! 安全约束：
//! - 仅执行 is_deployable() 资产
//! - DisplayOnly / Skip 步骤不触碰文件系统
//! - 写入前备份（M6 回滚依赖）

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::deployment::{DeploymentAction, DeploymentPlan, DeploymentStep};
use super::domain::AssetType;

// ─── 路径安全 ──────────────────────────────────────────────────────────────────

/// 校验 asset_id 不含路径遍历字符
///
/// 拒绝包含 `..`、`/`、`\`、空字节的值，防止恶意 team.toml 逃逸目标目录。
fn validate_asset_id(asset_id: &str) -> Result<(), String> {
    if asset_id.is_empty() {
        return Err("asset_id 不能为空".to_string());
    }
    if asset_id.contains("..") {
        return Err(format!("asset_id 含非法 '..' 序列: {asset_id}"));
    }
    if asset_id.contains('/') || asset_id.contains('\\') {
        return Err(format!("asset_id 含非法路径分隔符: {asset_id}"));
    }
    if asset_id.contains('\0') {
        return Err(format!("asset_id 含空字节: {asset_id}"));
    }
    // 拒绝绝对路径前缀（Windows: C:\, Unix: /）
    if asset_id.starts_with('/') || (asset_id.len() >= 2 && asset_id.as_bytes()[1] == b':') {
        return Err(format!("asset_id 不能为绝对路径: {asset_id}"));
    }
    Ok(())
}

/// 备份根目录：始终固定在 project_root 下
///
/// 执行器写入备份与回滚校验必须共用同一个根，否则回滚侧无法判断某个
/// backup_path 是否确实由本项目产生。
pub(crate) fn backup_root(project_root: &Path) -> PathBuf {
    project_root.join(".opensunstar").join("backups")
}

/// 断言解析后的路径仍在 project_root 内（纵深防御）
///
/// 即使 asset_id 校验通过，仍对最终路径做包含性检查。
pub(crate) fn assert_path_contained(path: &Path, root: &Path) -> Result<(), String> {
    // 使用 lexical 检查避免 canonicalize 对不存在路径的失败
    let normalized = normalize_lexical(path);
    let root_normalized = normalize_lexical(root);
    if !normalized.starts_with(&root_normalized) {
        return Err(format!(
            "路径遍历检测: {} 逃逸出 {}",
            path.display(),
            root.display()
        ));
    }
    Ok(())
}

/// 词法路径规范化（不访问文件系统）
fn normalize_lexical(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for comp in path.components() {
        match comp {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {}
            other => components.push(other),
        }
    }
    components.iter().collect()
}

// ─── 回执类型 ──────────────────────────────────────────────────────────────────

/// 单步执行回执
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepReceipt {
    pub asset_type: AssetType,
    pub asset_id: String,
    pub action: DeploymentAction,
    pub target_path: String,
    /// 执行是否成功
    pub success: bool,
    /// 写入后文件 SHA-256（成功时有值）
    pub post_write_sha256: Option<String>,
    /// 备份路径（写入前创建，用于回滚）
    pub backup_path: Option<String>,
    /// 错误信息（失败时有值）
    pub error: Option<String>,
}

/// 完整部署回执
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentReceipt {
    pub project_id: String,
    pub target_app: String,
    pub plan_sha256: String,
    /// 每步回执
    pub steps: Vec<StepReceipt>,
    /// 汇总
    pub summary: ReceiptSummary,
    /// 执行时间戳（Unix seconds）
    pub executed_at: i64,
}

/// 回执汇总
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceiptSummary {
    pub total_steps: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub skipped_count: usize,
    /// 是否全部成功
    pub all_success: bool,
}

// ─── 执行选项 ──────────────────────────────────────────────────────────────────

/// 执行选项
#[derive(Debug, Clone)]
pub struct ExecuteOptions {
    /// 团队配置包根目录（用于读取 content_ref 指向的资产文件）
    pub team_package_root: PathBuf,
    /// 是否为 dry-run（仅预览，不实际写入）
    pub dry_run: bool,
    /// 是否创建备份（默认 true，M6 回滚依赖）
    pub create_backup: bool,
}

impl Default for ExecuteOptions {
    fn default() -> Self {
        Self {
            team_package_root: PathBuf::from("."),
            dry_run: false,
            create_backup: true,
        }
    }
}

// ─── 执行引擎 ──────────────────────────────────────────────────────────────────

/// 执行部署计划
///
/// 遍历 plan 中所有写入步骤（Create/Update/Remove），依次执行并收集回执。
/// 非写入步骤（Skip/DisplayOnly）记录为 skipped。
///
/// # 安全保证
/// - 写入前备份原文件（create_backup=true 时）
/// - 写入后立即回读验证 SHA-256
/// - 单步失败不中断后续步骤（best-effort）
pub fn execute_deployment_plan(
    plan: &DeploymentPlan,
    project_root: &Path,
    options: &ExecuteOptions,
) -> DeploymentReceipt {
    let mut receipts: Vec<StepReceipt> = Vec::new();

    for step in &plan.steps {
        let receipt = execute_step(step, project_root, &plan.target_app, options);
        receipts.push(receipt);
    }

    let summary = compute_receipt_summary(&receipts);

    DeploymentReceipt {
        project_id: plan.project_id.clone(),
        target_app: plan.target_app.as_str().to_string(),
        plan_sha256: plan.plan_sha256.clone(),
        steps: receipts,
        summary,
        executed_at: chrono::Utc::now().timestamp(),
    }
}

/// 执行单个部署步骤
fn execute_step(
    step: &DeploymentStep,
    project_root: &Path,
    target_app: &super::domain::TargetApp,
    options: &ExecuteOptions,
) -> StepReceipt {
    let base = StepReceipt {
        asset_type: step.asset_type.clone(),
        asset_id: step.asset_id.clone(),
        action: step.action.clone(),
        target_path: step.target_path.clone(),
        success: false,
        post_write_sha256: None,
        backup_path: None,
        error: None,
    };

    // 非写入步骤直接跳过
    if !step.action.is_write() {
        return StepReceipt {
            success: true,
            error: None,
            ..base
        };
    }

    // Dry-run 模式
    if options.dry_run {
        return StepReceipt {
            success: true,
            error: Some("[dry-run] not executed".to_string()),
            ..base
        };
    }

    // 路径安全校验（C1: 防路径遍历）
    if let Err(e) = validate_asset_id(&step.asset_id) {
        return StepReceipt {
            error: Some(e),
            ..base
        };
    }

    let target_abs = project_root.join(&step.target_path);

    // 纵深防御：断言目标路径仍在项目根内（C4: TOCTOU 缓解）
    if let Err(e) = assert_path_contained(&target_abs, project_root) {
        return StepReceipt {
            error: Some(e),
            ..base
        };
    }

    match step.action {
        DeploymentAction::Create | DeploymentAction::Update => {
            execute_write(step, &target_abs, project_root, target_app, options, base)
        }
        DeploymentAction::Remove => execute_remove(step, &target_abs, project_root, options, base),
        _ => base,
    }
}

/// 执行写入（Create / Update）
fn execute_write(
    step: &DeploymentStep,
    target_abs: &Path,
    project_root: &Path,
    target_app: &super::domain::TargetApp,
    options: &ExecuteOptions,
    base: StepReceipt,
) -> StepReceipt {
    // 读取资产内容
    let content = match read_asset_content(step, &options.team_package_root) {
        Ok(c) => c,
        Err(e) => {
            return StepReceipt {
                error: Some(format!("读取资产内容失败: {e}")),
                ..base
            };
        }
    };

    // 备份
    let backup_path = if options.create_backup && target_abs.exists() {
        match create_backup(target_abs, project_root) {
            Ok(bp) => Some(bp),
            Err(e) => {
                return StepReceipt {
                    error: Some(format!("备份失败: {e}")),
                    ..base
                };
            }
        }
    } else {
        None
    };

    // 执行写入
    let (verify_path, written_bytes) = match write_asset(step, target_abs, &content) {
        Ok(v) => v,
        Err(e) => {
            return StepReceipt {
                error: Some(format!("写入失败: {e}")),
                backup_path,
                ..base
            };
        }
    };

    // 回读校验：落盘内容必须与实际写入内容逐字节一致。
    // 不能拿 desired_sha256 比对——它是团队资产内容的哈希，而 marker 注入 /
    // JSON 合并后的文件内容本就与之不同。
    if let Err(e) = verify_written_bytes(&verify_path, &written_bytes) {
        return StepReceipt {
            error: Some(e),
            backup_path,
            ..base
        };
    }

    // 回执验证：回读并计算 SHA-256
    let post_sha = verify_write(target_abs, step, target_app);

    StepReceipt {
        success: true,
        post_write_sha256: post_sha,
        backup_path,
        ..base
    }
}

/// 执行移除（Remove）
fn execute_remove(
    step: &DeploymentStep,
    target_abs: &Path,
    project_root: &Path,
    options: &ExecuteOptions,
    base: StepReceipt,
) -> StepReceipt {
    if !target_abs.exists() {
        // 目标不存在，视为成功
        return StepReceipt {
            success: true,
            ..base
        };
    }

    // 备份
    let backup_path = if options.create_backup {
        match create_backup(target_abs, project_root) {
            Ok(bp) => Some(bp),
            Err(e) => {
                return StepReceipt {
                    error: Some(format!("备份失败: {e}")),
                    ..base
                };
            }
        }
    } else {
        None
    };

    // 执行移除
    if let Err(e) = remove_asset(step, target_abs) {
        return StepReceipt {
            error: Some(format!("移除失败: {e}")),
            backup_path,
            ..base
        };
    }

    StepReceipt {
        success: true,
        backup_path,
        ..base
    }
}

// ─── 资产读写 ──────────────────────────────────────────────────────────────────

/// 从团队包读取资产内容
fn read_asset_content(step: &DeploymentStep, team_root: &Path) -> Result<String, String> {
    // 路径安全校验（C2: 防读取遍历）
    validate_asset_id(&step.asset_id)?;

    // content_ref 在 deployment step 中没有直接暴露，
    // 但 target_path 中的 asset_id 对应团队包中的 assets/ 目录
    // MVP 简化：从 team_root/assets/{asset_id} 读取
    let assets_dir = team_root.join("assets");
    let asset_path = assets_dir.join(&step.asset_id);

    // 纵深防御：断言资产路径仍在 team_root/assets/ 内
    assert_path_contained(&asset_path, &assets_dir)?;

    if asset_path.is_dir() {
        // 目录型资产（Skill）：读取 SKILL.md 或主文件
        let skill_md = asset_path.join("SKILL.md");
        if skill_md.is_file() {
            std::fs::read_to_string(&skill_md).map_err(|e| e.to_string())
        } else {
            // 尝试读取目录下第一个 .md 文件
            let entries: Vec<_> = std::fs::read_dir(&asset_path)
                .map_err(|e| e.to_string())?
                .flatten()
                .filter(|e| e.path().extension().map(|ext| ext == "md").unwrap_or(false))
                .collect();
            if let Some(entry) = entries.first() {
                std::fs::read_to_string(entry.path()).map_err(|e| e.to_string())
            } else {
                Err(format!("资产目录 '{}' 中无 .md 文件", step.asset_id))
            }
        }
    } else if asset_path.is_file() {
        std::fs::read_to_string(&asset_path).map_err(|e| e.to_string())
    } else {
        // 尝试带扩展名的路径
        for ext in ["md", "txt", "json", "toml"] {
            let with_ext = team_root
                .join("assets")
                .join(format!("{}.{}", step.asset_id, ext));
            if with_ext.is_file() {
                return std::fs::read_to_string(&with_ext).map_err(|e| e.to_string());
            }
        }
        Err(format!("资产文件不存在: assets/{}", step.asset_id))
    }
}

/// 写入资产到目标路径
///
/// 返回 `(校验路径, 实际写入字节)`，供写入后回读比对。
fn write_asset(
    step: &DeploymentStep,
    target_abs: &Path,
    content: &str,
) -> Result<(PathBuf, Vec<u8>), String> {
    match &step.asset_type {
        // Prompt/Rule → marker_merge 受管区域注入
        AssetType::Prompt | AssetType::Rule => {
            let section_id = format!("team:{}", step.asset_id);
            let existing = if target_abs.is_file() {
                std::fs::read_to_string(target_abs).map_err(|e| e.to_string())?
            } else {
                String::new()
            };
            let merged = crate::services::marker_merge::inject_markdown_section(
                &existing,
                &section_id,
                content,
            );
            ensure_parent_dir(target_abs)?;
            std::fs::write(target_abs, &merged).map_err(|e| e.to_string())?;
            Ok((target_abs.to_path_buf(), merged.into_bytes()))
        }
        // Skill → 目录复制
        AssetType::Skill => {
            ensure_parent_dir(target_abs)?;
            if target_abs.is_dir() {
                // 更新：清空后重写
                std::fs::remove_dir_all(target_abs).map_err(|e| e.to_string())?;
            }
            std::fs::create_dir_all(target_abs).map_err(|e| e.to_string())?;
            // 写入 SKILL.md
            let skill_md = target_abs.join("SKILL.md");
            std::fs::write(&skill_md, content).map_err(|e| e.to_string())?;
            Ok((skill_md, content.as_bytes().to_vec()))
        }
        // Ignore → 整体写入（team 为唯一 writer）
        AssetType::Ignore => {
            ensure_parent_dir(target_abs)?;
            std::fs::write(target_abs, content).map_err(|e| e.to_string())?;
            Ok((target_abs.to_path_buf(), content.as_bytes().to_vec()))
        }
        // Permission → 合并进目标 JSON 的 _opensunstar_team 受管段
        AssetType::Permission => {
            if target_abs.extension().map(|e| e == "json").unwrap_or(false) {
                let written = merge_permission_json(target_abs, &step.asset_id, content)?;
                Ok((target_abs.to_path_buf(), written))
            } else {
                // 非 JSON 目标（如 Codex 的 .codex/config.toml）整体覆写会摧毁用户
                // 自有配置，且没有对应的安全合并实现——阻断而非覆盖。
                Err(format!(
                    "Permission 部署到非 JSON 目标 '{}' 尚未支持安全合并，已阻断以防覆盖用户配置",
                    step.target_path
                ))
            }
        }
        // 其他类型不应到达这里（is_deployable 已过滤）
        _ => Err(format!("不支持部署的资产类型: {:?}", step.asset_type)),
    }
}

/// 移除资产
fn remove_asset(step: &DeploymentStep, target_abs: &Path) -> Result<(), String> {
    match &step.asset_type {
        // Prompt/Rule → 移除受管区域（注入空内容）
        AssetType::Prompt | AssetType::Rule => {
            if !target_abs.is_file() {
                return Ok(());
            }
            let section_id = format!("team:{}", step.asset_id);
            let existing = std::fs::read_to_string(target_abs).map_err(|e| e.to_string())?;
            let merged =
                crate::services::marker_merge::inject_markdown_section(&existing, &section_id, "");
            std::fs::write(target_abs, merged).map_err(|e| e.to_string())
        }
        // Skill → 删除目录
        AssetType::Skill => {
            if target_abs.is_dir() {
                std::fs::remove_dir_all(target_abs).map_err(|e| e.to_string())?;
            }
            Ok(())
        }
        // Ignore → 删除文件（team 为唯一 writer，整文件归团队所有）
        AssetType::Ignore => {
            if target_abs.is_file() {
                std::fs::remove_file(target_abs).map_err(|e| e.to_string())?;
            }
            Ok(())
        }
        // Permission → 仅剔除受管条目。目标是 .claude/settings.json 这类共享文件，
        // 整体删除会连用户自有配置一起抹掉。
        AssetType::Permission => {
            if !target_abs.is_file() {
                return Ok(());
            }
            if target_abs.extension().map(|e| e == "json").unwrap_or(false) {
                remove_permission_json(target_abs, &step.asset_id)
            } else {
                Err(format!(
                    "Permission 从非 JSON 目标 '{}' 移除尚未支持，已阻断以防删除用户配置",
                    step.target_path
                ))
            }
        }
        // 其他类型不应到达这里（is_deployable 已过滤）
        _ => Err(format!("不支持移除的资产类型: {:?}", step.asset_type)),
    }
}

/// 从 JSON 设置文件中剔除团队受管权限条目（`merge_permission_json` 的逆操作）
fn remove_permission_json(target: &Path, asset_id: &str) -> Result<(), String> {
    let raw = std::fs::read_to_string(target).map_err(|e| e.to_string())?;
    let mut settings: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        // 目标不是合法 JSON：不猜测、不删除
        Err(e) => return Err(format!("目标不是合法 JSON，拒绝改动: {e}")),
    };

    if let Some(team) = settings
        .as_object_mut()
        .and_then(|o| o.get_mut("_opensunstar_team"))
        .and_then(|v| v.as_object_mut())
    {
        if let Some(perms) = team.get_mut("permissions").and_then(|v| v.as_object_mut()) {
            perms.remove(asset_id);
        }
    }

    let output = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(target, output).map_err(|e| e.to_string())
}

/// 合并权限到 JSON 设置文件，返回实际写入的字节
fn merge_permission_json(target: &Path, asset_id: &str, content: &str) -> Result<Vec<u8>, String> {
    ensure_parent_dir(target)?;

    let mut settings: serde_json::Value = if target.is_file() {
        let raw = std::fs::read_to_string(target).map_err(|e| e.to_string())?;
        serde_json::from_str(&raw).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // 将权限内容存入 _opensunstar_team.permissions[asset_id]
    let team_obj = settings
        .as_object_mut()
        .ok_or("settings.json 不是有效 JSON 对象")?
        .entry("_opensunstar_team")
        .or_insert_with(|| serde_json::json!({}));

    if let Some(obj) = team_obj.as_object_mut() {
        let perms = obj
            .entry("permissions")
            .or_insert_with(|| serde_json::json!({}));
        if let Some(p) = perms.as_object_mut() {
            // content 可能是 JSON 字符串或纯文本
            let value: serde_json::Value =
                serde_json::from_str(content).unwrap_or_else(|_| serde_json::json!(content));
            p.insert(asset_id.to_string(), value);
        }
    }

    let output = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    std::fs::write(target, &output).map_err(|e| e.to_string())?;
    Ok(output.into_bytes())
}

/// 回读校验：落盘内容必须与实际写入内容逐字节一致
///
/// 覆盖截断写入、权限受限、杀软改写、并发写入等导致"写了但没写对"的情况。
fn verify_written_bytes(path: &Path, expected: &[u8]) -> Result<(), String> {
    let actual = std::fs::read(path).map_err(|e| format!("写入校验回读失败: {e}"))?;
    if actual != expected {
        return Err(format!(
            "写入校验失败: 落盘 {} 字节与预期写入 {} 字节不一致",
            actual.len(),
            expected.len()
        ));
    }
    Ok(())
}

// ─── 辅助函数 ──────────────────────────────────────────────────────────────────

/// 确保父目录存在
fn ensure_parent_dir(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {e}"))?;
        }
    }
    Ok(())
}

/// 创建备份（复制到 {project_root}/.opensunstar/backups/ 目录）
///
/// W8 修复：回退到 project_root 而非 CWD
/// W5 修复：追加随机后缀防止同秒碰撞
///
/// 备份目录固定在 project_root 下：此前沿 `ancestors()` 向上寻找 `.opensunstar`
/// 会把备份写到项目外，导致回滚侧无法对 backup_path 做包含性校验。
fn create_backup(target: &Path, project_root: &Path) -> Result<String, String> {
    let backup_dir = backup_root(project_root);

    std::fs::create_dir_all(&backup_dir).map_err(|e| format!("创建备份目录失败: {e}"))?;

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let file_name = target
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    // W5: 追加 4 字节随机 hex 防止同秒碰撞
    let nonce: u32 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() ^ (d.as_secs() as u32))
        .unwrap_or(0);
    let backup_name = format!("{timestamp}_{file_name}_{nonce:08x}");
    let backup_path = backup_dir.join(&backup_name);

    if target.is_dir() {
        // 目录备份：递归复制（跳过 symlink）
        copy_dir_recursive(target, &backup_path)?;
    } else {
        std::fs::copy(target, &backup_path).map_err(|e| format!("备份复制失败: {e}"))?;
    }

    Ok(backup_path.to_string_lossy().to_string())
}

/// 递归复制目录（跳过 symlink 防止跟随攻击）
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        // C3: 跳过 symlink，防止跟随攻击（symlink → 系统目录）
        let file_type = entry.file_type().map_err(|e| e.to_string())?;
        if file_type.is_symlink() {
            continue;
        }

        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// 写入后验证：回读文件计算 SHA-256
///
/// W3 修复：使用传入的 target_app 而非从路径字符串反推
fn verify_write(
    target_abs: &Path,
    step: &DeploymentStep,
    target_app: &super::domain::TargetApp,
) -> Option<String> {
    match &step.asset_type {
        AssetType::Skill => {
            if target_abs.is_dir() {
                Some(
                    super::deployment::scan_current_sha256(
                        target_abs.parent()?.parent()?,
                        &step.asset_type,
                        &step.asset_id,
                        target_app,
                    )
                    .unwrap_or_default(),
                )
            } else {
                None
            }
        }
        _ => {
            if target_abs.is_file() {
                std::fs::read(target_abs)
                    .ok()
                    .map(|content| super::release::sha256_of_content(&content))
            } else {
                None
            }
        }
    }
}

/// 计算回执汇总
fn compute_receipt_summary(receipts: &[StepReceipt]) -> ReceiptSummary {
    let mut summary = ReceiptSummary {
        total_steps: receipts.len(),
        success_count: 0,
        failure_count: 0,
        skipped_count: 0,
        all_success: true,
    };

    for r in receipts {
        if r.success {
            if r.action.is_write() {
                summary.success_count += 1;
            } else {
                summary.skipped_count += 1;
            }
        } else {
            summary.failure_count += 1;
            summary.all_success = false;
        }
    }

    summary
}

// ─── 测试 ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::team_config::deployment::{
        DeploymentAction, DeploymentPlan, DeploymentStep, PlanSummary, PlanWarning,
    };
    use crate::team_config::domain::{AssetType, RiskLevel, TargetApp};
    use std::fs;
    use tempfile::TempDir;

    fn make_plan(steps: Vec<DeploymentStep>) -> DeploymentPlan {
        let write_count = steps.iter().filter(|s| s.action.is_write()).count();
        DeploymentPlan {
            project_id: "proj_test".to_string(),
            target_app: TargetApp::ClaudeCode,
            summary: PlanSummary {
                total_assets: steps.len(),
                create_count: steps
                    .iter()
                    .filter(|s| s.action == DeploymentAction::Create)
                    .count(),
                update_count: steps
                    .iter()
                    .filter(|s| s.action == DeploymentAction::Update)
                    .count(),
                remove_count: steps
                    .iter()
                    .filter(|s| s.action == DeploymentAction::Remove)
                    .count(),
                skip_count: steps
                    .iter()
                    .filter(|s| s.action == DeploymentAction::Skip)
                    .count(),
                display_only_count: steps
                    .iter()
                    .filter(|s| s.action == DeploymentAction::DisplayOnly)
                    .count(),
                write_count,
                has_high_risk: false,
            },
            steps,
            warnings: Vec::<PlanWarning>::new(),
            plan_sha256: "test_sha".to_string(),
        }
    }

    fn make_step(
        asset_type: AssetType,
        asset_id: &str,
        action: DeploymentAction,
        target_path: &str,
    ) -> DeploymentStep {
        DeploymentStep {
            asset_type,
            asset_id: asset_id.to_string(),
            action,
            risk_level: RiskLevel::Safe,
            target_path: target_path.to_string(),
            desired_sha256: Some("desired_hash".to_string()),
            current_sha256: None,
            explanation: "test".to_string(),
        }
    }

    #[test]
    fn execute_create_ignore_file() {
        let project = TempDir::new().unwrap();
        let team_pkg = TempDir::new().unwrap();

        // 创建团队包中的资产文件
        fs::create_dir_all(team_pkg.path().join("assets")).unwrap();
        fs::write(
            team_pkg.path().join("assets").join("secrets"),
            "*.secret\n.env\n",
        )
        .unwrap();

        let plan = make_plan(vec![make_step(
            AssetType::Ignore,
            "secrets",
            DeploymentAction::Create,
            ".claudeignore",
        )]);

        let options = ExecuteOptions {
            team_package_root: team_pkg.path().to_path_buf(),
            dry_run: false,
            create_backup: true,
        };

        let receipt = execute_deployment_plan(&plan, project.path(), &options);
        assert!(receipt.summary.all_success);
        assert_eq!(receipt.summary.success_count, 1);

        // 验证文件已写入
        let written = fs::read_to_string(project.path().join(".claudeignore")).unwrap();
        assert_eq!(written, "*.secret\n.env\n");
        assert!(receipt.steps[0].post_write_sha256.is_some());
    }

    #[test]
    fn execute_create_prompt_with_marker() {
        let project = TempDir::new().unwrap();
        let team_pkg = TempDir::new().unwrap();

        fs::create_dir_all(team_pkg.path().join("assets")).unwrap();
        fs::write(
            team_pkg.path().join("assets").join("greeting"),
            "Hello from team!",
        )
        .unwrap();

        // 预存一个 CLAUDE.md
        fs::write(project.path().join("CLAUDE.md"), "# My Project\n").unwrap();

        let plan = make_plan(vec![make_step(
            AssetType::Prompt,
            "greeting",
            DeploymentAction::Create,
            "CLAUDE.md",
        )]);

        let options = ExecuteOptions {
            team_package_root: team_pkg.path().to_path_buf(),
            dry_run: false,
            create_backup: true,
        };

        let receipt = execute_deployment_plan(&plan, project.path(), &options);
        assert!(receipt.summary.all_success);

        // 验证 marker 注入
        let content = fs::read_to_string(project.path().join("CLAUDE.md")).unwrap();
        assert!(content.contains("# My Project"));
        assert!(content.contains("<!-- opensunstar:team:greeting -->"));
        assert!(content.contains("Hello from team!"));
        assert!(content.contains("<!-- /opensunstar:team:greeting -->"));

        // 验证备份已创建
        assert!(receipt.steps[0].backup_path.is_some());
    }

    #[test]
    fn execute_remove_prompt_section() {
        let project = TempDir::new().unwrap();
        let team_pkg = TempDir::new().unwrap();
        fs::create_dir_all(team_pkg.path().join("assets")).unwrap();

        // 预存带 marker 的 CLAUDE.md
        let content = "# My Project\n\n<!-- opensunstar:team:greeting -->\nHello!\n<!-- /opensunstar:team:greeting -->\n";
        fs::write(project.path().join("CLAUDE.md"), content).unwrap();

        let plan = make_plan(vec![make_step(
            AssetType::Prompt,
            "greeting",
            DeploymentAction::Remove,
            "CLAUDE.md",
        )]);

        let options = ExecuteOptions {
            team_package_root: team_pkg.path().to_path_buf(),
            dry_run: false,
            create_backup: true,
        };

        let receipt = execute_deployment_plan(&plan, project.path(), &options);
        assert!(receipt.summary.all_success);

        let after = fs::read_to_string(project.path().join("CLAUDE.md")).unwrap();
        assert!(!after.contains("team:greeting"));
        assert!(after.contains("# My Project"));
    }

    #[test]
    fn execute_skill_directory_create() {
        let project = TempDir::new().unwrap();
        let team_pkg = TempDir::new().unwrap();

        fs::create_dir_all(team_pkg.path().join("assets").join("code-review")).unwrap();
        fs::write(
            team_pkg
                .path()
                .join("assets")
                .join("code-review")
                .join("SKILL.md"),
            "# Code Review Skill\nReview code carefully.",
        )
        .unwrap();

        let plan = make_plan(vec![make_step(
            AssetType::Skill,
            "code-review",
            DeploymentAction::Create,
            ".claude/skills/code-review",
        )]);

        let options = ExecuteOptions {
            team_package_root: team_pkg.path().to_path_buf(),
            dry_run: false,
            create_backup: true,
        };

        let receipt = execute_deployment_plan(&plan, project.path(), &options);
        assert!(receipt.summary.all_success);

        let skill_md = project
            .path()
            .join(".claude")
            .join("skills")
            .join("code-review")
            .join("SKILL.md");
        assert!(skill_md.is_file());
        let content = fs::read_to_string(&skill_md).unwrap();
        assert!(content.contains("Code Review Skill"));
    }

    #[test]
    fn dry_run_does_not_write() {
        let project = TempDir::new().unwrap();
        let team_pkg = TempDir::new().unwrap();
        fs::create_dir_all(team_pkg.path().join("assets")).unwrap();
        fs::write(team_pkg.path().join("assets").join("x"), "content").unwrap();

        let plan = make_plan(vec![make_step(
            AssetType::Ignore,
            "x",
            DeploymentAction::Create,
            ".claudeignore",
        )]);

        let options = ExecuteOptions {
            team_package_root: team_pkg.path().to_path_buf(),
            dry_run: true,
            create_backup: true,
        };

        let receipt = execute_deployment_plan(&plan, project.path(), &options);
        assert!(receipt.summary.all_success);
        // 文件不应被创建
        assert!(!project.path().join(".claudeignore").exists());
        assert!(receipt.steps[0].error.as_ref().unwrap().contains("dry-run"));
    }

    #[test]
    fn skip_steps_are_not_executed() {
        let project = TempDir::new().unwrap();
        let team_pkg = TempDir::new().unwrap();

        let plan = make_plan(vec![
            make_step(
                AssetType::Mcp,
                "server",
                DeploymentAction::DisplayOnly,
                ".mcp.json",
            ),
            make_step(
                AssetType::Ignore,
                "x",
                DeploymentAction::Skip,
                ".claudeignore",
            ),
        ]);

        let options = ExecuteOptions {
            team_package_root: team_pkg.path().to_path_buf(),
            dry_run: false,
            create_backup: true,
        };

        let receipt = execute_deployment_plan(&plan, project.path(), &options);
        assert!(receipt.summary.all_success);
        assert_eq!(receipt.summary.skipped_count, 2);
        assert_eq!(receipt.summary.success_count, 0);
    }

    #[test]
    fn missing_asset_content_reports_error() {
        let project = TempDir::new().unwrap();
        let team_pkg = TempDir::new().unwrap();
        // 不创建资产文件

        let plan = make_plan(vec![make_step(
            AssetType::Ignore,
            "nonexistent",
            DeploymentAction::Create,
            ".claudeignore",
        )]);

        let options = ExecuteOptions {
            team_package_root: team_pkg.path().to_path_buf(),
            dry_run: false,
            create_backup: true,
        };

        let receipt = execute_deployment_plan(&plan, project.path(), &options);
        assert!(!receipt.summary.all_success);
        assert_eq!(receipt.summary.failure_count, 1);
        assert!(receipt.steps[0].error.is_some());
    }

    #[test]
    fn write_verification_detects_on_disk_mismatch() {
        let dir = TempDir::new().unwrap();
        let target = dir.path().join("a.txt");
        fs::write(&target, b"actual bytes").unwrap();

        assert!(
            verify_written_bytes(&target, b"expected bytes").is_err(),
            "落盘内容与写入内容不一致时必须报错"
        );
        assert!(verify_written_bytes(&target, b"actual bytes").is_ok());
    }

    #[test]
    fn write_verification_detects_missing_file() {
        let dir = TempDir::new().unwrap();
        let missing = dir.path().join("gone.txt");
        assert!(verify_written_bytes(&missing, b"anything").is_err());
    }

    #[test]
    fn remove_permission_preserves_settings_file() {
        let project = TempDir::new().unwrap();
        let team_pkg = TempDir::new().unwrap();
        fs::create_dir_all(team_pkg.path().join("assets")).unwrap();

        // 用户已有的 settings.json，含团队受管段和用户自有配置
        let settings_path = project.path().join(".claude").join("settings.json");
        fs::create_dir_all(settings_path.parent().unwrap()).unwrap();
        fs::write(
            &settings_path,
            r#"{"userSetting":"keep me","_opensunstar_team":{"permissions":{"bash-policy":{"allow":["ls"]}}}}"#,
        )
        .unwrap();

        let plan = make_plan(vec![make_step(
            AssetType::Permission,
            "bash-policy",
            DeploymentAction::Remove,
            ".claude/settings.json",
        )]);

        let options = ExecuteOptions {
            team_package_root: team_pkg.path().to_path_buf(),
            dry_run: false,
            create_backup: true,
        };

        let receipt = execute_deployment_plan(&plan, project.path(), &options);
        assert!(receipt.summary.all_success, "{:?}", receipt.steps[0].error);

        assert!(settings_path.is_file(), "共享配置文件不得被整体删除");
        let after: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&settings_path).unwrap()).unwrap();
        assert_eq!(after["userSetting"], "keep me", "用户自有配置必须保留");
        assert!(
            after["_opensunstar_team"]["permissions"]
                .get("bash-policy")
                .is_none(),
            "仅团队受管条目应被剔除"
        );
    }
}
