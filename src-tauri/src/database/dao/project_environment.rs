//! Project environment snapshots.
//!
//! A snapshot is bound to a real OpenSunstar project and stores the runtime
//! state needed to restore the user's working environment.

use crate::database::{lock_conn, Database};
use crate::error::AppError;
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectEnvironmentSnapshot {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub payload: String,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_applied_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_apply_receipt: Option<String>,
}

fn map_snapshot_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectEnvironmentSnapshot> {
    Ok(ProjectEnvironmentSnapshot {
        id: row.get(0)?,
        project_id: row.get(1)?,
        name: row.get(2)?,
        payload: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
        last_applied_at: row.get(6)?,
        last_apply_receipt: row.get(7)?,
    })
}

impl Database {
    pub fn get_project_environment_snapshots(
        &self,
        project_id: &str,
    ) -> Result<Vec<ProjectEnvironmentSnapshot>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT id, project_id, name, payload, created_at, updated_at,
                        last_applied_at, last_apply_receipt
                 FROM project_environment_snapshots
                 WHERE project_id = ?1
                 ORDER BY updated_at DESC, created_at DESC, name",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        let rows = stmt
            .query_map(params![project_id], map_snapshot_row)
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut snapshots = Vec::new();
        for row in rows {
            snapshots.push(row.map_err(|e| AppError::Database(e.to_string()))?);
        }
        Ok(snapshots)
    }

    pub fn get_project_environment_snapshot(
        &self,
        id: &str,
    ) -> Result<Option<ProjectEnvironmentSnapshot>, AppError> {
        let conn = lock_conn!(self.conn);
        let result = conn.query_row(
            "SELECT id, project_id, name, payload, created_at, updated_at,
                    last_applied_at, last_apply_receipt
             FROM project_environment_snapshots
             WHERE id = ?1",
            params![id],
            map_snapshot_row,
        );
        match result {
            Ok(snapshot) => Ok(Some(snapshot)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(AppError::Database(e.to_string())),
        }
    }

    pub fn save_project_environment_snapshot(
        &self,
        snapshot: &ProjectEnvironmentSnapshot,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT INTO project_environment_snapshots
             (id, project_id, name, payload, created_at, updated_at, last_applied_at, last_apply_receipt)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                payload = excluded.payload,
                updated_at = excluded.updated_at,
                last_applied_at = excluded.last_applied_at,
                last_apply_receipt = excluded.last_apply_receipt",
            params![
                snapshot.id,
                snapshot.project_id,
                snapshot.name,
                snapshot.payload,
                snapshot.created_at,
                snapshot.updated_at,
                snapshot.last_applied_at,
                snapshot.last_apply_receipt,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn update_project_environment_snapshot_receipt(
        &self,
        id: &str,
        last_applied_at: i64,
        receipt: &str,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        let affected = conn
            .execute(
                "UPDATE project_environment_snapshots
                 SET last_applied_at = ?2, last_apply_receipt = ?3, updated_at = ?2
                 WHERE id = ?1",
                params![id, last_applied_at, receipt],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        if affected == 0 {
            return Err(AppError::InvalidInput(format!("项目环境快照不存在: {id}")));
        }
        Ok(())
    }

    pub fn delete_project_environment_snapshot(&self, id: &str) -> Result<bool, AppError> {
        let conn = lock_conn!(self.conn);
        let affected = conn
            .execute(
                "DELETE FROM project_environment_snapshots WHERE id = ?1",
                params![id],
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(affected > 0)
    }
}
