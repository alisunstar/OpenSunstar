//! 审计引擎核心类型和扫描入口
//!
//! 负责：加载规则 → 遍历目录文件 → 调度分析器 → 评分 → 决定阻断

use crate::AppError;
use serde::{Deserialize, Serialize};
use std::path::Path;

use super::analyzers::{static_analyzer, unicode};
use super::rules::RuleSet;

// ── 严重级别 ────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info = 1,
    Low = 5,
    Medium = 10,
    High = 20,
    Critical = 50,
}

impl Severity {
    pub fn label(self) -> &'static str {
        match self {
            Severity::Info => "INFO",
            Severity::Low => "LOW",
            Severity::Medium => "MEDIUM",
            Severity::High => "HIGH",
            Severity::Critical => "CRITICAL",
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

impl std::str::FromStr for Severity {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "INFO" => Ok(Severity::Info),
            "LOW" => Ok(Severity::Low),
            "MEDIUM" => Ok(Severity::Medium),
            "HIGH" => Ok(Severity::High),
            "CRITICAL" => Ok(Severity::Critical),
            other => Err(format!("Unknown severity: {other}")),
        }
    }
}

// ── 阻断阈值 ────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BlockThreshold {
    /// 仅阻断 CRITICAL（默认）
    Critical,
    /// 阻断 HIGH 及以上
    High,
    /// 阻断 MEDIUM 及以上
    Medium,
    /// 永远不阻断（仅报告）
    Never,
}

impl Default for BlockThreshold {
    fn default() -> Self {
        Self::Critical
    }
}

impl BlockThreshold {
    pub fn should_block(&self, severity: Severity) -> bool {
        match self {
            Self::Critical => severity >= Severity::Critical,
            Self::High => severity >= Severity::High,
            Self::Medium => severity >= Severity::Medium,
            Self::Never => false,
        }
    }
}

// ── 单条命中记录 ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// 规则 ID
    pub rule_id: String,
    /// 严重级别
    pub severity: Severity,
    /// 检测类别 (prompt-injection / credential-access / destructive-commands / ...)
    pub category: String,
    /// 命中文件路径（相对于扫描目录）
    pub file: String,
    /// 行号
    pub line: usize,
    /// 命中内容摘要（截断到 120 字符）
    pub snippet: String,
    /// 人类可读消息
    pub message: String,
}

// ── 审计来源上下文 ──────────────────────────────────────

#[derive(Debug, Clone)]
pub enum AuditSource {
    Install {
        owner: String,
        repo: String,
    },
    Update {
        owner: String,
        repo: String,
        skill_name: String,
    },
    DesignContractInstall {
        contract_name: String,
    },
    RecipeInstall {
        recipe_name: String,
        change_id: String,
    },
}

// ── 审计上下文 ──────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct AuditContext {
    pub source: AuditSource,
    pub threshold: BlockThreshold,
}

// ── 扫描结果 ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResult {
    /// 扫描目录
    pub scanned_dir: String,
    /// 扫描文件数
    pub files_scanned: usize,
    /// 命中列表
    pub findings: Vec<Finding>,
    /// 命中摘要（按严重级别统计）
    pub summary: SeveritySummary,
    /// 是否应阻断
    pub blocked: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SeveritySummary {
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub info: usize,
}

impl AuditResult {
    pub fn should_block(&self) -> bool {
        self.blocked
    }

    pub fn total_findings(&self) -> usize {
        self.findings.len()
    }
}

// ── 目录扫描入口 ────────────────────────────────────────

/// 扫描指定目录下的所有文件，返回审计结果
pub fn scan_dir(dir: &Path, ctx: &AuditContext) -> Result<AuditResult, AppError> {
    let mut findings: Vec<Finding> = Vec::new();
    let mut files_scanned = 0usize;

    let rule_set = RuleSet::default();
    let max_file_size: u64 = 2 * 1024 * 1024; // 跳过 >2MB 文件

    scan_dir_recursive(
        dir,
        dir,
        &rule_set,
        ctx,
        &mut findings,
        &mut files_scanned,
        max_file_size,
    )?;

    // 汇总
    let mut summary = SeveritySummary::default();
    for f in &findings {
        match f.severity {
            Severity::Critical => summary.critical += 1,
            Severity::High => summary.high += 1,
            Severity::Medium => summary.medium += 1,
            Severity::Low => summary.low += 1,
            Severity::Info => summary.info += 1,
        }
    }

    let blocked = findings
        .iter()
        .any(|f| ctx.threshold.should_block(f.severity));

    Ok(AuditResult {
        scanned_dir: dir.display().to_string(),
        files_scanned,
        findings,
        summary,
        blocked,
    })
}

fn should_skip_file(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    // 跳过二进制/大型文件
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let skip_exts = [
        "png", "jpg", "jpeg", "gif", "bmp", "ico", "svg", "webp", "woff", "woff2", "ttf", "eot",
        "otf", "mp3", "mp4", "wav", "ogg", "avi", "mov", "zip", "tar", "gz", "bz2", "xz", "7z",
        "rar", "exe", "dll", "so", "dylib", "wasm", "bin", "dat", "db", "sqlite", "sqlite3", "pdf",
        "doc", "docx", "xls", "xlsx", "class", "jar", "war",
    ];
    if skip_exts.contains(&ext) {
        return true;
    }
    // 跳过隐藏文件和常见非代码文件
    if name.starts_with('.') && name != ".gitignore" && name != ".env.example" {
        return true;
    }
    // 跳过 node_modules / .git 目录已在递归中处理
    false
}

fn scan_dir_recursive(
    base_dir: &Path,
    current_dir: &Path,
    rule_set: &RuleSet,
    ctx: &AuditContext,
    findings: &mut Vec<Finding>,
    files_scanned: &mut usize,
    max_file_size: u64,
) -> Result<(), AppError> {
    let entries = match std::fs::read_dir(current_dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let ft = match entry.file_type() {
            Ok(ft) => ft,
            Err(_) => continue,
        };

        if ft.is_dir() {
            let dir_name = entry.file_name();
            let dir_name_str = dir_name.to_string_lossy();
            // 跳过常见的非代码目录
            if dir_name_str == ".git"
                || dir_name_str == "node_modules"
                || dir_name_str == "__pycache__"
                || dir_name_str == ".venv"
                || dir_name_str == "venv"
                || dir_name_str == "target"
                || dir_name_str == "dist"
                || dir_name_str == "build"
            {
                continue;
            }
            scan_dir_recursive(
                base_dir,
                &path,
                rule_set,
                ctx,
                findings,
                files_scanned,
                max_file_size,
            )?;
        } else if ft.is_file() {
            if should_skip_file(&path) {
                continue;
            }

            let metadata = match std::fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };
            if metadata.len() > max_file_size {
                continue;
            }

            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue, // 跳过无法读取为 UTF-8 的文件（可能是二进制）
            };

            *files_scanned += 1;

            // 执行分析器
            let rel_path = path
                .strip_prefix(base_dir)
                .unwrap_or(&path)
                .display()
                .to_string();

            static_analyzer::scan(&rel_path, &content, rule_set, findings);
            unicode::scan(&rel_path, &content, findings);
        }
    }

    Ok(())
}
