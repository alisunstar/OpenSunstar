//! Claude Code hooks data access

use crate::database::{lock_conn, Database};
use crate::error::AppError;
use crate::hook::Hook;
use rusqlite::params;

impl Database {
    pub fn get_all_hooks(&self) -> Result<Vec<Hook>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT id, event_type, tool_pattern, hook_command, timeout_seconds,
                        enabled_claude, description, sort_index, created_at
                 FROM hooks
                 ORDER BY sort_index ASC, created_at ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(Hook {
                    id: row.get(0)?,
                    event_type: row.get(1)?,
                    tool_pattern: row.get(2)?,
                    hook_command: row.get(3)?,
                    timeout_seconds: row.get(4)?,
                    enabled_claude: row.get(5)?,
                    description: row.get(6)?,
                    sort_index: row.get(7)?,
                    created_at: row.get(8)?,
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))
    }

    pub fn save_hook(&self, hook: &Hook) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT OR REPLACE INTO hooks (
                id, event_type, tool_pattern, hook_command, timeout_seconds,
                enabled_claude, description, sort_index, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                hook.id,
                hook.event_type,
                hook.tool_pattern,
                hook.hook_command,
                hook.timeout_seconds,
                hook.enabled_claude,
                hook.description,
                hook.sort_index,
                hook.created_at,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn delete_hook(&self, id: &str) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute("DELETE FROM hooks WHERE id = ?1", params![id])
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }
}
