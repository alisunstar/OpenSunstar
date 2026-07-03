//! Tool permissions data access

use crate::database::{lock_conn, Database};
use crate::error::AppError;
use crate::tool_permission::ToolPermission;
use rusqlite::params;

const PERM_SELECT: &str = "SELECT id, permission_type, tool_pattern, enabled_claude,
                        enabled_codex, enabled_gemini, enabled_opencode, enabled_hermes,
                        enabled_openclaw, description, sort_index, created_at";

impl Database {
    pub fn get_all_tool_permissions(&self) -> Result<Vec<ToolPermission>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(&format!(
                "{PERM_SELECT} FROM tool_permissions ORDER BY sort_index ASC, created_at ASC"
            ))
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(ToolPermission {
                    id: row.get(0)?,
                    permission_type: row.get(1)?,
                    tool_pattern: row.get(2)?,
                    enabled_claude: row.get(3)?,
                    enabled_codex: row.get(4)?,
                    enabled_gemini: row.get(5)?,
                    enabled_opencode: row.get(6)?,
                    enabled_hermes: row.get(7)?,
                    enabled_openclaw: row.get(8)?,
                    description: row.get(9)?,
                    sort_index: row.get(10)?,
                    created_at: row.get(11)?,
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))
    }

    pub fn save_tool_permission(&self, perm: &ToolPermission) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT OR REPLACE INTO tool_permissions (
                id, permission_type, tool_pattern, enabled_claude,
                enabled_codex, enabled_gemini, enabled_opencode, enabled_hermes,
                enabled_openclaw, description, sort_index, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                perm.id,
                perm.permission_type,
                perm.tool_pattern,
                perm.enabled_claude,
                perm.enabled_codex,
                perm.enabled_gemini,
                perm.enabled_opencode,
                perm.enabled_hermes,
                perm.enabled_openclaw,
                perm.description,
                perm.sort_index,
                perm.created_at,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn delete_tool_permission(&self, id: &str) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute("DELETE FROM tool_permissions WHERE id = ?1", params![id])
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn clear_tool_permissions(&self) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute("DELETE FROM tool_permissions", [])
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }
}
