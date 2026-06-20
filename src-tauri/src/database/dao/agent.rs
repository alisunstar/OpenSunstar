//! Subagent definitions data access

use crate::agent::Agent;
use crate::database::{lock_conn, Database};
use crate::error::AppError;
use indexmap::IndexMap;
use rusqlite::params;

impl Database {
    pub fn get_all_agents(&self) -> Result<IndexMap<String, Agent>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn
            .prepare(
                "SELECT id, name, description, content,
                        enabled_claude, enabled_codex, enabled_gemini,
                        enabled_opencode, enabled_hermes, created_at, updated_at
                 FROM agents
                 ORDER BY created_at ASC, name ASC",
            )
            .map_err(|e| AppError::Database(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    Agent {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        description: row.get(2)?,
                        content: row.get(3)?,
                        enabled_claude: row.get(4)?,
                        enabled_codex: row.get(5)?,
                        enabled_gemini: row.get(6)?,
                        enabled_opencode: row.get(7)?,
                        enabled_hermes: row.get(8)?,
                        created_at: row.get(9)?,
                        updated_at: row.get(10)?,
                    },
                ))
            })
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut agents = IndexMap::new();
        for row in rows {
            let (id, agent) = row.map_err(|e| AppError::Database(e.to_string()))?;
            agents.insert(id, agent);
        }
        Ok(agents)
    }

    pub fn save_agent(&self, agent: &Agent) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT OR REPLACE INTO agents (
                id, name, description, content,
                enabled_claude, enabled_codex, enabled_gemini,
                enabled_opencode, enabled_hermes, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                agent.id,
                agent.name,
                agent.description,
                agent.content,
                agent.enabled_claude,
                agent.enabled_codex,
                agent.enabled_gemini,
                agent.enabled_opencode,
                agent.enabled_hermes,
                agent.created_at,
                agent.updated_at,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    pub fn delete_agent(&self, id: &str) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute("DELETE FROM agents WHERE id = ?1", params![id])
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }
}
