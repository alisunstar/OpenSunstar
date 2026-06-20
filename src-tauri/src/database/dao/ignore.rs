//! Ignore rules data access

use crate::database::{lock_conn, Database};
use crate::error::AppError;
use crate::ignore_rule::IgnoreRule;
use rusqlite::params;

impl Database {
    pub fn get_all_ignore_rules(&self) -> Result<Vec<IgnoreRule>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT id, pattern, description,
                        enabled_claude, enabled_codex, enabled_gemini,
                        enabled_opencode, enabled_hermes, sort_index, created_at
                 FROM ignore_rules
                 ORDER BY sort_index ASC, created_at ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(IgnoreRule {
                    id: row.get(0)?,
                    pattern: row.get(1)?,
                    description: row.get(2)?,
                    enabled_claude: row.get(3)?,
                    enabled_codex: row.get(4)?,
                    enabled_gemini: row.get(5)?,
                    enabled_opencode: row.get(6)?,
                    enabled_hermes: row.get(7)?,
                    sort_index: row.get(8)?,
                    created_at: row.get(9)?,
                })
            })
            .map_err(|e| AppError::Database(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::Database(e.to_string()))
    }

    pub fn save_ignore_rule(&self, rule: &IgnoreRule) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT OR REPLACE INTO ignore_rules (
                id, pattern, description,
                enabled_claude, enabled_codex, enabled_gemini,
                enabled_opencode, enabled_hermes, sort_index, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                rule.id,
                rule.pattern,
                rule.description,
                rule.enabled_claude,
                rule.enabled_codex,
                rule.enabled_gemini,
                rule.enabled_opencode,
                rule.enabled_hermes,
                rule.sort_index,
                rule.created_at,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn delete_ignore_rule(&self, id: &str) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute("DELETE FROM ignore_rules WHERE id = ?1", params![id])
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn clear_ignore_rules(&self) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute("DELETE FROM ignore_rules", [])
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }
}
