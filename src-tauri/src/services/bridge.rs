//! Prompt bridge engine — cross-tool prompt synchronization.
//!
//! Three-layer bridge strategy:
//! - Layer 1 (Identity): Direct copy for universal markdown
//! - Layer 2 (Section-Mapping): Heuristic section title transformation
//! - Layer 3 (Preview): Show diff, let user confirm

use crate::database::{lock_conn, Database};
use crate::error::AppError;
use rusqlite::params;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgePreview {
    pub converted_content: String,
    pub unmapped_sections: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeCandidate {
    pub id: String,
    pub name: String,
    pub app_type: String,
    pub content_preview: String,
    pub bridge_source: Option<String>,
}

pub fn get_filename_for_app(app_type: &str) -> &'static str {
    match app_type {
        "claude" | "claude-desktop" => "CLAUDE.md",
        "gemini" => "GEMINI.md",
        _ => "AGENTS.md",
    }
}

/// Preview how content would be transformed from source to target app
pub fn preview_bridge(source_app: &str, target_app: &str, content: &str) -> BridgePreview {
    let mut converted = content.to_string();
    let mut unmapped = Vec::new();
    let mut warnings = Vec::new();

    if source_app.starts_with("claude") && !target_app.starts_with("claude") {
        converted = converted.replace("# Instructions", "# Project Instructions");
        let mut new_lines = Vec::new();
        let mut in_ref_block = false;
        for line in converted.lines() {
            if line.starts_with('@') && !line.contains(' ') {
                if !in_ref_block {
                    new_lines.push("## Referenced Files".to_string());
                    in_ref_block = true;
                }
                new_lines.push(format!("- {}", &line[1..]));
            } else {
                in_ref_block = false;
                new_lines.push(line.to_string());
            }
        }
        converted = new_lines.join("\n");
    } else if !source_app.starts_with("claude") && target_app.starts_with("claude") {
        converted = converted.replace("# Project Instructions", "# Instructions");
        converted = converted.replace("# Available Tools", "# Tools");
    }

    if target_app == "gemini" && converted.contains("# Skills") {
        warnings.push(
            "Gemini CLI does not support Skills sections. This section will be included but may not be recognized.".to_string(),
        );
        unmapped.push("# Skills".to_string());
    }

    BridgePreview {
        converted_content: converted,
        unmapped_sections: unmapped,
        warnings,
    }
}

/// Create a bridge from source to target app
pub fn bridge_prompt(
    db: &Database,
    source_app: &str,
    target_app: &str,
    prompt_id: &str,
) -> Result<serde_json::Value, AppError> {
    let conn = lock_conn!(db.conn);

    let (name, content, description): (String, String, Option<String>) = conn
        .query_row(
            "SELECT name, content, description FROM prompts WHERE id = ?1 AND app_type = ?2",
            params![prompt_id, source_app],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|e| AppError::Database(format!("Source prompt not found: {e}")))?;

    let preview = preview_bridge(source_app, target_app, &content);
    let bridge_source_ref = format!("{source_app}:{prompt_id}");
    let now = chrono::Utc::now().timestamp();

    conn.execute(
        "INSERT OR REPLACE INTO prompts (id, app_type, name, content, description, enabled, created_at, updated_at, bridge_source)
         VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6, ?6, ?7)",
        params![
            prompt_id,
            target_app,
            name,
            preview.converted_content,
            description,
            now,
            bridge_source_ref
        ],
    )
    .map_err(|e| AppError::Database(format!("Failed to create bridged prompt: {e}")))?;

    Ok(serde_json::json!({
        "id": prompt_id,
        "app_type": target_app,
        "bridge_source": bridge_source_ref,
        "warnings": preview.warnings
    }))
}

/// Get all prompts that can be bridged from a source app
pub fn get_bridgeable_prompts(
    db: &Database,
    source_app: &str,
) -> Result<Vec<BridgeCandidate>, AppError> {
    let conn = lock_conn!(db.conn);
    let mut stmt = conn
        .prepare(
            "SELECT id, name, app_type, content, bridge_source FROM prompts WHERE app_type = ?1",
        )
        .map_err(|e| AppError::Database(e.to_string()))?;

    let candidates = stmt
        .query_map(params![source_app], |row| {
            let content: String = row.get(3)?;
            let preview = if content.len() > 100 {
                format!("{}...", &content[..100])
            } else {
                content
            };
            Ok(BridgeCandidate {
                id: row.get(0)?,
                name: row.get(1)?,
                app_type: row.get(2)?,
                content_preview: preview,
                bridge_source: row.get(4)?,
            })
        })
        .map_err(|e| AppError::Database(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(candidates)
}

/// Push changes from source to all bridged targets
pub fn push_bridge_changes(
    db: &Database,
    source_app: &str,
    source_id: &str,
) -> Result<Vec<serde_json::Value>, AppError> {
    let conn = lock_conn!(db.conn);
    let bridge_ref = format!("{source_app}:{source_id}");

    let content: String = conn
        .query_row(
            "SELECT content FROM prompts WHERE id = ?1 AND app_type = ?2",
            params![source_id, source_app],
            |row| row.get(0),
        )
        .map_err(|e| AppError::Database(format!("Source prompt not found: {e}")))?;

    let mut stmt = conn
        .prepare("SELECT id, app_type FROM prompts WHERE bridge_source = ?1")
        .map_err(|e| AppError::Database(e.to_string()))?;

    let targets: Vec<(String, String)> = stmt
        .query_map(params![bridge_ref], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| AppError::Database(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    let mut results = Vec::new();
    let now = chrono::Utc::now().timestamp();

    for (target_id, target_app) in &targets {
        let preview = preview_bridge(source_app, target_app, &content);
        conn.execute(
            "UPDATE prompts SET content = ?1, updated_at = ?2 WHERE id = ?3 AND app_type = ?4",
            params![preview.converted_content, now, target_id, target_app],
        )
        .map_err(|e| AppError::Database(e.to_string()))?;
        results.push(serde_json::json!({
            "id": target_id, "app_type": target_app, "updated": true
        }));
    }

    Ok(results)
}

/// Unlink a bridge relationship
pub fn unlink_bridge(db: &Database, app_type: &str, prompt_id: &str) -> Result<(), AppError> {
    let conn = lock_conn!(db.conn);
    conn.execute(
        "UPDATE prompts SET bridge_source = NULL WHERE id = ?1 AND app_type = ?2",
        params![prompt_id, app_type],
    )
    .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(())
}
