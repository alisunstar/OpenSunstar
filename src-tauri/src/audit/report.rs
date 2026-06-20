//! 审计报告 JSON 序列化结构。
//!
//! SARIF 2.1.0 格式将在 Phase 2+ 实现。

use serde::Serialize;

use super::engine::{AuditResult, SeveritySummary};

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
