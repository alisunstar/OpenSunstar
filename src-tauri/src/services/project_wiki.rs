//! Project Wiki Baseline 服务层
//!
//! 提供项目 Wiki 的扫描、inventory、lint、初始化和变更映射能力。
//!
//! 设计约束：
//! - lint 核心逻辑 Rust 原生实现，不依赖 Python 运行时（修正 1）
//! - wiki 的 stale/invalid 状态复用 asset_health 的 DRIFTED 机制（修正 3）
//! - changed-files 映射在冷启动态（effective_source_pages < 5）不执行（修正 2）
//! - 不保存 Wiki 正文到数据库，状态存 .opensunstar/wiki/*.json

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::AppError;

// ── 模板嵌入 ─────────────────────────────────
// 将 wiki 脚手架模板嵌入二进制，避免运行时文件依赖。

const TMPL_INDEX: &str = include_str!("../../assets/recipes/wiki/index.md.tmpl");
const TMPL_LOG: &str = include_str!("../../assets/recipes/wiki/log.md.tmpl");
const TMPL_SCHEMA: &str = include_str!("../../assets/recipes/wiki/SCHEMA.md.tmpl");
const TMPL_OVERVIEW: &str = include_str!("../../assets/recipes/wiki/overview.md.tmpl");
const TMPL_SOURCE_MAP: &str = include_str!("../../assets/recipes/wiki/source-map.md.tmpl");
const TMPL_CONFIG_CACHE: &str =
    include_str!("../../assets/recipes/wiki/components/config-and-cache.md.tmpl");
const TMPL_EXTERNAL_DEP: &str =
    include_str!("../../assets/recipes/wiki/components/external-dependencies.md.tmpl");
const TMPL_AUTH_IDENTITY: &str =
    include_str!("../../assets/recipes/wiki/flows/auth-and-identity.md.tmpl");
const TMPL_BUSINESS_FLOWS: &str =
    include_str!("../../assets/recipes/wiki/flows/business-flows.md.tmpl");
const TMPL_FIELD_PROP: &str =
    include_str!("../../assets/recipes/wiki/flows/field-propagation.md.tmpl");
const TMPL_HTTP_ENDPOINTS: &str =
    include_str!("../../assets/recipes/wiki/apis/http-endpoints.md.tmpl");
const TMPL_REQUEST_TROUBLESHOOT: &str =
    include_str!("../../assets/recipes/wiki/runbooks/request-troubleshooting.md.tmpl");
const TMPL_IDL_SOURCE: &str =
    include_str!("../../assets/recipes/wiki/questions/idl-source-location.md.tmpl");
const TMPL_OPS_METADATA: &str =
    include_str!("../../assets/recipes/wiki/questions/operations-metadata.md.tmpl");
const TMPL_QUERIES_KEEP: &str = include_str!("../../assets/recipes/wiki/queries/.gitkeep");
const TMPL_DECISIONS_KEEP: &str = include_str!("../../assets/recipes/wiki/decisions/.gitkeep");
const TMPL_SCRIPTS_REF_README: &str =
    include_str!("../../assets/recipes/wiki/scripts/reference/README.md.tmpl");

// ── 常量 ──────────────────────────────────────

/// wiki 目录名
const WIKI_DIR: &str = "wiki";

/// wiki/index.md 路径（相对项目根）
const WIKI_INDEX_REL: &str = "wiki/index.md";

/// .opensunstar/wiki/ 目录名
const WIKI_STATE_DIR: &str = ".opensunstar/wiki";

/// profile.json 文件名
const WIKI_PROFILE_FILE: &str = "profile.json";

/// lint-result.json 文件名
const WIKI_LINT_RESULT_FILE: &str = "lint-result.json";

/// inventory.json 文件名
const WIKI_INVENTORY_FILE: &str = "inventory.json";

/// changed-files 冷启动默认阈值
const DEFAULT_CHANGED_FILES_THRESHOLD: u32 = 5;

/// 跳过扫描的目录名
const SKIP_DIRS: &[&str] = &["node_modules", "target", "dist", ".git"];

// ── 扫描结果 ──────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WikiScanResult {
    pub project_id: String,
    pub wiki_root: String,
    pub exists: bool,
    pub base_status: String,
    pub quality_level: String,
    pub page_count: u32,
    pub core_page_coverage: WikiCorePageCoverage,
    pub source_ref_count: u32,
    pub question_count: u32,
    pub latest_mtime: Option<i64>,
    pub content_sha256: Option<String>,
    pub last_lint_passed: Option<bool>,
    pub last_lint_at: Option<i64>,
    pub checked_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WikiCorePageCoverage {
    pub has_index: bool,
    pub has_overview: bool,
    pub has_source_map: bool,
    pub has_log: bool,
    pub has_schema: bool,
    pub component_pages: u32,
    pub flow_pages: u32,
    pub api_pages: u32,
    pub runbook_pages: u32,
}

// ── Inventory ─────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WikiInventory {
    pub project_id: String,
    pub pages: Vec<WikiPageMeta>,
    pub summary: WikiInventorySummary,
    pub generated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WikiPageMeta {
    pub path: String,
    pub title: String,
    pub page_type: String,
    pub status: String,
    pub source_files: Vec<String>,
    pub last_verified: Option<String>,
    pub last_verified_commit: Option<String>,
    pub tags: Vec<String>,
    pub mtime: i64,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WikiInventorySummary {
    pub total_pages: u32,
    pub by_type: HashMap<String, u32>,
    pub by_status: HashMap<String, u32>,
    pub effective_source_pages: u32,
}

// ── Lint 结果 ─────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WikiLintResult {
    pub project_id: String,
    pub wiki_root: String,
    pub checked_at: i64,
    pub quality_mode: bool,
    pub errors: Vec<WikiLintIssue>,
    pub warnings: Vec<WikiLintIssue>,
    pub summary: WikiLintSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WikiLintIssue {
    pub rule_id: String,
    pub file: String,
    pub line: Option<u32>,
    pub message: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WikiLintSummary {
    pub total_files: u32,
    pub error_count: u32,
    pub warning_count: u32,
    pub passed: bool,
    pub quality_level: String,
}

// ── 初始化 ────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WikiInitPlan {
    pub project_id: String,
    pub files: Vec<WikiInitFilePlan>,
    pub audit: WikiInitAudit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WikiInitFilePlan {
    pub target_path: String,
    pub source_template: String,
    pub will_create: bool,
    pub already_exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WikiInitAudit {
    pub blocked: bool,
    pub existing_wiki_files: u32,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WikiInitResult {
    pub project_id: String,
    pub files_created: Vec<String>,
    pub files_skipped: Vec<String>,
    pub profile_path: String,
}

// ── Changed-Files ─────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WikiChangedFilesResult {
    pub cold_start: bool,
    pub effective_source_pages: u32,
    pub threshold: u32,
    pub changed_files: Vec<String>,
    pub affected_pages: Vec<String>,
    pub unmapped_changed_files: Vec<String>,
    pub guidance: Option<String>,
}

// ── 服务函数 ──────────────────────────────────

/// 扫描项目 Wiki 状态
pub fn scan_project_wiki(project_path: &str, project_id: &str) -> Result<WikiScanResult, AppError> {
    let root = PathBuf::from(project_path);
    if !root.is_dir() {
        return Err(AppError::Message(format!(
            "项目路径不存在或不是目录: {project_path}"
        )));
    }

    let wiki_root = root.join(WIKI_DIR);
    let index_path = root.join(WIKI_INDEX_REL);
    let exists = index_path.is_file();

    let now = Utc::now().timestamp();

    if !exists {
        return Ok(WikiScanResult {
            project_id: project_id.to_string(),
            wiki_root: wiki_root.to_string_lossy().to_string(),
            exists: false,
            base_status: "missing".to_string(),
            quality_level: "N/A".to_string(),
            page_count: 0,
            core_page_coverage: WikiCorePageCoverage::default(),
            source_ref_count: 0,
            question_count: 0,
            latest_mtime: None,
            content_sha256: None,
            last_lint_passed: None,
            last_lint_at: None,
            checked_at: now,
        });
    }

    // 扫描 wiki/ 目录
    let pages = collect_wiki_pages(&wiki_root)?;
    let core_coverage = compute_core_page_coverage(&root, &pages);
    let content_sha256 = compute_wiki_content_hash(&wiki_root, &pages)?;
    let latest_mtime = pages.iter().map(|p| p.mtime).max();
    let source_ref_count = count_source_refs(&root, &pages);
    let question_count = pages.iter().filter(|p| p.page_type == "question").count() as u32;

    // 读取上次 lint 结果
    let last_lint = read_last_lint_result(&root);

    // 判定 base_status
    let base_status = determine_base_status(&core_coverage, &content_sha256, &last_lint);

    // 判定 quality_level（P1 简化版：基于核心页面覆盖率）
    let quality_level = determine_quality_level(&core_coverage, &base_status);

    Ok(WikiScanResult {
        project_id: project_id.to_string(),
        wiki_root: wiki_root.to_string_lossy().to_string(),
        exists: true,
        base_status,
        quality_level,
        page_count: pages.len() as u32,
        core_page_coverage: core_coverage,
        source_ref_count,
        question_count,
        latest_mtime,
        content_sha256: Some(content_sha256),
        last_lint_passed: last_lint.as_ref().map(|l| l.passed),
        last_lint_at: last_lint.map(|l| l.checked_at),
        checked_at: now,
    })
}

/// 构建 Wiki Inventory
pub fn build_wiki_inventory(
    project_path: &str,
    project_id: &str,
) -> Result<WikiInventory, AppError> {
    let root = PathBuf::from(project_path);
    let wiki_root = root.join(WIKI_DIR);

    if !wiki_root.is_dir() {
        return Ok(WikiInventory {
            project_id: project_id.to_string(),
            pages: Vec::new(),
            summary: WikiInventorySummary {
                total_pages: 0,
                by_type: HashMap::new(),
                by_status: HashMap::new(),
                effective_source_pages: 0,
            },
            generated_at: Utc::now().timestamp(),
        });
    }

    let pages = collect_wiki_pages(&wiki_root)?;

    let mut by_type: HashMap<String, u32> = HashMap::new();
    let mut by_status: HashMap<String, u32> = HashMap::new();
    let mut effective_source_pages = 0u32;

    for page in &pages {
        *by_type.entry(page.page_type.clone()).or_default() += 1;
        *by_status.entry(page.status.clone()).or_default() += 1;

        // 判定是否为有效 source_files 页面
        if !page.source_files.is_empty()
            && page.source_files.iter().any(|s| {
                let p = root.join(s);
                p.exists()
            })
        {
            effective_source_pages += 1;
        }
    }

    let summary = WikiInventorySummary {
        total_pages: pages.len() as u32,
        by_type,
        by_status,
        effective_source_pages,
    };

    // 写入 .opensunstar/wiki/inventory.json
    let inventory = WikiInventory {
        project_id: project_id.to_string(),
        pages: pages
            .iter()
            .map(|p| WikiPageMeta {
                path: p.path.clone(),
                title: p.title.clone(),
                page_type: p.page_type.clone(),
                status: p.status.clone(),
                source_files: p.source_files.clone(),
                last_verified: None,
                last_verified_commit: None,
                tags: Vec::new(),
                mtime: p.mtime,
                size_bytes: 0,
            })
            .collect(),
        summary: summary.clone(),
        generated_at: Utc::now().timestamp(),
    };

    let _ = write_state_file(&root, WIKI_INVENTORY_FILE, &inventory);

    Ok(inventory)
}

/// 运行 Wiki Lint（Rust 原生实现，修正 1：零 Python 依赖）
pub fn run_wiki_lint(
    project_path: &str,
    project_id: &str,
    quality_mode: bool,
) -> Result<WikiLintResult, AppError> {
    let root = PathBuf::from(project_path);
    let wiki_root = root.join(WIKI_DIR);

    if !wiki_root.is_dir() {
        let result = WikiLintResult {
            project_id: project_id.to_string(),
            wiki_root: wiki_root.to_string_lossy().to_string(),
            checked_at: Utc::now().timestamp(),
            quality_mode,
            errors: Vec::new(),
            warnings: Vec::new(),
            summary: WikiLintSummary {
                total_files: 0,
                error_count: 0,
                warning_count: 0,
                passed: false,
                quality_level: "N/A".to_string(),
            },
        };
        return Ok(result);
    }

    let now = Utc::now().timestamp();
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let mut total_files = 0u32;

    // S001: 检查 index.md
    let has_index = wiki_root.join("index.md").is_file();
    if !has_index {
        errors.push(WikiLintIssue {
            rule_id: "S001".to_string(),
            file: "index.md".to_string(),
            line: None,
            message: "缺少 wiki/index.md".to_string(),
            severity: "error".to_string(),
        });
    }

    // 收集所有 wiki 页面
    let page_paths = collect_wiki_file_paths(&wiki_root)?;

    // 读取 index.md 的内容用于 S009 检查
    let index_content = if has_index {
        fs::read_to_string(wiki_root.join("index.md")).unwrap_or_default()
    } else {
        String::new()
    };

    for (rel_path, full_path) in &page_paths {
        total_files += 1;
        let content = fs::read_to_string(full_path).unwrap_or_default();
        let is_questions = rel_path.starts_with("questions/");

        // S002: 非 questions/ 页面缺少 frontmatter
        let (frontmatter, body) = split_frontmatter(&content);
        if frontmatter.is_none() && !is_questions {
            errors.push(WikiLintIssue {
                rule_id: "S002".to_string(),
                file: rel_path.clone(),
                line: Some(1),
                message: "缺少 frontmatter".to_string(),
                severity: "error".to_string(),
            });
            continue;
        }

        let fm_text = frontmatter.clone().unwrap_or_default();
        let yaml: serde_yaml::Value =
            serde_yaml::from_str(&fm_text).unwrap_or(serde_yaml::Value::Null);
        let map = match &yaml {
            serde_yaml::Value::Mapping(m) => m,
            _ => {
                if !is_questions {
                    errors.push(WikiLintIssue {
                        rule_id: "S012".to_string(),
                        file: rel_path.clone(),
                        line: None,
                        message: "frontmatter YAML 解析失败".to_string(),
                        severity: "error".to_string(),
                    });
                }
                continue;
            }
        };

        // S003/S004/S005: 必填字段
        if !is_questions {
            for (field, rule_id) in [("title", "S003"), ("type", "S004"), ("status", "S005")] {
                if !map.contains_key(serde_yaml::Value::String(field.to_string())) {
                    errors.push(WikiLintIssue {
                        rule_id: rule_id.to_string(),
                        file: rel_path.clone(),
                        line: None,
                        message: format!("frontmatter 缺少 {}", field),
                        severity: "error".to_string(),
                    });
                }
            }
        }

        // S006: type 枚举
        let valid_types = [
            "overview",
            "component",
            "flow",
            "api",
            "runbook",
            "query",
            "question",
            "decision",
        ];
        if let Some(t) = map
            .get(&serde_yaml::Value::String("type".into()))
            .and_then(|v| v.as_str())
        {
            if !valid_types.contains(&t) {
                errors.push(WikiLintIssue {
                    rule_id: "S006".to_string(),
                    file: rel_path.clone(),
                    line: None,
                    message: format!("type 值不在枚举范围内: {}", t),
                    severity: "error".to_string(),
                });
            }
        }

        // S007: status 枚举
        let valid_statuses = ["active", "draft", "retired"];
        if let Some(s) = map
            .get(&serde_yaml::Value::String("status".into()))
            .and_then(|v| v.as_str())
        {
            if !valid_statuses.contains(&s) {
                errors.push(WikiLintIssue {
                    rule_id: "S007".to_string(),
                    file: rel_path.clone(),
                    line: None,
                    message: format!("status 值不在枚举范围内: {}", s),
                    severity: "error".to_string(),
                });
            }
        }

        // S008: 相对 Markdown 链接断裂
        for (line_num, line) in body.lines().enumerate() {
            for cap in REGEX_MD_LINK.captures_iter(line) {
                let link = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                if link.starts_with("http://")
                    || link.starts_with("https://")
                    || link.starts_with("#")
                {
                    continue;
                }
                let resolved = resolve_wiki_link(&wiki_root, full_path, link);
                if !resolved.exists() {
                    warnings.push(WikiLintIssue {
                        rule_id: "S008".to_string(),
                        file: rel_path.clone(),
                        line: Some((line_num + 1) as u32),
                        message: format!("链接断裂: {}", link),
                        severity: "warning".to_string(),
                    });
                }
            }
        }

        // S009: 核心页面未被 index.md 收录
        let core_pages = ["overview.md", "source-map.md"];
        if core_pages.contains(&rel_path.as_str()) {
            let link_pattern = format!("[{}]({})", rel_path.replace(".md", ""), rel_path);
            if !index_content.contains(&link_pattern) && !index_content.contains(rel_path) {
                warnings.push(WikiLintIssue {
                    rule_id: "S009".to_string(),
                    file: "index.md".to_string(),
                    line: None,
                    message: format!("核心页面未被 index.md 收录: {}", rel_path),
                    severity: "warning".to_string(),
                });
            }
        }

        // S010: source_files 指向文件不存在（跳过 glob 模式）
        if let Some(sources) = map
            .get(&serde_yaml::Value::String("source_files".into()))
            .and_then(|v| v.as_sequence())
        {
            for src in sources.iter().filter_map(|v| v.as_str()) {
                if src.contains('*') {
                    continue; // glob 模式跳过
                }
                let resolved = root.join(src);
                if !resolved.exists() {
                    // retired decisions 豁免
                    let is_retired = map
                        .get(&serde_yaml::Value::String("status".into()))
                        .and_then(|v| v.as_str())
                        == Some("retired");
                    if !is_retired {
                        errors.push(WikiLintIssue {
                            rule_id: "S010".to_string(),
                            file: rel_path.clone(),
                            line: None,
                            message: format!("source_files 指向的文件不存在: {}", src),
                            severity: "error".to_string(),
                        });
                    }
                }
            }
        }

        // S011: 正文残留 TODO/TBD/FIXME
        for (line_num, line) in body.lines().enumerate() {
            let lower = line.to_lowercase();
            if lower.contains("todo") || lower.contains("tbd") || lower.contains("fixme") {
                warnings.push(WikiLintIssue {
                    rule_id: "S011".to_string(),
                    file: rel_path.clone(),
                    line: Some((line_num + 1) as u32),
                    message: "正文残留 TODO/TBD/FIXME".to_string(),
                    severity: "warning".to_string(),
                });
            }
        }

        // Quality 模式检查（Q001-Q010）
        if quality_mode {
            let page_type = map
                .get(&serde_yaml::Value::String("type".into()))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            check_quality_rules(page_type, &body, rel_path, &mut warnings);
        }
    }

    let error_count = errors.len() as u32;
    let warning_count = warnings.len() as u32;
    let passed = error_count == 0;
    let quality_level = if !passed {
        "N/A".to_string()
    } else {
        compute_quality_level_from_wiki(&wiki_root, &root)
    };

    let result = WikiLintResult {
        project_id: project_id.to_string(),
        wiki_root: wiki_root.to_string_lossy().to_string(),
        checked_at: now,
        quality_mode,
        errors,
        warnings,
        summary: WikiLintSummary {
            total_files,
            error_count,
            warning_count,
            passed,
            quality_level,
        },
    };

    // 写入 .opensunstar/wiki/lint-result.json
    let _ = write_state_file(&root, WIKI_LINT_RESULT_FILE, &result);

    Ok(result)
}

/// 预览 Wiki 初始化（safe install 预检）
pub fn preview_wiki_init(project_path: &str, project_id: &str) -> Result<WikiInitPlan, AppError> {
    let root = PathBuf::from(project_path);
    if !root.is_dir() {
        return Err(AppError::Message(format!(
            "项目路径不存在或不是目录: {project_path}"
        )));
    }

    let templates = get_template_list();
    let mut files = Vec::new();
    let mut existing_wiki_files = 0u32;
    let mut warnings = Vec::new();

    for (target_rel, _source) in &templates {
        let target = root.join(target_rel);
        let already_exists = target.exists();
        if already_exists {
            existing_wiki_files += 1;
        }
        files.push(WikiInitFilePlan {
            target_path: target_rel.to_string(),
            source_template: _source.to_string(),
            will_create: !already_exists,
            already_exists,
        });
    }

    // 安全审计：若 wiki/index.md 已存在，标记 blocked
    let blocked = root.join("wiki/index.md").exists();
    if blocked {
        warnings.push("wiki/index.md 已存在，初始化将跳过所有已存在文件".to_string());
    }

    Ok(WikiInitPlan {
        project_id: project_id.to_string(),
        files,
        audit: WikiInitAudit {
            blocked: false, // 不阻断，只是跳过已存在文件
            existing_wiki_files,
            warnings,
        },
    })
}

/// 初始化 Wiki（safe install，never overwrite）
pub fn init_project_wiki(
    project_path: &str,
    project_id: &str,
    project_name: &str,
) -> Result<WikiInitResult, AppError> {
    let root = PathBuf::from(project_path);
    if !root.is_dir() {
        return Err(AppError::Message(format!(
            "项目路径不存在或不是目录: {project_path}"
        )));
    }

    let date = Utc::now().format("%Y-%m-%d").to_string();
    let timestamp = Utc::now().timestamp().to_string();
    let vars = TemplateVars {
        project_name: project_name.to_string(),
        date: date.clone(),
        timestamp: timestamp.clone(),
    };

    let templates = get_template_list();
    let mut files_created = Vec::new();
    let mut files_skipped = Vec::new();

    for (target_rel, source_template) in &templates {
        let target = root.join(target_rel);
        if target.exists() {
            files_skipped.push(target_rel.to_string());
            continue;
        }

        // 创建父目录
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                AppError::Message(format!("创建目录失败 {}: {e}", parent.display()))
            })?;
        }

        let content = render_template(source_template, &vars);
        fs::write(&target, &content)
            .map_err(|e| AppError::Message(format!("写入文件失败 {}: {e}", target.display())))?;
        files_created.push(target_rel.to_string());
    }

    // 写入 profile.json（若不存在）
    let profile_path = root.join(WIKI_STATE_DIR).join(WIKI_PROFILE_FILE);
    if !profile_path.exists() {
        let profile = serde_json::json!({
            "version": 1,
            "mode": "repo",
            "initialized_at": Utc::now().timestamp(),
            "quality_target": "l2",
            "gate_enabled": false,
            "changed_files_threshold": DEFAULT_CHANGED_FILES_THRESHOLD,
        });
        if let Some(parent) = profile_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                AppError::Message(format!("创建目录失败 {}: {e}", parent.display()))
            })?;
        }
        let profile_json = serde_json::to_string_pretty(&profile)
            .map_err(|e| AppError::Message(format!("序列化 profile.json 失败: {e}")))?;
        fs::write(&profile_path, &profile_json)
            .map_err(|e| AppError::Message(format!("写入 profile.json 失败: {e}")))?;
        files_created.push(WIKI_PROFILE_FILE.to_string());
    } else {
        files_skipped.push(WIKI_PROFILE_FILE.to_string());
    }

    Ok(WikiInitResult {
        project_id: project_id.to_string(),
        files_created,
        files_skipped,
        profile_path: profile_path.to_string_lossy().to_string(),
    })
}

/// 映射变更文件到 wiki 页面（修正 2：冷启动检测）
pub fn map_wiki_changed_files(
    project_path: &str,
    changed_files: Option<Vec<String>>,
) -> Result<WikiChangedFilesResult, AppError> {
    let root = PathBuf::from(project_path);
    let wiki_root = root.join(WIKI_DIR);

    if !wiki_root.is_dir() {
        return Ok(WikiChangedFilesResult {
            cold_start: true,
            effective_source_pages: 0,
            threshold: DEFAULT_CHANGED_FILES_THRESHOLD,
            changed_files: Vec::new(),
            affected_pages: Vec::new(),
            unmapped_changed_files: Vec::new(),
            guidance: Some("Wiki 未初始化，请先运行 os wiki init".to_string()),
        });
    }

    // 获取变更文件列表
    let changed = match changed_files {
        Some(files) => files,
        None => git_changed_files(&root).unwrap_or_default(),
    };

    // 读取 profile.json 获取阈值
    let threshold = read_changed_files_threshold(&root);

    // 构建 wiki 页面的 source_files 反向索引
    let pages = collect_wiki_pages(&wiki_root)?;
    let mut effective_source_pages = 0u32;
    let mut source_index: Vec<(String, Vec<String>)> = Vec::new();

    for page in &pages {
        let sources: Vec<String> = page
            .source_files
            .iter()
            .filter(|s| {
                if s.contains('*') {
                    return true;
                }
                root.join(s).exists()
            })
            .cloned()
            .collect();
        if !sources.is_empty() {
            effective_source_pages += 1;
        }
        source_index.push((page.path.clone(), sources));
    }

    // 冷启动检测（修正 2）
    if effective_source_pages < threshold {
        let guidance = format!(
            "Wiki 刚初始化，尚无足够页面包含有效 source_files 引用。当前 {} 个有效页面，需达到 {} 个后 changed-files 映射自动生效。请在 Claude Code/Codex 中使用 maintain-repo-wiki Skill 填充 wiki 内容。",
            effective_source_pages, threshold
        );
        return Ok(WikiChangedFilesResult {
            cold_start: true,
            effective_source_pages,
            threshold,
            changed_files: changed.clone(),
            affected_pages: Vec::new(),
            unmapped_changed_files: Vec::new(),
            guidance: Some(guidance),
        });
    }

    // 正常态：执行映射
    let changed_set: std::collections::HashSet<&str> = changed.iter().map(|s| s.as_str()).collect();
    let mut affected_pages = Vec::new();
    let mut mapped_files = std::collections::HashSet::new();

    for (page_path, sources) in &source_index {
        let intersection: Vec<_> = sources
            .iter()
            .filter(|s| {
                // glob 匹配
                if s.contains('*') {
                    return glob_matches(s, &changed_set);
                }
                changed_set.contains(s.as_str())
            })
            .collect();
        if !intersection.is_empty() {
            affected_pages.push(page_path.clone());
            mapped_files.extend(intersection.into_iter().cloned());
        }
    }

    affected_pages.sort();

    let unmapped: Vec<String> = changed
        .iter()
        .filter(|f| !mapped_files.contains(*f))
        .cloned()
        .collect();

    Ok(WikiChangedFilesResult {
        cold_start: false,
        effective_source_pages,
        threshold,
        changed_files: changed,
        affected_pages,
        unmapped_changed_files: unmapped,
        guidance: None,
    })
}

// ── 内部辅助函数 ──────────────────────────────

/// Markdown 链接正则：匹配 [text](path) 中的 path
static REGEX_MD_LINK: once_cell::sync::Lazy<regex::Regex> =
    once_cell::sync::Lazy::new(|| regex::Regex::new(r"\[[^\]]*\]\(([^)]+)\)").unwrap());

/// 模板变量
struct TemplateVars {
    project_name: String,
    date: String,
    timestamp: String,
}

/// 渲染模板变量
fn render_template(template: &str, vars: &TemplateVars) -> String {
    template
        .replace("{{project_name}}", &vars.project_name)
        .replace("{{date}}", &vars.date)
        .replace("{{timestamp}}", &vars.timestamp)
}

/// 获取模板列表：(target_rel_path, source_template_const_name)
fn get_template_list() -> Vec<(&'static str, &'static str)> {
    vec![
        ("wiki/index.md", TMPL_INDEX),
        ("wiki/log.md", TMPL_LOG),
        ("wiki/SCHEMA.md", TMPL_SCHEMA),
        ("wiki/overview.md", TMPL_OVERVIEW),
        ("wiki/source-map.md", TMPL_SOURCE_MAP),
        ("wiki/components/config-and-cache.md", TMPL_CONFIG_CACHE),
        (
            "wiki/components/external-dependencies.md",
            TMPL_EXTERNAL_DEP,
        ),
        ("wiki/flows/auth-and-identity.md", TMPL_AUTH_IDENTITY),
        ("wiki/flows/business-flows.md", TMPL_BUSINESS_FLOWS),
        ("wiki/flows/field-propagation.md", TMPL_FIELD_PROP),
        ("wiki/apis/http-endpoints.md", TMPL_HTTP_ENDPOINTS),
        (
            "wiki/runbooks/request-troubleshooting.md",
            TMPL_REQUEST_TROUBLESHOOT,
        ),
        ("wiki/questions/idl-source-location.md", TMPL_IDL_SOURCE),
        ("wiki/questions/operations-metadata.md", TMPL_OPS_METADATA),
        ("wiki/queries/.gitkeep", TMPL_QUERIES_KEEP),
        ("wiki/decisions/.gitkeep", TMPL_DECISIONS_KEEP),
        ("wiki/scripts/reference/README.md", TMPL_SCRIPTS_REF_README),
    ]
}

/// 收集 wiki 目录下所有 .md 文件的 (rel_path, full_path)
fn collect_wiki_file_paths(wiki_root: &Path) -> Result<Vec<(String, PathBuf)>, AppError> {
    let mut result = Vec::new();
    collect_wiki_file_paths_recursive(wiki_root, wiki_root, &mut result)?;
    Ok(result)
}

fn collect_wiki_file_paths_recursive(
    wiki_root: &Path,
    current: &Path,
    result: &mut Vec<(String, PathBuf)>,
) -> Result<(), AppError> {
    let entries = fs::read_dir(current)
        .map_err(|e| AppError::Message(format!("遍历目录失败 {}: {e}", current.display())))?;

    for entry in entries.flatten() {
        let path = entry.path();
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        if metadata.is_dir() {
            let name = path.file_name().map(|n| n.to_string_lossy().to_string());
            if name.as_deref().is_some_and(|n| SKIP_DIRS.contains(&n)) {
                continue;
            }
            collect_wiki_file_paths_recursive(wiki_root, &path, result)?;
        } else if metadata.is_file() && path.extension().is_some_and(|ext| ext == "md") {
            let rel = path
                .strip_prefix(wiki_root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            result.push((rel, path));
        }
    }
    Ok(())
}

/// 解析 Markdown 链接为绝对路径
fn resolve_wiki_link(wiki_root: &Path, source_file: &Path, link: &str) -> PathBuf {
    // 去除锚点
    let link_path = link.split('#').next().unwrap_or(link);
    if link_path.is_empty() {
        return PathBuf::new();
    }

    // 相对链接：基于 source_file 所在目录解析
    if let Some(dir) = source_file.parent() {
        let resolved = dir.join(link_path);
        if resolved.exists() {
            return resolved;
        }
    }

    // 基于 wiki_root 解析
    let from_root = wiki_root.join(link_path);
    if from_root.exists() {
        return from_root;
    }

    // 尝试加 .md 后缀
    if !link_path.ends_with(".md") {
        let with_md = wiki_root.join(format!("{}.md", link_path));
        if with_md.exists() {
            return with_md;
        }
    }

    PathBuf::new()
}

/// Quality 规则检查（Q001-Q010）
fn check_quality_rules(page_type: &str, body: &str, file: &str, warnings: &mut Vec<WikiLintIssue>) {
    let has_section = |section: &str| body.contains(&format!("## {}", section));

    match page_type {
        "api" => {
            if !body.contains("Field") || !body.contains("Source") || !body.contains("Required") {
                warnings.push(WikiLintIssue {
                    rule_id: "Q001".to_string(),
                    file: file.to_string(),
                    line: None,
                    message: "API 页面缺少 Field | Source | Required 字段表".to_string(),
                    severity: "warning".to_string(),
                });
            }
            if !has_section("IDL Source") {
                warnings.push(WikiLintIssue {
                    rule_id: "Q002".to_string(),
                    file: file.to_string(),
                    line: None,
                    message: "API 页面缺少 IDL Source section".to_string(),
                    severity: "warning".to_string(),
                });
            }
            if !body.contains("Example Request") && !body.contains("Example Response") {
                warnings.push(WikiLintIssue {
                    rule_id: "Q003".to_string(),
                    file: file.to_string(),
                    line: None,
                    message: "API 页面缺少请求或响应示例".to_string(),
                    severity: "warning".to_string(),
                });
            }
            if !has_section("Errors And Status Codes") {
                warnings.push(WikiLintIssue {
                    rule_id: "Q004".to_string(),
                    file: file.to_string(),
                    line: None,
                    message: "API 页面缺少错误行为说明".to_string(),
                    severity: "warning".to_string(),
                });
            }
        }
        "component" => {
            if file.contains("external-dependencies") {
                if !body.to_lowercase().contains("matrix") && !has_section("Dependency Matrix") {
                    warnings.push(WikiLintIssue {
                        rule_id: "Q005".to_string(),
                        file: file.to_string(),
                        line: None,
                        message: "依赖页面缺少 dependency matrix".to_string(),
                        severity: "warning".to_string(),
                    });
                }
                if !has_section("Failure Impact") {
                    warnings.push(WikiLintIssue {
                        rule_id: "Q005".to_string(),
                        file: file.to_string(),
                        line: None,
                        message: "依赖页面缺少 failure impact".to_string(),
                        severity: "warning".to_string(),
                    });
                }
            }
            if file.contains("config-and-cache") {
                if !body.to_lowercase().contains("default") && !has_section("Config And Defaults") {
                    warnings.push(WikiLintIssue {
                        rule_id: "Q006".to_string(),
                        file: file.to_string(),
                        line: None,
                        message: "配置/缓存页面缺少 defaults".to_string(),
                        severity: "warning".to_string(),
                    });
                }
            }
        }
        "flow" => {
            if file.contains("auth-and-identity") && !has_section("Identity Sources") {
                warnings.push(WikiLintIssue {
                    rule_id: "Q007".to_string(),
                    file: file.to_string(),
                    line: None,
                    message: "auth/identity 页面缺少 identity sources".to_string(),
                    severity: "warning".to_string(),
                });
            }
            for (section, rule) in [
                ("Business Context", "Q008"),
                ("Field Propagation", "Q008"),
                ("Runtime Observability", "Q008"),
                ("Failure Modes", "Q008"),
            ] {
                if !has_section(section) {
                    warnings.push(WikiLintIssue {
                        rule_id: rule.to_string(),
                        file: file.to_string(),
                        line: None,
                        message: format!("flow 页面缺少 {}", section),
                        severity: "warning".to_string(),
                    });
                }
            }
            if file.contains("field-propagation") {
                for (section, rule) in [
                    ("Field Matrix", "Q009"),
                    ("Downstream Request Mapping", "Q009"),
                    ("Config And Defaults", "Q009"),
                    ("Observability", "Q009"),
                ] {
                    if !has_section(section) && !body.contains(&section.replace(" ", "")) {
                        warnings.push(WikiLintIssue {
                            rule_id: rule.to_string(),
                            file: file.to_string(),
                            line: None,
                            message: format!("字段传播页面缺少 {}", section),
                            severity: "warning".to_string(),
                        });
                    }
                }
            }
        }
        "runbook" => {
            for (section, rule) in [
                ("Owners", "Q010"),
                ("Metrics Dashboards", "Q010"),
                ("Alerts", "Q010"),
                ("Common Error Logs", "Q010"),
            ] {
                if !has_section(section) {
                    warnings.push(WikiLintIssue {
                        rule_id: rule.to_string(),
                        file: file.to_string(),
                        line: None,
                        message: format!("runbook 缺少 {}", section),
                        severity: "warning".to_string(),
                    });
                }
            }
        }
        _ => {}
    }
}

/// 从 wiki 目录计算质量等级（lint 通过后调用）
fn compute_quality_level_from_wiki(wiki_root: &Path, project_root: &Path) -> String {
    let pages = collect_wiki_pages(wiki_root).unwrap_or_default();
    let coverage = compute_core_page_coverage(project_root, &pages);

    // L1: index + overview + source-map
    let l1 = coverage.has_index && coverage.has_overview && coverage.has_source_map;
    if !l1 {
        return "N/A".to_string();
    }

    // L2: L1 + 至少 1 component + 1 flow + 1 api
    let l2 = coverage.component_pages >= 1 && coverage.flow_pages >= 1 && coverage.api_pages >= 1;
    if !l2 {
        return "L1".to_string();
    }

    // L3: L2 + 至少 1 runbook + log
    let l3 = coverage.runbook_pages >= 1 && coverage.has_log;
    if !l3 {
        return "L2".to_string();
    }

    "L3".to_string()
}

/// 执行 git diff --name-only 获取变更文件
fn git_changed_files(repo_root: &Path) -> Result<Vec<String>, AppError> {
    let output = std::process::Command::new("git")
        .arg("diff")
        .arg("--name-only")
        .current_dir(repo_root)
        .output()
        .map_err(|e| AppError::Message(format!("git 命令执行失败: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Message(format!(
            "git diff 失败: {}",
            stderr.trim()
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

/// 读取 profile.json 的 changed_files_threshold
fn read_changed_files_threshold(root: &Path) -> u32 {
    let path = root.join(WIKI_STATE_DIR).join(WIKI_PROFILE_FILE);
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return DEFAULT_CHANGED_FILES_THRESHOLD,
    };
    let value: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return DEFAULT_CHANGED_FILES_THRESHOLD,
    };
    value
        .get("changed_files_threshold")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
        .unwrap_or(DEFAULT_CHANGED_FILES_THRESHOLD)
}

/// 简易 glob 匹配（使用 globset，支持 * 和 ** 语义）
fn glob_matches(pattern: &str, candidates: &std::collections::HashSet<&str>) -> bool {
    let matcher = match globset::Glob::new(pattern) {
        Ok(g) => g.compile_matcher(),
        Err(_) => return false,
    };
    candidates.iter().any(|c| matcher.is_match(c))
}

/// wiki 页面的内部表示
#[derive(Debug, Clone)]
struct WikiPageInfo {
    path: String,
    mtime: i64,
    page_type: String,
    status: String,
    source_files: Vec<String>,
    title: String,
}

/// 递归收集 wiki/ 目录下所有 .md 文件的元数据
fn collect_wiki_pages(wiki_root: &Path) -> Result<Vec<WikiPageInfo>, AppError> {
    let mut pages = Vec::new();
    collect_wiki_pages_recursive(wiki_root, wiki_root, &mut pages)?;
    Ok(pages)
}

fn collect_wiki_pages_recursive(
    wiki_root: &Path,
    current: &Path,
    pages: &mut Vec<WikiPageInfo>,
) -> Result<(), AppError> {
    let entries = fs::read_dir(current)
        .map_err(|e| AppError::Message(format!("遍历 wiki 目录失败 {}: {e}", current.display())))?;

    for entry in entries.flatten() {
        let path = entry.path();
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        if metadata.is_dir() {
            let name = path.file_name().map(|n| n.to_string_lossy().to_string());
            if name.as_deref().is_some_and(|n| SKIP_DIRS.contains(&n)) {
                continue;
            }
            collect_wiki_pages_recursive(wiki_root, &path, pages)?;
        } else if metadata.is_file() && path.extension().is_some_and(|ext| ext == "md") {
            let mtime = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

            let rel_path = path
                .strip_prefix(wiki_root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");

            let content = fs::read_to_string(&path).unwrap_or_default();
            let (frontmatter, _) = split_frontmatter(&content);
            let (page_type, status, source_files, title) = parse_frontmatter_fields(&frontmatter);

            pages.push(WikiPageInfo {
                path: rel_path,
                mtime,
                page_type,
                status,
                source_files,
                title,
            });
        }
    }
    Ok(())
}

/// 计算核心页面覆盖率
fn compute_core_page_coverage(root: &Path, pages: &[WikiPageInfo]) -> WikiCorePageCoverage {
    let has_file = |rel: &str| root.join("wiki").join(rel).is_file();

    let component_pages = pages
        .iter()
        .filter(|p| p.page_type == "component" || p.path.starts_with("components/"))
        .count() as u32;
    let flow_pages = pages
        .iter()
        .filter(|p| p.page_type == "flow" || p.path.starts_with("flows/"))
        .count() as u32;
    let api_pages = pages
        .iter()
        .filter(|p| p.page_type == "api" || p.path.starts_with("apis/"))
        .count() as u32;
    let runbook_pages = pages
        .iter()
        .filter(|p| p.page_type == "runbook" || p.path.starts_with("runbooks/"))
        .count() as u32;

    WikiCorePageCoverage {
        has_index: has_file("index.md"),
        has_overview: has_file("overview.md"),
        has_source_map: has_file("source-map.md"),
        has_log: has_file("log.md"),
        has_schema: has_file("SCHEMA.md"),
        component_pages,
        flow_pages,
        api_pages,
        runbook_pages,
    }
}

/// 计算 wiki 目录内容的 SHA-256 hash
fn compute_wiki_content_hash(wiki_root: &Path, pages: &[WikiPageInfo]) -> Result<String, AppError> {
    let mut hasher = Sha256::new();

    // 对路径排序后逐文件更新 hash
    let mut sorted_paths: Vec<&str> = pages.iter().map(|p| p.path.as_str()).collect();
    sorted_paths.sort();

    for rel_path in &sorted_paths {
        let full_path = wiki_root.join(rel_path);
        hasher.update(rel_path.as_bytes());
        hasher.update(b"\0");
        if let Ok(bytes) = fs::read(&full_path) {
            hasher.update(&bytes);
        }
        hasher.update(b"\0");
    }

    let hash = hasher.finalize();
    Ok(format!("{:x}", hash))
}

/// 统计有效 source_files 引用数
fn count_source_refs(root: &Path, pages: &[WikiPageInfo]) -> u32 {
    pages
        .iter()
        .flat_map(|p| &p.source_files)
        .filter(|s| {
            // glob 模式跳过字面检查
            if s.contains('*') {
                return true;
            }
            root.join(s).exists()
        })
        .count() as u32
}

/// 上次 lint 结果的简化表示
struct LastLintInfo {
    passed: bool,
    checked_at: i64,
}

/// 读取 .opensunstar/wiki/lint-result.json
fn read_last_lint_result(root: &Path) -> Option<LastLintInfo> {
    let path = root.join(WIKI_STATE_DIR).join(WIKI_LINT_RESULT_FILE);
    let content = fs::read_to_string(&path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&content).ok()?;
    let summary = value.get("summary")?;
    let passed = summary.get("passed")?.as_bool()?;
    let checked_at = value.get("checked_at")?.as_i64()?;
    Some(LastLintInfo { passed, checked_at })
}

/// 判定 base_status
fn determine_base_status(
    coverage: &WikiCorePageCoverage,
    content_sha256: &str,
    last_lint: &Option<LastLintInfo>,
) -> String {
    // 如果 lint 有 error，则为 invalid
    if let Some(lint) = last_lint {
        if !lint.passed {
            return "invalid".to_string();
        }
    }

    // 核心页面不全则为 scaffolded
    let is_scaffolded = !coverage.has_overview || !coverage.has_source_map;
    if is_scaffolded {
        return "scaffolded".to_string();
    }

    // 比较与上次 lint 时的 content_sha256
    // 若不一致则为 drifted，否则 effective
    // P1 简化：暂无法对比历史 hash，先返回 effective
    // Phase 2 lint 实现后，lint-result.json 会保存 content_sha256，届时可精确对比
    let _ = content_sha256; // 后续 Phase 2 使用
    "effective".to_string()
}

/// 判定 quality_level（P1 简化版，Phase 2 由 lint 精确判定）
fn determine_quality_level(coverage: &WikiCorePageCoverage, base_status: &str) -> String {
    if base_status == "missing" || base_status == "scaffolded" {
        return "N/A".to_string();
    }

    // L1: index + overview + source-map 就绪
    let l1 = coverage.has_index && coverage.has_overview && coverage.has_source_map;
    if !l1 {
        return "N/A".to_string();
    }

    // L2: L1 + 至少 1 个 component + 1 个 flow + 1 个 api
    let l2 = coverage.component_pages >= 1 && coverage.flow_pages >= 1 && coverage.api_pages >= 1;
    if !l2 {
        return "L1".to_string();
    }

    // L3: L2 + 至少 1 个 runbook + log
    let l3 = coverage.runbook_pages >= 1 && coverage.has_log;
    if !l3 {
        return "L2".to_string();
    }

    "L3".to_string()
}

/// 分割 frontmatter 和 body
fn split_frontmatter(content: &str) -> (Option<String>, String) {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return (None, content.to_string());
    }

    let after_first_delim = &trimmed[3..];
    // 找第二个 ---
    if let Some(end) = after_first_delim.find("\n---") {
        let frontmatter = after_first_delim[..end].trim().to_string();
        let body_start = end + 4; // \n---
        let body = after_first_delim[body_start..]
            .trim_start_matches('\n')
            .to_string();
        return (Some(frontmatter), body);
    }

    (None, content.to_string())
}

/// 从 frontmatter 文本中解析关键字段
fn parse_frontmatter_fields(frontmatter: &Option<String>) -> (String, String, Vec<String>, String) {
    let empty = (String::new(), String::new(), Vec::new(), String::new());
    let fm = match frontmatter {
        Some(f) => f,
        None => return empty,
    };

    let yaml: serde_yaml::Value = match serde_yaml::from_str(fm) {
        Ok(v) => v,
        Err(_) => return empty,
    };

    let map = match yaml {
        serde_yaml::Value::Mapping(m) => m,
        _ => return empty,
    };

    let page_type = map
        .get(&serde_yaml::Value::String("type".into()))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let status = map
        .get(&serde_yaml::Value::String("status".into()))
        .and_then(|v| v.as_str())
        .unwrap_or("draft")
        .to_string();

    let title = map
        .get(&serde_yaml::Value::String("title".into()))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let source_files = map
        .get(&serde_yaml::Value::String("source_files".into()))
        .and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    (page_type, status, source_files, title)
}

/// 写入状态文件到 .opensunstar/wiki/
fn write_state_file<T: Serialize>(root: &Path, filename: &str, data: &T) -> Result<(), AppError> {
    let state_dir = root.join(WIKI_STATE_DIR);
    fs::create_dir_all(&state_dir)
        .map_err(|e| AppError::Message(format!("创建状态目录失败: {e}")))?;

    let path = state_dir.join(filename);
    let json = serde_json::to_string_pretty(data)
        .map_err(|e| AppError::Message(format!("序列化状态文件失败: {e}")))?;

    fs::write(&path, json)
        .map_err(|e| AppError::Message(format!("写入状态文件失败 {}: {e}", path.display())))?;

    Ok(())
}

// ── 单元测试 ──────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_scan_missing_wiki() {
        let tmp = TempDir::new().unwrap();
        let result = scan_project_wiki(tmp.path().to_str().unwrap(), "test-project").unwrap();
        assert!(!result.exists);
        assert_eq!(result.base_status, "missing");
        assert_eq!(result.quality_level, "N/A");
    }

    #[test]
    fn test_scan_scaffolded_wiki() {
        let tmp = TempDir::new().unwrap();
        let wiki_dir = tmp.path().join("wiki");
        fs::create_dir_all(&wiki_dir).unwrap();
        fs::write(
            wiki_dir.join("index.md"),
            "---\ntype: overview\nstatus: draft\n---\n# Test",
        )
        .unwrap();

        let result = scan_project_wiki(tmp.path().to_str().unwrap(), "test-project").unwrap();
        assert!(result.exists);
        assert_eq!(result.base_status, "scaffolded");
        assert!(result.core_page_coverage.has_index);
        assert!(!result.core_page_coverage.has_overview);
    }

    #[test]
    fn test_split_frontmatter() {
        let content = "---\ntitle: Test\ntype: overview\n---\n# Body";
        let (fm, body) = split_frontmatter(content);
        assert!(fm.is_some());
        assert!(body.contains("# Body"));
    }

    #[test]
    fn test_split_frontmatter_no_frontmatter() {
        let content = "# Just body";
        let (fm, body) = split_frontmatter(content);
        assert!(fm.is_none());
        assert_eq!(body, "# Just body");
    }

    #[test]
    fn test_parse_frontmatter_fields() {
        let fm = Some(
            "title: Test Page\ntype: component\nstatus: active\nsource_files:\n  - src/main.rs"
                .to_string(),
        );
        let (page_type, status, source_files, title) = parse_frontmatter_fields(&fm);
        assert_eq!(page_type, "component");
        assert_eq!(status, "active");
        assert_eq!(title, "Test Page");
        assert_eq!(source_files, vec!["src/main.rs"]);
    }
}
