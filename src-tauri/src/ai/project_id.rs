//! 看板 AI 统一的 project_id 解析
//!
//! 规范：
//! - `proj_*` — SQLite `projects.id`（看板 canonical id）
//! - `__portfolio__` — 组合级 AI 操作（周报、NL 查询成本汇总）
//! - `path_*` — 遗留 id，迁移后应全部合并为 `proj_*`

use crate::database::Database;

/// 组合级 / 非项目 AI 操作的统一 project_id
pub const PORTFOLIO_PROJECT_ID: &str = "__portfolio__";

/// 根据项目路径计算遗留 `path_*` id（仅用于迁移合并）
pub fn path_legacy_id(project_path: &str) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(project_path.as_bytes());
    let hex: String = hash[..8].iter().map(|b| format!("{b:02x}")).collect();
    format!("path_{hex}")
}

/// 解析为 canonical project_id。优先使用 `proj_*`；否则按路径查 SQLite projects 表。
pub fn resolve_canonical_project_id(
    db: &Database,
    project_id: &str,
    project_path: Option<&str>,
) -> String {
    if project_id == PORTFOLIO_PROJECT_ID || project_id.starts_with("proj_") {
        return project_id.to_string();
    }

    if let Some(path) = project_path {
        if let Ok(Some(id)) = db.get_project_id_by_path(path) {
            return id;
        }
    }

    project_id.to_string()
}
