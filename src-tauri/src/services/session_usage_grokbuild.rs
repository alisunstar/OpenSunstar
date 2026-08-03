//! Grok Build 会话用量同步。
//!
//! Grok Build 将逐轮用量写入 `~/.grok/{sessions,archived_sessions}/**/updates.jsonl`。
//! 本模块只接入全局会话用量，不扩展项目级 Command、Hook、Ignore、Permission
//! 或 Subagent 配置。

use crate::database::{lock_conn, Database};
use crate::error::AppError;
use crate::grok_config::get_grok_config_dir;
use crate::proxy::usage::calculator::CostCalculator;
use crate::proxy::usage::parser::TokenUsage;
use crate::services::session_usage::{
    get_sync_state, metadata_modified_nanos, update_sync_state, SessionSyncResult,
};
use crate::services::usage_stats::{
    find_model_pricing, has_recent_grokbuild_proxy_activity, SESSION_PROXY_DEDUP_WINDOW_SECONDS,
};
use rust_decimal::Decimal;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

const SETTLE_WINDOW_SECONDS: i64 = SESSION_PROXY_DEDUP_WINDOW_SECONDS;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct GrokCounters {
    input: u64,
    output: u64,
    cached: u64,
    api_ms: u64,
    cost_ticks: u64,
    cost_partial: bool,
}

impl GrokCounters {
    fn is_zero(self) -> bool {
        self.input == 0 && self.output == 0 && self.cached == 0
    }

    fn reported_cost_usd(self) -> Option<Decimal> {
        (self.cost_ticks > 0)
            .then(|| Decimal::from(self.cost_ticks) / Decimal::from(10_000_000_000u64))
    }
}

#[derive(Debug, PartialEq, Eq)]
struct GrokUsageEvent {
    created_at: i64,
    prompt_id: String,
    cost_is_partial: bool,
    per_model: Vec<(String, GrokCounters)>,
}

/// 同步 Grok Build 会话日志到使用统计数据库。
pub fn sync_grokbuild_usage(db: &Database) -> Result<SessionSyncResult, AppError> {
    let files = collect_grok_updates_files();
    let mut result = SessionSyncResult {
        files_scanned: files.len() as u32,
        ..Default::default()
    };

    for file_path in files {
        match sync_single_grok_file(db, &file_path) {
            Ok(file_result) => result.merge(file_result),
            Err(error) => {
                let message = format!(
                    "Grok Build 会话文件解析失败 {}: {error}",
                    file_path.display()
                );
                log::warn!("[GROK-SYNC] {message}");
                result.errors.push(message);
            }
        }
    }

    Ok(result)
}

fn collect_grok_updates_files() -> Vec<PathBuf> {
    let mut files = Vec::new();
    let config_dir = get_grok_config_dir();
    for root_name in ["sessions", "archived_sessions"] {
        collect_files_named(&config_dir.join(root_name), "updates.jsonl", &mut files);
    }
    files.sort();
    files.dedup();
    files
}

fn collect_files_named(root: &Path, name: &str, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files_named(&path, name, files);
        } else if path.file_name().and_then(|value| value.to_str()) == Some(name) {
            files.push(path);
        }
    }
}

fn sync_single_grok_file(db: &Database, file_path: &Path) -> Result<SessionSyncResult, AppError> {
    let file_path_string = file_path.to_string_lossy().to_string();
    let metadata = fs::metadata(file_path)
        .map_err(|error| AppError::Config(format!("读取 Grok 会话文件元数据失败: {error}")))?;
    let file_modified = metadata_modified_nanos(&metadata);
    let (last_modified, _) = get_sync_state(db, &file_path_string)?;
    if file_modified <= last_modified {
        return Ok(SessionSyncResult::default());
    }

    let content = fs::read_to_string(file_path)
        .map_err(|error| AppError::Config(format!("读取 Grok 会话文件失败: {error}")))?;
    let events = parse_grok_usage_events(&content);
    let session_id = file_path
        .parent()
        .and_then(|path| path.file_name())
        .and_then(|value| value.to_str())
        .unwrap_or("unknown")
        .to_string();
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);

    let mut result = SessionSyncResult::default();
    let mut deferred = false;
    for (index, event) in events.iter().enumerate() {
        if now.saturating_sub(event.created_at) < SETTLE_WINDOW_SECONDS {
            deferred = true;
            break;
        }

        let proxy_active = {
            let conn = lock_conn!(db.usage_conn());
            has_recent_grokbuild_proxy_activity(&conn, event.created_at)?
        };
        let turn_key = if event.prompt_id.is_empty() {
            format!("idx{index}")
        } else {
            event.prompt_id.clone()
        };

        for (model, counters) in &event.per_model {
            if counters.is_zero() {
                continue;
            }
            if proxy_active {
                result.skipped += 1;
                continue;
            }

            let request_id = format!("grok_session:{session_id}:{turn_key}:{model}");
            match insert_grok_session_entry(
                db,
                &request_id,
                counters,
                event.cost_is_partial || counters.cost_partial,
                model,
                &session_id,
                event.created_at,
            ) {
                Ok(true) => result.imported += 1,
                Ok(false) => result.skipped += 1,
                Err(error) => {
                    log::warn!("[GROK-SYNC] 插入失败 ({request_id}): {error}");
                    result.skipped += 1;
                }
            }
        }
    }

    if !deferred {
        update_sync_state(db, &file_path_string, file_modified, events.len() as i64)?;
    }
    Ok(result)
}

fn parse_grok_usage_events(content: &str) -> Vec<GrokUsageEvent> {
    let mut events = Vec::new();
    for line in content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let Ok(record) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if record.get("method").and_then(Value::as_str) != Some("_x.ai/session/update") {
            continue;
        }
        let update = record.get("params").and_then(|params| params.get("update"));
        let kind = update
            .and_then(|value| value.get("sessionUpdate"))
            .and_then(Value::as_str);
        if kind.is_some() && kind != Some("turn_completed") {
            continue;
        }
        let Some(usage) = update
            .and_then(|value| value.get("usage"))
            .filter(|value| value.is_object())
        else {
            continue;
        };
        let Some(created_at) = parse_event_timestamp(record.get("timestamp")) else {
            continue;
        };

        let mut per_model: Vec<(String, GrokCounters)> = usage
            .get("modelUsage")
            .and_then(Value::as_object)
            .map(|models| {
                models
                    .iter()
                    .map(|(model, counters)| (model.clone(), parse_grok_counters(counters)))
                    .collect()
            })
            .unwrap_or_default();
        if per_model.is_empty() {
            per_model.push(("unknown".to_string(), parse_grok_counters(usage)));
        }
        per_model.sort_by(|left, right| left.0.cmp(&right.0));
        events.push(GrokUsageEvent {
            created_at,
            prompt_id: update
                .and_then(|value| value.get("prompt_id"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            cost_is_partial: usage
                .get("costIsPartial")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            per_model,
        });
    }
    events
}

fn parse_grok_counters(value: &Value) -> GrokCounters {
    let get = |key: &str| value.get(key).and_then(Value::as_u64).unwrap_or(0);
    GrokCounters {
        input: get("inputTokens"),
        output: get("outputTokens"),
        cached: get("cachedReadTokens"),
        api_ms: get("apiDurationMs"),
        cost_ticks: get("costUsdTicks"),
        cost_partial: value
            .get("costIsPartial")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    }
}

fn parse_event_timestamp(value: Option<&Value>) -> Option<i64> {
    let value = value?;
    if let Some(timestamp) = value.as_i64() {
        return Some(if timestamp > 100_000_000_000 {
            timestamp / 1000
        } else {
            timestamp
        });
    }
    value
        .as_str()
        .and_then(|timestamp| chrono::DateTime::parse_from_rfc3339(timestamp).ok())
        .map(|timestamp| timestamp.timestamp())
}

fn insert_grok_session_entry(
    db: &Database,
    request_id: &str,
    counters: &GrokCounters,
    cost_is_partial: bool,
    model: &str,
    session_id: &str,
    created_at: i64,
) -> Result<bool, AppError> {
    let conn = lock_conn!(db.usage_conn());
    let clamp = |value: u64| value.min(u32::MAX as u64) as u32;
    let usage = TokenUsage {
        input_tokens: clamp(counters.input),
        output_tokens: clamp(counters.output),
        cache_read_tokens: clamp(counters.cached),
        cache_creation_tokens: 0,
        model: Some(model.to_string()),
        message_id: None,
    };
    let pricing = find_model_pricing(&conn, model);
    let reported_cost = counters.reported_cost_usd();
    let (input_cost, output_cost, cache_read_cost, cache_creation_cost, total_cost) = match pricing
    {
        Some(pricing) => {
            let calculated =
                CostCalculator::calculate_for_app("grokbuild", &usage, &pricing, Decimal::from(1));
            let total = if !cost_is_partial {
                reported_cost.unwrap_or(calculated.total_cost)
            } else {
                calculated.total_cost
            };
            (
                calculated.input_cost.to_string(),
                calculated.output_cost.to_string(),
                calculated.cache_read_cost.to_string(),
                calculated.cache_creation_cost.to_string(),
                total.to_string(),
            )
        }
        None => (
            "0".to_string(),
            "0".to_string(),
            "0".to_string(),
            "0".to_string(),
            reported_cost
                .map(|cost| cost.to_string())
                .unwrap_or_else(|| "0".to_string()),
        ),
    };

    let inserted = conn
        .execute(
            "INSERT INTO proxy_request_logs (
                request_id, provider_id, app_type, model, request_model,
                input_tokens, output_tokens, cache_read_tokens, cache_creation_tokens,
                input_cost_usd, output_cost_usd, cache_read_cost_usd, cache_creation_cost_usd, total_cost_usd,
                latency_ms, first_token_ms, status_code, error_message, session_id,
                provider_type, is_streaming, cost_multiplier, created_at, data_source
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24)
            ON CONFLICT(request_id) DO UPDATE SET
                model = excluded.model,
                input_tokens = excluded.input_tokens,
                output_tokens = excluded.output_tokens,
                cache_read_tokens = excluded.cache_read_tokens,
                input_cost_usd = excluded.input_cost_usd,
                output_cost_usd = excluded.output_cost_usd,
                cache_read_cost_usd = excluded.cache_read_cost_usd,
                cache_creation_cost_usd = excluded.cache_creation_cost_usd,
                total_cost_usd = excluded.total_cost_usd,
                latency_ms = excluded.latency_ms
            WHERE proxy_request_logs.data_source = 'grok_session'",
            rusqlite::params![
                request_id,
                "_grok_session",
                "grokbuild",
                model,
                model,
                usage.input_tokens,
                usage.output_tokens,
                usage.cache_read_tokens,
                0i64,
                input_cost,
                output_cost,
                cache_read_cost,
                cache_creation_cost,
                total_cost,
                counters.api_ms.min(i64::MAX as u64) as i64,
                Option::<i64>::None,
                200i64,
                Option::<String>::None,
                session_id,
                Some("grok_session"),
                1i64,
                "1.0",
                created_at,
                "grok_session",
            ],
        )
        .map_err(|error| AppError::Database(format!("插入 Grok Build 会话日志失败: {error}")))?;

    if inserted > 0 {
        crate::usage_events::notify_log_recorded();
    }
    Ok(inserted > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_turn_completed_and_ignores_noise() {
        let content = r#"
{"method":"other","timestamp":1700000000}
{"method":"_x.ai/session/update","timestamp":1700000001,"params":{"update":{"sessionUpdate":"agent_message_chunk","usage":{"inputTokens":99}}}}
{"method":"_x.ai/session/update","timestamp":1700000002,"params":{"update":{"sessionUpdate":"turn_completed","prompt_id":"p1","usage":{"modelUsage":{"grok-4.5":{"inputTokens":100,"outputTokens":20,"cachedReadTokens":8,"apiDurationMs":500,"costUsdTicks":1234}}}}}}
"#;
        let events = parse_grok_usage_events(content);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].created_at, 1_700_000_002);
        assert_eq!(events[0].prompt_id, "p1");
        assert_eq!(events[0].per_model[0].0, "grok-4.5");
        assert_eq!(events[0].per_model[0].1.input, 100);
        assert_eq!(events[0].per_model[0].1.cached, 8);
    }

    #[test]
    fn falls_back_to_top_level_usage_when_model_usage_is_missing() {
        let content = r#"{"method":"_x.ai/session/update","timestamp":"2023-11-14T22:13:20Z","params":{"update":{"sessionUpdate":"turn_completed","usage":{"inputTokens":10,"outputTokens":5,"cachedReadTokens":2}}}}"#;
        let events = parse_grok_usage_events(content);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].per_model[0].0, "unknown");
        assert_eq!(events[0].per_model[0].1.output, 5);
    }

    #[test]
    fn normalizes_millisecond_timestamp() {
        let value = serde_json::json!(1_700_000_000_000i64);
        assert_eq!(parse_event_timestamp(Some(&value)), Some(1_700_000_000));
    }

    #[test]
    fn converts_cost_ticks_to_usd() {
        let counters = GrokCounters {
            cost_ticks: 10_000_000_000,
            ..Default::default()
        };
        assert_eq!(counters.reported_cost_usd(), Some(Decimal::ONE));
    }
}
