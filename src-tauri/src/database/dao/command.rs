//! Slash commands data access

use crate::command::Command;
use crate::database::{lock_conn, Database};
use crate::error::AppError;
use indexmap::IndexMap;
use rusqlite::params;

impl Database {
    pub fn get_all_commands(&self) -> Result<IndexMap<String, Command>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT id, name, description, content, arguments,
                        enabled_claude, enabled_codex, enabled_gemini,
                        enabled_opencode, enabled_hermes, created_at, updated_at
                 FROM commands
                 ORDER BY created_at ASC, name ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    Command {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        description: row.get(2)?,
                        content: row.get(3)?,
                        arguments: row.get(4)?,
                        enabled_claude: row.get(5)?,
                        enabled_codex: row.get(6)?,
                        enabled_gemini: row.get(7)?,
                        enabled_opencode: row.get(8)?,
                        enabled_hermes: row.get(9)?,
                        created_at: row.get(10)?,
                        updated_at: row.get(11)?,
                    },
                ))
            })
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut commands = IndexMap::new();
        for row in rows {
            let (id, command) = row.map_err(|e| AppError::Database(e.to_string()))?;
            commands.insert(id, command);
        }
        Ok(commands)
    }

    pub fn save_command(&self, command: &Command) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT OR REPLACE INTO commands (
                id, name, description, content, arguments,
                enabled_claude, enabled_codex, enabled_gemini,
                enabled_opencode, enabled_hermes, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                command.id,
                command.name,
                command.description,
                command.content,
                command.arguments,
                command.enabled_claude,
                command.enabled_codex,
                command.enabled_gemini,
                command.enabled_opencode,
                command.enabled_hermes,
                command.created_at,
                command.updated_at,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn delete_command(&self, id: &str) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute("DELETE FROM commands WHERE id = ?1", params![id])
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }
}
