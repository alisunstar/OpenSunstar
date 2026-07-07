//! 项目级配置隔离 - 数据访问对象
//!
//! Projects 及 MCP/Skills/Prompts 项目关联均读写统一表 `project_asset_links`（v25+）。

use crate::database::{lock_conn, Database};
use crate::error::AppError;
use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::project_assets::{ASSET_MCP, ASSET_PROMPT, ASSET_SKILL};

/// 项目信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    pub git_remote_url: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    /// 项目级目标 CLI（S2-41）；空则沿用看板组合默认
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_app: Option<String>,
    /// 最近应用的 Blueprint id（S2-11）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blueprint_id: Option<String>,
    /// 看板阶段：mvp | rapid | stable
    #[serde(default = "default_project_stage")]
    pub stage: String,
    /// MVP 进度 0–100；None 表示未设置
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mvp_progress: Option<i32>,
}

fn default_project_stage() -> String {
    "mvp".to_string()
}

fn map_project_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Project> {
    Ok(Project {
        id: row.get(0)?,
        name: row.get(1)?,
        path: row.get(2)?,
        git_remote_url: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
        target_app: row.get(6)?,
        blueprint_id: row.get(7)?,
        stage: row.get::<_, String>(8).unwrap_or_else(|_| "mvp".to_string()),
        mvp_progress: row.get(9)?,
    })
}

const PROJECT_SELECT: &str =
    "SELECT id, name, path, git_remote_url, created_at, updated_at, target_app, blueprint_id, stage, mvp_progress";

/// 项目关联的配置项（通用中间表行）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfigLink {
    pub project_id: String,
    pub config_id: String,
    pub enabled: bool,
    pub created_at: i64,
}

/// 项目关联的 Prompt 配置项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectPromptLink {
    pub project_id: String,
    pub prompt_id: String,
    pub prompt_app_type: String,
    pub enabled: bool,
    pub created_at: i64,
}

impl Database {
    // ========== Projects CRUD ==========

    /// 获取所有项目
    pub fn get_all_projects(&self) -> Result<Vec<Project>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(&format!(
                "{PROJECT_SELECT} FROM projects ORDER BY updated_at DESC"
            ))
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], map_project_row)
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut projects = Vec::new();
        for row in rows {
            projects.push(row.map_err(|e| AppError::Database(e.to_string()))?);
        }
        Ok(projects)
    }

    /// 根据 ID 获取项目
    pub fn get_project(&self, id: &str) -> Result<Option<Project>, AppError> {
        let conn = lock_conn!(self.conn);
        let result = conn.query_row(
            &format!("{PROJECT_SELECT} FROM projects WHERE id = ?1"),
            params![id],
            map_project_row,
        );

        match result {
            Ok(project) => Ok(Some(project)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    /// 根据路径获取项目
    pub fn get_project_by_path(&self, path: &str) -> Result<Option<Project>, AppError> {
        let conn = lock_conn!(self.conn);
        let result = conn.query_row(
            &format!("{PROJECT_SELECT} FROM projects WHERE path = ?1"),
            params![path],
            map_project_row,
        );

        match result {
            Ok(project) => Ok(Some(project)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    /// 创建或更新项目
    pub fn upsert_project(&self, project: &Project) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT INTO projects (id, name, path, git_remote_url, created_at, updated_at, target_app, blueprint_id, stage, mvp_progress)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                path = excluded.path,
                git_remote_url = excluded.git_remote_url,
                updated_at = excluded.updated_at,
                target_app = excluded.target_app,
                blueprint_id = excluded.blueprint_id",
            params![
                project.id,
                project.name,
                project.path,
                project.git_remote_url,
                project.created_at,
                project.updated_at,
                project.target_app,
                project.blueprint_id,
                project.stage,
                project.mvp_progress,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// 删除项目（级联删除关联中间表记录）
    pub fn delete_project(&self, id: &str) -> Result<bool, AppError> {
        let conn = lock_conn!(self.conn);
        let affected = conn
            .execute("DELETE FROM projects WHERE id = ?1", params![id])
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(affected > 0)
    }

    pub fn set_project_target_app(
        &self,
        project_id: &str,
        target_app: Option<&str>,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        conn.execute(
            "UPDATE projects SET target_app = ?2, updated_at = ?3 WHERE id = ?1",
            params![project_id, target_app, now],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn set_project_blueprint_id(
        &self,
        project_id: &str,
        blueprint_id: Option<&str>,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        conn.execute(
            "UPDATE projects SET blueprint_id = ?2, updated_at = ?3 WHERE id = ?1",
            params![project_id, blueprint_id, now],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// 更新看板阶段与 MVP 进度
    pub fn update_project_board_metadata(
        &self,
        project_id: &str,
        stage: &str,
        mvp_progress: Option<i32>,
    ) -> Result<(), AppError> {
        let normalized_stage = match stage {
            "mvp" | "rapid" | "stable" => stage,
            other => {
                return Err(AppError::InvalidInput(format!(
                    "无效的项目阶段: {other}（允许: mvp, rapid, stable）"
                )));
            }
        };

        if let Some(progress) = mvp_progress {
            if !(0..=100).contains(&progress) {
                return Err(AppError::InvalidInput(format!(
                    "MVP 进度必须在 0–100 之间，收到: {progress}"
                )));
            }
        }

        let conn = lock_conn!(self.conn);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let affected = conn
            .execute(
                "UPDATE projects SET stage = ?2, mvp_progress = ?3, updated_at = ?4 WHERE id = ?1",
                params![project_id, normalized_stage, mvp_progress, now],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        if affected == 0 {
            return Err(AppError::InvalidInput(format!("项目不存在: {project_id}")));
        }
        Ok(())
    }

    // ========== Project × MCP / Skills / Prompts（SSOT: project_asset_links）==========

    /// 获取项目关联的 MCP 服务器 ID 列表
    pub fn get_project_mcp_servers(&self, project_id: &str) -> Result<Vec<ProjectConfigLink>, AppError> {
        let links = self.get_project_asset_links(project_id, Some(ASSET_MCP))?;
        Ok(links
            .into_iter()
            .map(|l| ProjectConfigLink {
                project_id: l.project_id,
                config_id: l.asset_id,
                enabled: l.enabled,
                created_at: l.created_at,
            })
            .collect())
    }

    pub fn link_project_mcp_server(
        &self,
        project_id: &str,
        mcp_server_id: &str,
        enabled: bool,
    ) -> Result<(), AppError> {
        self.link_project_asset(project_id, ASSET_MCP, mcp_server_id, "", enabled)
    }

    pub fn unlink_project_mcp_server(
        &self,
        project_id: &str,
        mcp_server_id: &str,
    ) -> Result<bool, AppError> {
        self.unlink_project_asset(project_id, ASSET_MCP, mcp_server_id, "")
    }

    pub fn set_project_mcp_servers(
        &self,
        project_id: &str,
        mcp_server_ids: &[String],
    ) -> Result<(), AppError> {
        self.set_project_assets(project_id, ASSET_MCP, mcp_server_ids)
    }

    pub fn get_project_skills(&self, project_id: &str) -> Result<Vec<ProjectConfigLink>, AppError> {
        let links = self.get_project_asset_links(project_id, Some(ASSET_SKILL))?;
        Ok(links
            .into_iter()
            .map(|l| ProjectConfigLink {
                project_id: l.project_id,
                config_id: l.asset_id,
                enabled: l.enabled,
                created_at: l.created_at,
            })
            .collect())
    }

    pub fn link_project_skill(
        &self,
        project_id: &str,
        skill_id: &str,
        enabled: bool,
    ) -> Result<(), AppError> {
        self.link_project_asset(project_id, ASSET_SKILL, skill_id, "", enabled)
    }

    pub fn unlink_project_skill(
        &self,
        project_id: &str,
        skill_id: &str,
    ) -> Result<bool, AppError> {
        self.unlink_project_asset(project_id, ASSET_SKILL, skill_id, "")
    }

    pub fn set_project_skills(
        &self,
        project_id: &str,
        skill_ids: &[String],
    ) -> Result<(), AppError> {
        self.set_project_assets(project_id, ASSET_SKILL, skill_ids)
    }

    pub fn get_project_prompts(&self, project_id: &str) -> Result<Vec<ProjectPromptLink>, AppError> {
        let links = self.get_project_asset_links(project_id, Some(ASSET_PROMPT))?;
        Ok(links
            .into_iter()
            .map(|l| ProjectPromptLink {
                project_id: l.project_id,
                prompt_id: l.asset_id,
                prompt_app_type: l.asset_app_type,
                enabled: l.enabled,
                created_at: l.created_at,
            })
            .collect())
    }

    pub fn link_project_prompt(
        &self,
        project_id: &str,
        prompt_id: &str,
        prompt_app_type: &str,
        enabled: bool,
    ) -> Result<(), AppError> {
        self.link_project_asset(project_id, ASSET_PROMPT, prompt_id, prompt_app_type, enabled)
    }

    pub fn unlink_project_prompt(
        &self,
        project_id: &str,
        prompt_id: &str,
        prompt_app_type: &str,
    ) -> Result<bool, AppError> {
        self.unlink_project_asset(project_id, ASSET_PROMPT, prompt_id, prompt_app_type)
    }

    pub fn set_project_prompts(
        &self,
        project_id: &str,
        prompts: &[(String, String)],
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        conn.execute(
            "DELETE FROM project_asset_links WHERE project_id = ?1 AND asset_type = ?2",
            params![project_id, ASSET_PROMPT],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        let mut stmt = conn
            .prepare(
                "INSERT INTO project_asset_links
                 (project_id, asset_type, asset_id, asset_app_type, enabled, scope, source, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 1, 'project', 'manual', ?5, ?5)",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        for (prompt_id, app_type) in prompts {
            stmt.execute(params![project_id, ASSET_PROMPT, prompt_id, app_type, now])
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
        Ok(())
    }
}
