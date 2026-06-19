//! Usage data export commands — export usage/cost data to CSV/JSON

use crate::database::lock_conn;
use crate::error::AppError;
use crate::store::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportUsageRequest {
    pub app_type: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub format: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageExportResult {
    pub content: String,
    pub suggested_filename: String,
}

#[tauri::command]
pub async fn export_usage(
    state: State<'_, AppState>,
    request: ExportUsageRequest,
) -> Result<UsageExportResult, String> {
    let db = state.db.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let conn = lock_conn!(db.conn);

        let mut sql = String::from(
            "SELECT request_id, provider_id, app_type, model, request_model, \
                    input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens, \
                    total_cost_usd, latency_ms, status_code, created_at, data_source \
             FROM proxy_request_logs WHERE 1=1",
        );
        let mut params_vec: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref app) = request.app_type {
            sql.push_str(" AND app_type = ?");
            params_vec.push(Box::new(app.clone()));
        }
        if let Some(ref start) = request.start_date {
            sql.push_str(" AND date(created_at, 'unixepoch') >= ?");
            params_vec.push(Box::new(start.clone()));
        }
        if let Some(ref end) = request.end_date {
            sql.push_str(" AND date(created_at, 'unixepoch') <= ?");
            params_vec.push(Box::new(end.clone()));
        }
        sql.push_str(" ORDER BY created_at DESC LIMIT 10000");

        let params_refs: Vec<&dyn rusqlite::types::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| format!("Query failed: {e}"))?;

        #[allow(clippy::type_complexity)]
        let rows: Vec<(
            String,
            String,
            String,
            String,
            Option<String>,
            i64,
            i64,
            i64,
            i64,
            String,
            i64,
            i64,
            i64,
            String,
        )> = stmt
            .query_map(params_refs.as_slice(), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, i64>(11)?,
                    row.get::<_, i64>(12)?,
                    row.get::<_, String>(13)?,
                ))
            })
            .map_err(|e| format!("Query map failed: {e}"))?
            .filter_map(|r| r.ok())
            .collect();

        let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");

        let (content, ext) = match request.format.as_str() {
            "json" => {
                let json_rows: Vec<serde_json::Value> = rows
                    .iter()
                    .map(|r| {
                        let time = chrono::DateTime::from_timestamp(r.12, 0)
                            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                            .unwrap_or_else(|| r.12.to_string());
                        serde_json::json!({
                            "request_id": r.0,
                            "provider_id": r.1,
                            "app_type": r.2,
                            "model": r.3,
                            "request_model": r.4,
                            "input_tokens": r.5,
                            "output_tokens": r.6,
                            "cache_read_tokens": r.7,
                            "cache_creation_tokens": r.8,
                            "total_cost_usd": r.9,
                            "latency_ms": r.10,
                            "status_code": r.11,
                            "created_at": time,
                            "data_source": r.13,
                        })
                    })
                    .collect();
                (
                    serde_json::to_string_pretty(&json_rows).unwrap_or_default(),
                    "json",
                )
            }
            _ => {
                // CSV with BOM for Excel compatibility
                let mut csv = String::from("\u{FEFF}");
                csv.push_str(
                    "Request ID,Provider ID,App Type,Model,Request Model,\
                     Input Tokens,Output Tokens,Cache Read Tokens,Cache Creation Tokens,\
                     Total Cost USD,Latency ms,Status Code,Created At,Data Source\n",
                );
                for r in &rows {
                    let time = chrono::DateTime::from_timestamp(r.12, 0)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| r.12.to_string());
                    csv.push_str(&format!(
                        "{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
                        r.0,
                        r.1,
                        r.2,
                        r.3,
                        r.4.as_deref().unwrap_or(""),
                        r.5,
                        r.6,
                        r.7,
                        r.8,
                        r.9,
                        r.10,
                        r.11,
                        time,
                        r.13,
                    ));
                }
                (csv, "csv")
            }
        };

        let suggested_filename = format!("usage_export_{}.{}", timestamp, ext);
        Ok(UsageExportResult {
            content,
            suggested_filename,
        })
    })
    .await
    .map_err(|e| format!("Task join failed: {e}"))?;

    result
}
