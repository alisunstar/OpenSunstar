//! 项目 × 资产关联（SSOT：`project_asset_links` 统一 8 类资产）

use crate::database::{lock_conn, Database};
use crate::error::AppError;
use rusqlite::params;
use serde::{Deserialize, Serialize};

pub const ASSET_MCP: &str = "mcp";
pub const ASSET_SKILL: &str = "skill";
pub const ASSET_PROMPT: &str = "prompt";
pub const ASSET_COMMAND: &str = "command";
pub const ASSET_HOOK: &str = "hook";
pub const ASSET_IGNORE: &str = "ignore";
pub const ASSET_PERMISSION: &str = "permission";
pub const ASSET_SUBAGENT: &str = "subagent";

/// 全部 8 类项目资产类型
pub const PROJECT_ASSET_TYPES: &[&str] = &[
    ASSET_MCP,
    ASSET_SKILL,
    ASSET_PROMPT,
    ASSET_COMMAND,
    ASSET_HOOK,
    ASSET_IGNORE,
    ASSET_PERMISSION,
    ASSET_SUBAGENT,
];

/// 扩展 5 类（历史命名，与 `PROJECT_ASSET_TYPES` 后五项一致）
pub const EXTENDED_ASSET_TYPES: &[&str] = &[
    ASSET_COMMAND,
    ASSET_HOOK,
    ASSET_IGNORE,
    ASSET_PERMISSION,
    ASSET_SUBAGENT,
];

/// 项目资产关联行
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectAssetLink {
    pub project_id: String,
    pub asset_type: String,
    pub asset_id: String,
    pub asset_app_type: String,
    pub enabled: bool,
    pub scope: String,
    pub source: String,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 8 类资产启用计数（矩阵 / readiness 聚合）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectAllAssetCounts {
    pub mcp: u32,
    pub skills: u32,
    pub prompts: u32,
    pub commands: u32,
    pub hooks: u32,
    pub ignore: u32,
    pub permissions: u32,
    pub subagents: u32,
}

pub(crate) fn validate_project_asset_type(asset_type: &str) -> Result<(), AppError> {
    if PROJECT_ASSET_TYPES.contains(&asset_type) {
        Ok(())
    } else {
        Err(AppError::InvalidInput(format!(
            "不支持的 asset_type: {asset_type}（支持: mcp/skill/prompt/command/hook/ignore/permission/subagent）"
        )))
    }
}

fn now_ts() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

impl Database {
    pub fn get_project_asset_links(
        &self,
        project_id: &str,
        asset_type: Option<&str>,
    ) -> Result<Vec<ProjectAssetLink>, AppError> {
        if let Some(t) = asset_type {
            validate_project_asset_type(t)?;
        }

        let conn = lock_conn!(self.conn);
        let map_row = |row: &rusqlite::Row<'_>| {
            Ok(ProjectAssetLink {
                project_id: row.get(0)?,
                asset_type: row.get(1)?,
                asset_id: row.get(2)?,
                asset_app_type: row.get(3)?,
                enabled: row.get::<_, i64>(4)? != 0,
                scope: row.get(5)?,
                source: row.get(6)?,
                created_at: row.get(7)?,
                updated_at: row.get(8)?,
            })
        };

        if let Some(t) = asset_type {
            let mut stmt = conn
                .prepare(
                    "SELECT project_id, asset_type, asset_id, asset_app_type, enabled, scope, source, created_at, updated_at
                     FROM project_asset_links WHERE project_id = ?1 AND asset_type = ?2
                     ORDER BY created_at ASC",
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            let rows = stmt
                .query_map(params![project_id, t], map_row)
                .map_err(|e| AppError::Database(e.to_string()))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| AppError::Database(e.to_string()))
        } else {
            let mut stmt = conn
                .prepare(
                    "SELECT project_id, asset_type, asset_id, asset_app_type, enabled, scope, source, created_at, updated_at
                     FROM project_asset_links WHERE project_id = ?1
                     ORDER BY asset_type ASC, created_at ASC",
                )
                .map_err(|e| AppError::Database(e.to_string()))?;
            let rows = stmt
                .query_map(params![project_id], map_row)
                .map_err(|e| AppError::Database(e.to_string()))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| AppError::Database(e.to_string()))
        }
    }

    pub fn link_project_asset(
        &self,
        project_id: &str,
        asset_type: &str,
        asset_id: &str,
        asset_app_type: &str,
        enabled: bool,
    ) -> Result<(), AppError> {
        validate_project_asset_type(asset_type)?;
        let now = now_ts();
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT INTO project_asset_links
             (project_id, asset_type, asset_id, asset_app_type, enabled, scope, source, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'project', 'manual', ?6, ?6)
             ON CONFLICT(project_id, asset_type, asset_id, asset_app_type)
             DO UPDATE SET enabled = excluded.enabled, updated_at = excluded.updated_at",
            params![
                project_id,
                asset_type,
                asset_id,
                asset_app_type,
                enabled as i64,
                now,
            ],
        )
        .map_err(|e| AppError::Database(format!("link project_asset 失败: {e}")))?;
        Ok(())
    }

    pub fn unlink_project_asset(
        &self,
        project_id: &str,
        asset_type: &str,
        asset_id: &str,
        asset_app_type: &str,
    ) -> Result<bool, AppError> {
        validate_project_asset_type(asset_type)?;
        let conn = lock_conn!(self.conn);
        let affected = conn
            .execute(
                "DELETE FROM project_asset_links
                 WHERE project_id = ?1 AND asset_type = ?2 AND asset_id = ?3 AND asset_app_type = ?4",
                params![project_id, asset_type, asset_id, asset_app_type],
            )
            .map_err(|e| AppError::Database(format!("unlink project_asset 失败: {e}")))?;
        Ok(affected > 0)
    }

    pub fn set_project_assets(
        &self,
        project_id: &str,
        asset_type: &str,
        asset_ids: &[String],
    ) -> Result<(), AppError> {
        validate_project_asset_type(asset_type)?;
        if asset_type == ASSET_PROMPT {
            return Err(AppError::InvalidInput(
                "prompt 批量设置请使用 set_project_prompts（需 prompt_app_type）；不可通过 set_project_assets 写入"
                    .into(),
            ));
        }
        let conn = lock_conn!(self.conn);
        let now = now_ts();

        conn.execute(
            "DELETE FROM project_asset_links WHERE project_id = ?1 AND asset_type = ?2",
            params![project_id, asset_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        for id in asset_ids {
            conn.execute(
                "INSERT INTO project_asset_links
                 (project_id, asset_type, asset_id, asset_app_type, enabled, scope, source, created_at, updated_at)
                 VALUES (?1, ?2, ?3, '', 1, 'project', 'manual', ?4, ?4)",
                params![project_id, asset_type, id, now],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        }
        Ok(())
    }

    pub fn count_enabled_project_assets(
        &self,
        project_id: &str,
        asset_type: &str,
    ) -> Result<u32, AppError> {
        validate_project_asset_type(asset_type)?;
        let conn = lock_conn!(self.conn);
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM project_asset_links
                 WHERE project_id = ?1 AND asset_type = ?2 AND enabled = 1",
                params![project_id, asset_type],
                |row| row.get(0),
            )
            .unwrap_or(0);
        Ok(count as u32)
    }

    pub fn max_project_asset_links_updated_at(
        &self,
        project_id: &str,
    ) -> Result<Option<i64>, AppError> {
        let conn = lock_conn!(self.conn);
        let ts: Option<i64> = conn
            .query_row(
                "SELECT MAX(updated_at) FROM project_asset_links WHERE project_id = ?1",
                params![project_id],
                |row| row.get(0),
            )
            .unwrap_or(None);
        Ok(ts)
    }

    /// 聚合 8 类资产计数（统一读 `project_asset_links`）
    pub fn get_project_all_asset_counts(
        &self,
        project_id: &str,
    ) -> Result<ProjectAllAssetCounts, AppError> {
        Ok(ProjectAllAssetCounts {
            mcp: self
                .count_enabled_project_assets(project_id, ASSET_MCP)
                .unwrap_or(0),
            skills: self
                .count_enabled_project_assets(project_id, ASSET_SKILL)
                .unwrap_or(0),
            prompts: self
                .count_enabled_project_assets(project_id, ASSET_PROMPT)
                .unwrap_or(0),
            commands: self
                .count_enabled_project_assets(project_id, ASSET_COMMAND)
                .unwrap_or(0),
            hooks: self
                .count_enabled_project_assets(project_id, ASSET_HOOK)
                .unwrap_or(0),
            ignore: self
                .count_enabled_project_assets(project_id, ASSET_IGNORE)
                .unwrap_or(0),
            permissions: self
                .count_enabled_project_assets(project_id, ASSET_PERMISSION)
                .unwrap_or(0),
            subagents: self
                .count_enabled_project_assets(project_id, ASSET_SUBAGENT)
                .unwrap_or(0),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::dao::projects::Project;
    use crate::database::Database;

    fn test_db() -> Database {
        Database::memory().expect("memory db")
    }

    fn seed_project(db: &Database, id: &str) {
        let now = 1_700_000_000_i64;
        db.upsert_project(&Project {
            id: id.into(),
            name: "test".into(),
            path: format!("/tmp/{id}"),
            git_remote_url: None,
            created_at: now,
            updated_at: now,
            target_app: None,
            blueprint_id: None,
            stage: "mvp".into(),
            mvp_progress: None,
        })
        .unwrap();
    }

    #[test]
    fn legacy_mcp_link_api_writes_unified_table() {
        let db = test_db();
        seed_project(&db, "p1");
        db.link_project_mcp_server("p1", "mcp-a", true).unwrap();
        let links = db.get_project_asset_links("p1", Some(ASSET_MCP)).unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].asset_id, "mcp-a");
        assert!(links[0].enabled);
    }

    #[test]
    fn set_project_assets_rejects_prompt_type() {
        let db = test_db();
        seed_project(&db, "p1");
        let err = db
            .set_project_assets("p1", ASSET_PROMPT, &["pr1".into()])
            .expect_err("prompt must use set_project_prompts");
        assert!(err.to_string().contains("set_project_prompts"));
    }

    #[test]
    fn legacy_prompt_link_api_writes_unified_table() {
        let db = test_db();
        seed_project(&db, "p1");
        db.link_project_prompt("p1", "pr1", "claude", true).unwrap();
        let links = db
            .get_project_asset_links("p1", Some(ASSET_PROMPT))
            .unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].asset_id, "pr1");
        assert_eq!(links[0].asset_app_type, "claude");
    }
}
