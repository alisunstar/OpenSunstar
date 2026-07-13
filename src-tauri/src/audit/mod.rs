//! SkillShare 审计规则复用模块
//!
//! 在 Skill 安装/更新前执行安全扫描，基于 SkillShare 的 100+ 规则体系。
//! 采用分阶段渐进式集成：Phase 1 落地 static 分析器 + CRITICAL/HIGH 规则。

pub mod analyzers;
pub mod engine;
pub mod report;
pub mod rules;

pub use engine::{scan_dir, AuditContext, AuditSource};
pub use report::AuditReport;
