//! 项目级配置隔离 - 数据访问对象
//!
//! 提供 Projects 及其与 MCP/Skills/Prompts 中间表的 CRUD 操作。

use crate::database::{lock_conn, Database};
use crate::error::AppError;
use rusqlite::params;
use serde::{Deserialize, Serialize};

/// 项目信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    pub git_remote_url: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

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
            .prepare(
                "SELECT id, name, path, git_remote_url, created_at, updated_at
                 FROM projects ORDER BY updated_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    path: row.get(2)?,
                    git_remote_url: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })
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
            "SELECT id, name, path, git_remote_url, created_at, updated_at
             FROM projects WHERE id = ?1",
            params![id],
            |row| {
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    path: row.get(2)?,
                    git_remote_url: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            },
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
            "SELECT id, name, path, git_remote_url, created_at, updated_at
             FROM projects WHERE path = ?1",
            params![path],
            |row| {
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    path: row.get(2)?,
                    git_remote_url: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            },
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
            "INSERT INTO projects (id, name, path, git_remote_url, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                path = excluded.path,
                git_remote_url = excluded.git_remote_url,
                updated_at = excluded.updated_at",
            params![
                project.id,
                project.name,
                project.path,
                project.git_remote_url,
                project.created_at,
                project.updated_at,
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

    // ========== Project × MCP Servers ==========

    /// 获取项目关联的 MCP 服务器 ID 列表
    pub fn get_project_mcp_servers(&self, project_id: &str) -> Result<Vec<ProjectConfigLink>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT project_id, mcp_server_id, enabled, created_at
                 FROM project_mcp_servers WHERE project_id = ?1
                 ORDER BY created_at ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params![project_id], |row| {
                Ok(ProjectConfigLink {
                    project_id: row.get(0)?,
                    config_id: row.get(1)?,
                    enabled: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut links = Vec::new();
        for row in rows {
            links.push(row.map_err(|e| AppError::Database(e.to_string()))?);
        }
        Ok(links)
    }

    /// 关联 MCP 服务器到项目
    pub fn link_project_mcp_server(
        &self,
        project_id: &str,
        mcp_server_id: &str,
        enabled: bool,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        conn.execute(
            "INSERT INTO project_mcp_servers (project_id, mcp_server_id, enabled, created_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(project_id, mcp_server_id) DO UPDATE SET enabled = excluded.enabled",
            params![project_id, mcp_server_id, enabled, now],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// 取消 MCP 服务器与项目的关联
    pub fn unlink_project_mcp_server(
        &self,
        project_id: &str,
        mcp_server_id: &str,
    ) -> Result<bool, AppError> {
        let conn = lock_conn!(self.conn);
        let affected = conn
            .execute(
                "DELETE FROM project_mcp_servers WHERE project_id = ?1 AND mcp_server_id = ?2",
                params![project_id, mcp_server_id],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(affected > 0)
    }

    /// 批量设置项目的 MCP 服务器关联（替换所有现有关联）
    pub fn set_project_mcp_servers(
        &self,
        project_id: &str,
        mcp_server_ids: &[String],
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        conn.execute(
            "DELETE FROM project_mcp_servers WHERE project_id = ?1",
            params![project_id],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        let mut stmt = conn
            .prepare(
                "INSERT INTO project_mcp_servers (project_id, mcp_server_id, enabled, created_at)
                 VALUES (?1, ?2, 1, ?3)",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        for id in mcp_server_ids {
            stmt.execute(params![project_id, id, now])
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
        Ok(())
    }

    // ========== Project × Skills ==========

    /// 获取项目关联的 Skill ID 列表
    pub fn get_project_skills(&self, project_id: &str) -> Result<Vec<ProjectConfigLink>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT project_id, skill_id, enabled, created_at
                 FROM project_skills WHERE project_id = ?1
                 ORDER BY created_at ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params![project_id], |row| {
                Ok(ProjectConfigLink {
                    project_id: row.get(0)?,
                    config_id: row.get(1)?,
                    enabled: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut links = Vec::new();
        for row in rows {
            links.push(row.map_err(|e| AppError::Database(e.to_string()))?);
        }
        Ok(links)
    }

    /// 关联 Skill 到项目
    pub fn link_project_skill(
        &self,
        project_id: &str,
        skill_id: &str,
        enabled: bool,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        conn.execute(
            "INSERT INTO project_skills (project_id, skill_id, enabled, created_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(project_id, skill_id) DO UPDATE SET enabled = excluded.enabled",
            params![project_id, skill_id, enabled, now],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// 取消 Skill 与项目的关联
    pub fn unlink_project_skill(
        &self,
        project_id: &str,
        skill_id: &str,
    ) -> Result<bool, AppError> {
        let conn = lock_conn!(self.conn);
        let affected = conn
            .execute(
                "DELETE FROM project_skills WHERE project_id = ?1 AND skill_id = ?2",
                params![project_id, skill_id],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(affected > 0)
    }

    /// 批量设置项目的 Skill 关联（替换所有现有关联）
    pub fn set_project_skills(
        &self,
        project_id: &str,
        skill_ids: &[String],
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        conn.execute(
            "DELETE FROM project_skills WHERE project_id = ?1",
            params![project_id],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        let mut stmt = conn
            .prepare(
                "INSERT INTO project_skills (project_id, skill_id, enabled, created_at)
                 VALUES (?1, ?2, 1, ?3)",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        for id in skill_ids {
            stmt.execute(params![project_id, id, now])
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
        Ok(())
    }

    // ========== Project × Prompts ==========

    /// 获取项目关联的 Prompt 列表
    pub fn get_project_prompts(&self, project_id: &str) -> Result<Vec<ProjectPromptLink>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT project_id, prompt_id, prompt_app_type, enabled, created_at
                 FROM project_prompts WHERE project_id = ?1
                 ORDER BY created_at ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params![project_id], |row| {
                Ok(ProjectPromptLink {
                    project_id: row.get(0)?,
                    prompt_id: row.get(1)?,
                    prompt_app_type: row.get(2)?,
                    enabled: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut links = Vec::new();
        for row in rows {
            links.push(row.map_err(|e| AppError::Database(e.to_string()))?);
        }
        Ok(links)
    }

    /// 关联 Prompt 到项目
    pub fn link_project_prompt(
        &self,
        project_id: &str,
        prompt_id: &str,
        prompt_app_type: &str,
        enabled: bool,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        conn.execute(
            "INSERT INTO project_prompts (project_id, prompt_id, prompt_app_type, enabled, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(project_id, prompt_id, prompt_app_type) DO UPDATE SET enabled = excluded.enabled",
            params![project_id, prompt_id, prompt_app_type, enabled, now],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// 取消 Prompt 与项目的关联
    pub fn unlink_project_prompt(
        &self,
        project_id: &str,
        prompt_id: &str,
        prompt_app_type: &str,
    ) -> Result<bool, AppError> {
        let conn = lock_conn!(self.conn);
        let affected = conn
            .execute(
                "DELETE FROM project_prompts WHERE project_id = ?1 AND prompt_id = ?2 AND prompt_app_type = ?3",
                params![project_id, prompt_id, prompt_app_type],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(affected > 0)
    }

    /// 批量设置项目的 Prompt 关联（替换所有现有关联）
    pub fn set_project_prompts(
        &self,
        project_id: &str,
        prompts: &[(String, String)], // (prompt_id, app_type)
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        conn.execute(
            "DELETE FROM project_prompts WHERE project_id = ?1",
            params![project_id],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

        let mut stmt = conn
            .prepare(
                "INSERT INTO project_prompts (project_id, prompt_id, prompt_app_type, enabled, created_at)
                 VALUES (?1, ?2, ?3, 1, ?4)",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        for (prompt_id, app_type) in prompts {
            stmt.execute(params![project_id, prompt_id, app_type, now])
                .map_err(|e| AppError::Database(e.to_string()))?;
        }
        Ok(())
    }
}
