//! Team Config workspace & release — data access (Git MVP Local Alpha)
//!
//! team_workspaces: 已连接的团队配置源（只读元数据缓存）
//! team_releases: Release 快照（lock.json manifest），用于 Diff 基线

use crate::database::{lock_conn, Database};
use crate::error::AppError;
use rusqlite::params;
use rusqlite::OptionalExtension;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamWorkspace {
    pub workspace_id: String,
    pub name: String,
    pub source_kind: String,
    pub source_path: String,
    pub branch: Option<String>,
    pub last_synced_commit: Option<String>,
    pub last_synced_at: Option<i64>,
    pub team_toml_snapshot: Option<String>,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamRelease {
    pub release_id: String,
    pub workspace_id: String,
    pub tag: String,
    pub commit_sha: Option<String>,
    pub manifest_json: String,
    pub created_at: i64,
    pub created_by: Option<String>,
    pub status: String,
}

impl Database {
    // ─── team_workspaces ─────────────────────────────────────────────────────

    pub fn upsert_team_workspace(&self, ws: &TeamWorkspace) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT INTO team_workspaces (
                workspace_id, name, source_kind, source_path, branch,
                last_synced_commit, last_synced_at, team_toml_snapshot,
                status, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            ON CONFLICT(workspace_id) DO UPDATE SET
                name = excluded.name,
                source_kind = excluded.source_kind,
                source_path = excluded.source_path,
                branch = excluded.branch,
                last_synced_commit = excluded.last_synced_commit,
                last_synced_at = excluded.last_synced_at,
                team_toml_snapshot = excluded.team_toml_snapshot,
                status = excluded.status,
                updated_at = excluded.updated_at",
            params![
                ws.workspace_id,
                ws.name,
                ws.source_kind,
                ws.source_path,
                ws.branch,
                ws.last_synced_commit,
                ws.last_synced_at,
                ws.team_toml_snapshot,
                ws.status,
                ws.created_at,
                ws.updated_at,
            ],
        )
        .map_err(|e| AppError::Database(format!("upsert team_workspaces 失败: {e}")))?;
        Ok(())
    }

    pub fn get_team_workspace(&self, workspace_id: &str) -> Result<Option<TeamWorkspace>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT workspace_id, name, source_kind, source_path, branch,
                        last_synced_commit, last_synced_at, team_toml_snapshot,
                        status, created_at, updated_at
                 FROM team_workspaces WHERE workspace_id = ?1",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let result = stmt
            .query_row(params![workspace_id], |row| {
                Ok(TeamWorkspace {
                    workspace_id: row.get(0)?,
                    name: row.get(1)?,
                    source_kind: row.get(2)?,
                    source_path: row.get(3)?,
                    branch: row.get(4)?,
                    last_synced_commit: row.get(5)?,
                    last_synced_at: row.get(6)?,
                    team_toml_snapshot: row.get(7)?,
                    status: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })
            .optional()
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(result)
    }

    pub fn get_team_workspace_by_path(&self, source_path: &str) -> Result<Option<TeamWorkspace>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT workspace_id, name, source_kind, source_path, branch,
                        last_synced_commit, last_synced_at, team_toml_snapshot,
                        status, created_at, updated_at
                 FROM team_workspaces WHERE source_path = ?1 AND status = 'active'",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let result = stmt
            .query_row(params![source_path], |row| {
                Ok(TeamWorkspace {
                    workspace_id: row.get(0)?,
                    name: row.get(1)?,
                    source_kind: row.get(2)?,
                    source_path: row.get(3)?,
                    branch: row.get(4)?,
                    last_synced_commit: row.get(5)?,
                    last_synced_at: row.get(6)?,
                    team_toml_snapshot: row.get(7)?,
                    status: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })
            .optional()
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(result)
    }

    pub fn list_team_workspaces(&self) -> Result<Vec<TeamWorkspace>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT workspace_id, name, source_kind, source_path, branch,
                        last_synced_commit, last_synced_at, team_toml_snapshot,
                        status, created_at, updated_at
                 FROM team_workspaces WHERE status = 'active'
                 ORDER BY updated_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(TeamWorkspace {
                    workspace_id: row.get(0)?,
                    name: row.get(1)?,
                    source_kind: row.get(2)?,
                    source_path: row.get(3)?,
                    branch: row.get(4)?,
                    last_synced_commit: row.get(5)?,
                    last_synced_at: row.get(6)?,
                    team_toml_snapshot: row.get(7)?,
                    status: row.get(8)?,
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| AppError::Database(e.to_string()))?);
        }
        Ok(result)
    }

    pub fn archive_team_workspace(&self, workspace_id: &str) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "UPDATE team_workspaces SET status = 'archived', updated_at = ?2 WHERE workspace_id = ?1",
            params![workspace_id, now],
        )
        .map_err(|e| AppError::Database(format!("archive team_workspaces 失败: {e}")))?;
        Ok(())
    }

    // ─── team_releases ───────────────────────────────────────────────────────

    pub fn insert_team_release(&self, release: &TeamRelease) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT INTO team_releases (
                release_id, workspace_id, tag, commit_sha,
                manifest_json, created_at, created_by, status
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                release.release_id,
                release.workspace_id,
                release.tag,
                release.commit_sha,
                release.manifest_json,
                release.created_at,
                release.created_by,
                release.status,
            ],
        )
        .map_err(|e| AppError::Database(format!("insert team_releases 失败: {e}")))?;
        Ok(())
    }

    pub fn get_latest_team_release(&self, workspace_id: &str) -> Result<Option<TeamRelease>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT release_id, workspace_id, tag, commit_sha,
                        manifest_json, created_at, created_by, status
                 FROM team_releases
                 WHERE workspace_id = ?1 AND status = 'active'
                 ORDER BY created_at DESC LIMIT 1",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let result = stmt
            .query_row(params![workspace_id], |row| {
                Ok(TeamRelease {
                    release_id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    tag: row.get(2)?,
                    commit_sha: row.get(3)?,
                    manifest_json: row.get(4)?,
                    created_at: row.get(5)?,
                    created_by: row.get(6)?,
                    status: row.get(7)?,
                })
            })
            .optional()
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(result)
    }

    pub fn list_team_releases(&self, workspace_id: &str) -> Result<Vec<TeamRelease>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT release_id, workspace_id, tag, commit_sha,
                        manifest_json, created_at, created_by, status
                 FROM team_releases
                 WHERE workspace_id = ?1
                 ORDER BY created_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params![workspace_id], |row| {
                Ok(TeamRelease {
                    release_id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    tag: row.get(2)?,
                    commit_sha: row.get(3)?,
                    manifest_json: row.get(4)?,
                    created_at: row.get(5)?,
                    created_by: row.get(6)?,
                    status: row.get(7)?,
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| AppError::Database(e.to_string()))?);
        }
        Ok(result)
    }

    // ─── team_assignments (M1) ──────────────────────────────────────────────

    pub fn upsert_team_assignment(
        &self,
        assignment_id: &str,
        project_id: &str,
        workspace_id: &str,
        profile_id: &str,
        status: &str,
        created_at: i64,
        updated_at: i64,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT INTO team_assignments (
                assignment_id, project_id, workspace_id, profile_id,
                status, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(assignment_id) DO UPDATE SET
                profile_id = excluded.profile_id,
                status = excluded.status,
                updated_at = excluded.updated_at",
            params![assignment_id, project_id, workspace_id, profile_id, status, created_at, updated_at],
        )
        .map_err(|e| AppError::Database(format!("upsert team_assignments 失败: {e}")))?;
        Ok(())
    }

    pub fn get_active_team_assignment(&self, project_id: &str) -> Result<Option<TeamAssignmentRow>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT assignment_id, project_id, workspace_id, profile_id,
                        status, deployed_at, deployed_release_id, created_at, updated_at
                 FROM team_assignments
                 WHERE project_id = ?1 AND status = 'active'",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let result = stmt
            .query_row(params![project_id], |row| {
                Ok(TeamAssignmentRow {
                    assignment_id: row.get(0)?,
                    project_id: row.get(1)?,
                    workspace_id: row.get(2)?,
                    profile_id: row.get(3)?,
                    status: row.get(4)?,
                    deployed_at: row.get(5)?,
                    deployed_release_id: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })
            .optional()
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(result)
    }

    pub fn list_team_assignments_by_workspace(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<TeamAssignmentRow>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT assignment_id, project_id, workspace_id, profile_id,
                        status, deployed_at, deployed_release_id, created_at, updated_at
                 FROM team_assignments
                 WHERE workspace_id = ?1 AND status != 'removed'
                 ORDER BY updated_at DESC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = stmt
            .query_map(params![workspace_id], |row| {
                Ok(TeamAssignmentRow {
                    assignment_id: row.get(0)?,
                    project_id: row.get(1)?,
                    workspace_id: row.get(2)?,
                    profile_id: row.get(3)?,
                    status: row.get(4)?,
                    deployed_at: row.get(5)?,
                    deployed_release_id: row.get(6)?,
                    created_at: row.get(7)?,
                    updated_at: row.get(8)?,
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut result = Vec::new();
        for row in rows {
            result.push(row.map_err(|e| AppError::Database(e.to_string()))?);
        }
        Ok(result)
    }

    pub fn update_team_assignment_status(
        &self,
        assignment_id: &str,
        status: &str,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "UPDATE team_assignments SET status = ?2, updated_at = ?3 WHERE assignment_id = ?1",
            params![assignment_id, status, now],
        )
        .map_err(|e| AppError::Database(format!("update team_assignments status 失败: {e}")))?;
        Ok(())
    }

    pub fn mark_team_assignment_deployed(
        &self,
        assignment_id: &str,
        release_id: &str,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "UPDATE team_assignments SET deployed_at = ?2, deployed_release_id = ?3, updated_at = ?2
             WHERE assignment_id = ?1",
            params![assignment_id, now, release_id],
        )
        .map_err(|e| AppError::Database(format!("mark team_assignments deployed 失败: {e}")))?;
        Ok(())
    }

    // ─── team_audit_local (M8) ──────────────────────────────────────────────

    pub fn insert_team_audit_event(
        &self,
        event_id: &str,
        workspace_id: Option<&str>,
        project_id: Option<&str>,
        action: &str,
        actor_ref: &str,
        details_json: Option<&str>,
        success: bool,
        created_at: i64,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT INTO team_audit_local (
                event_id, workspace_id, project_id, action,
                actor_ref, details_json, success, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![event_id, workspace_id, project_id, action, actor_ref, details_json, success as i32, created_at],
        )
        .map_err(|e| AppError::Database(format!("insert team_audit_local 失败: {e}")))?;
        Ok(())
    }

    pub fn list_team_audit_events(
        &self,
        workspace_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<TeamAuditRow>, AppError> {
        let conn = lock_conn!(self.conn);
        let limit_i64 = limit as i64;

        let rows = match workspace_id {
            Some(ws) => {
                let mut stmt = conn
                    .prepare(
                        "SELECT event_id, workspace_id, project_id, action,
                                actor_ref, details_json, success, created_at
                         FROM team_audit_local WHERE workspace_id = ?1
                         ORDER BY created_at DESC LIMIT ?2",
                    )
                    .map_err(|e| AppError::Database(e.to_string()))?;
                let result = stmt
                    .query_map(params![ws, limit_i64], Self::map_audit_row)
                    .map_err(|e| AppError::Database(e.to_string()))?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| AppError::Database(e.to_string()))?;
                result
            }
            None => {
                let mut stmt = conn
                    .prepare(
                        "SELECT event_id, workspace_id, project_id, action,
                                actor_ref, details_json, success, created_at
                         FROM team_audit_local
                         ORDER BY created_at DESC LIMIT ?1",
                    )
                    .map_err(|e| AppError::Database(e.to_string()))?;
                let result = stmt
                    .query_map(params![limit_i64], Self::map_audit_row)
                    .map_err(|e| AppError::Database(e.to_string()))?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| AppError::Database(e.to_string()))?;
                result
            }
        };

        Ok(rows)
    }

    fn map_audit_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TeamAuditRow> {
        Ok(TeamAuditRow {
            event_id: row.get(0)?,
            workspace_id: row.get(1)?,
            project_id: row.get(2)?,
            action: row.get(3)?,
            actor_ref: row.get(4)?,
            details_json: row.get(5)?,
            success: row.get::<_, i32>(6)? != 0,
            created_at: row.get(7)?,
        })
    }
}

/// team_assignments 行映射（DAO 内部用）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamAssignmentRow {
    pub assignment_id: String,
    pub project_id: String,
    pub workspace_id: String,
    pub profile_id: String,
    pub status: String,
    pub deployed_at: Option<i64>,
    pub deployed_release_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// team_audit_local 行映射
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamAuditRow {
    pub event_id: String,
    pub workspace_id: Option<String>,
    pub project_id: Option<String>,
    pub action: String,
    pub actor_ref: String,
    pub details_json: Option<String>,
    pub success: bool,
    pub created_at: i64,
}
