//! 提示词数据访问对象
//!
//! 提供提示词（Prompt）的 CRUD 操作。

use crate::database::{lock_conn, Database};
use crate::error::AppError;
use crate::prompt::Prompt;
use indexmap::IndexMap;
use rusqlite::params;

fn row_to_prompt(row: &rusqlite::Row<'_>) -> rusqlite::Result<Prompt> {
    Ok(Prompt {
        id: row.get(0)?,
        name: row.get(1)?,
        content: row.get(2)?,
        description: row.get(3)?,
        enabled: row.get(4)?,
        targets: row.get(5)?,
        globs: row.get(6)?,
        priority: row.get(7)?,
        is_fragment: row.get(8)?,
        parent_prompt_id: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

const PROMPT_SELECT: &str = "SELECT id, name, content, description, enabled,
    targets, globs, priority, is_fragment, parent_prompt_id, created_at, updated_at";

impl Database {
    /// 获取指定应用类型的所有提示词
    pub fn get_prompts(&self, app_type: &str) -> Result<IndexMap<String, Prompt>, AppError> {
        let conn = lock_conn!(self.conn);
        let sql = format!(
            "{PROMPT_SELECT} FROM prompts WHERE app_type = ?1 ORDER BY created_at ASC, id ASC"
        );
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| AppError::Database(e.to_string()))?;

        let prompt_iter = stmt
            .query_map(params![app_type], row_to_prompt)
            .map_err(|e| AppError::Database(e.to_string()))?;

        let mut prompts = IndexMap::new();
        for prompt_res in prompt_iter {
            let prompt = prompt_res.map_err(|e| AppError::Database(e.to_string()))?;
            prompts.insert(prompt.id.clone(), prompt);
        }
        Ok(prompts)
    }

    pub fn count_fragments_for_parent(
        &self,
        app_type: &str,
        parent_id: &str,
    ) -> Result<usize, AppError> {
        let conn = lock_conn!(self.conn);
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM prompts
                 WHERE app_type = ?1 AND parent_prompt_id = ?2 AND is_fragment = 1",
                params![app_type, parent_id],
                |row| row.get(0),
            )
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(count as usize)
    }

    pub fn delete_fragments_for_parent(
        &self,
        app_type: &str,
        parent_id: &str,
    ) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "DELETE FROM prompts WHERE app_type = ?1 AND parent_prompt_id = ?2",
            params![app_type, parent_id],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// 保存提示词
    pub fn save_prompt(&self, app_type: &str, prompt: &Prompt) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT OR REPLACE INTO prompts (
                id, app_type, name, content, description, enabled,
                targets, globs, priority, is_fragment, parent_prompt_id,
                created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                prompt.id,
                app_type,
                prompt.name,
                prompt.content,
                prompt.description,
                prompt.enabled,
                prompt.targets,
                prompt.globs,
                prompt.priority,
                prompt.is_fragment,
                prompt.parent_prompt_id,
                prompt.created_at,
                prompt.updated_at,
            ],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }

    /// 删除提示词
    pub fn delete_prompt(&self, app_type: &str, id: &str) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "DELETE FROM prompts WHERE id = ?1 AND app_type = ?2",
            params![id, app_type],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }
}
