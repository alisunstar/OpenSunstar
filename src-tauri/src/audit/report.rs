//! 审计报告生成 (Text / JSON)
//!
//! SARIF 2.1.0 格式将在 Phase 2+ 实现。

use serde::Serialize;

use super::engine::{AuditResult, SeveritySummary};

/// Text 报告
pub fn to_text(result: &AuditResult) -> String {
    let mut buf = String::new();

    buf.push_str("═══════════════════════════════════════════════════\n");
    buf.push_str("  SkillShare 安全审计报告\n");
    buf.push_str("═══════════════════════════════════════════════════\n\n");

    buf.push_str(&format!("扫描目录: {}\n", result.scanned_dir));
    buf.push_str(&format!("扫描文件: {} 个\n", result.files_scanned));
    buf.push_str(&format!("发现问题: {} 项\n\n", result.total_findings()));

    buf.push_str("── 按严重级别统计 ──\n");
    buf.push_str(&format!(
        "  CRITICAL: {:>4}   HIGH: {:>4}   MEDIUM: {:>4}   LOW: {:>4}   INFO: {:>4}\n\n",
        result.summary.critical,
        result.summary.high,
        result.summary.medium,
        result.summary.low,
        result.summary.info
    ));

    buf.push_str(&format!(
        "阻断状态: {}\n\n",
        if result.blocked {
            "⛔ 已阻断（存在达到阈值的严重问题）"
        } else {
            "✅ 通过（未达到阻断阈值）"
        }
    ));

    if !result.findings.is_empty() {
        buf.push_str("── 详细发现 ──\n\n");
        for f in &result.findings {
            buf.push_str(&format!(
                "[{}] {}  →  {}:{}\n",
                f.severity.label(),
                f.rule_id,
                f.file,
                f.line
            ));
            buf.push_str(&format!("  类别: {}\n", f.category));
            buf.push_str(&format!("  消息: {}\n", f.message));
            if !f.snippet.is_empty() {
                buf.push_str(&format!("  摘要: {}\n", f.snippet));
            }
            buf.push('\n');
        }
    }

    buf.push_str("═══════════════════════════════════════════════════\n");
    buf
}

// ── JSON 报告 ──────────────────────────────────────────

#[derive(Serialize)]
pub struct AuditReport {
    pub audit_version: &'static str,
    pub scanned_dir: String,
    pub files_scanned: usize,
    pub total_findings: usize,
    pub summary: SeveritySummary,
    pub blocked: bool,
    pub findings: Vec<FindingEntry>,
}

#[derive(Serialize)]
pub struct FindingEntry {
    pub rule_id: String,
    pub severity: String,
    pub category: String,
    pub file: String,
    pub line: usize,
    pub snippet: String,
    pub message: String,
}

impl From<&AuditResult> for AuditReport {
    fn from(result: &AuditResult) -> Self {
        AuditReport {
            audit_version: "1.0.0",
            scanned_dir: result.scanned_dir.clone(),
            files_scanned: result.files_scanned,
            total_findings: result.total_findings(),
            summary: result.summary.clone(),
            blocked: result.blocked,
            findings: result
                .findings
                .iter()
                .map(|f| FindingEntry {
                    rule_id: f.rule_id.clone(),
                    severity: f.severity.label().to_string(),
                    category: f.category.clone(),
                    file: f.file.clone(),
                    line: f.line,
                    snippet: f.snippet.clone(),
                    message: f.message.clone(),
                })
                .collect(),
        }
    }
}

pub fn to_json(result: &AuditResult) -> Result<String, serde_json::Error> {
    let report = AuditReport::from(result);
    serde_json::to_string_pretty(&report)
}
