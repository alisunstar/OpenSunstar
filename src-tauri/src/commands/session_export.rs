//! Session export commands — export conversation sessions to Markdown/JSON/TXT

use crate::database::lock_conn;
use crate::error::AppError;
use crate::store::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportSessionRequest {
    pub app_type: String,
    pub session_id: String,
    pub format: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub content: String,
    pub suggested_filename: String,
}

#[tauri::command]
pub async fn export_session(
    state: State<'_, AppState>,
    request: ExportSessionRequest,
) -> Result<ExportResult, String> {
    let db = state.db.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let conn = lock_conn!(db.conn);

        let mut stmt = conn
            .prepare(
                "SELECT model, request_id, created_at, error_message, provider_id \
                 FROM proxy_request_logs \
                 WHERE app_type = ?1 AND session_id = ?2 \
                 ORDER BY created_at ASC",
            )
            .map_err(|e| format!("Query failed: {e}"))?;

        let rows: Vec<(String, String, i64, Option<String>, String)> = stmt
            .query_map(
                rusqlite::params![request.app_type, request.session_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )
            .map_err(|e| format!("Query map failed: {e}"))?
            .filter_map(|r| r.ok())
            .collect();

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");

        let (content, ext) = match request.format.as_str() {
            "json" => {
                let json_rows: Vec<serde_json::Value> = rows
                    .iter()
                    .map(|(model, req_id, ts, err, provider)| {
                        let time = chrono::DateTime::from_timestamp(*ts, 0)
                            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                            .unwrap_or_else(|| ts.to_string());
                        serde_json::json!({
                            "model": model,
                            "request_id": req_id,
                            "created_at": time,
                            "error_message": err,
                            "provider_id": provider,
                        })
                    })
                    .collect();
                let json = serde_json::to_string_pretty(&json_rows).unwrap_or_default();
                (json, "json")
            }
            "text" => {
                let mut text = String::new();
                for (model, _req_id, ts, err, provider) in &rows {
                    let time = chrono::DateTime::from_timestamp(*ts, 0)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| ts.to_string());
                    text.push_str(&format!("[{time}] {provider}/{model}"));
                    if let Some(e) = err {
                        text.push_str(&format!(" ERROR: {e}"));
                    }
                    text.push('\n');
                }
                (text, "txt")
            }
            _ => {
                let mut md = format!("# Session Export: {}\n\n", request.session_id);
                md.push_str(&format!("- **App**: {}\n", request.app_type));
                md.push_str(&format!(
                    "- **Exported**: {}\n",
                    chrono::Utc::now().to_rfc3339()
                ));
                md.push_str(&format!("- **Entries**: {}\n\n---\n\n", rows.len()));
                for (model, _req_id, ts, err, provider) in &rows {
                    let time = chrono::DateTime::from_timestamp(*ts, 0)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| ts.to_string());
                    md.push_str(&format!("### [{time}] {provider} / {model}\n\n"));
                    if let Some(e) = err {
                        md.push_str(&format!("> Error: {e}\n\n"));
                    }
                }
                (md, "md")
            }
        };

        let suggested_filename = format!("session_{}_{}.{}", request.app_type, timestamp, ext);

        Ok(ExportResult {
            content,
            suggested_filename,
        })
    })
    .await
    .map_err(|e| format!("Task join failed: {e}"))?;

    result
}
