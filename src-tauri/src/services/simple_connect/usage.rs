//! 本地 CLI 会话用量只读扫描（beeapi-switch usage.rs 精简移植）

use crate::error::AppError;
use crate::services::simple_connect::token_usage;
use serde::Serialize;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Clone, Serialize)]
pub struct UsageRecord {
    pub ts: i64,
    pub tool: String,
    pub session: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
}

#[derive(Serialize)]
pub struct ToolUsageBreakdown {
    pub tool: String,
    pub records: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Serialize)]
pub struct SimpleConnectUsageSummary {
    pub files_scanned: usize,
    pub record_count: usize,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cache_read_tokens: u64,
    pub by_tool: Vec<ToolUsageBreakdown>,
    pub recent_records: Vec<UsageRecord>,
    pub proxy_session_input: u64,
    pub proxy_session_output: u64,
    pub proxy_total_input: u64,
    pub proxy_total_output: u64,
    pub proxy_port: u16,
    pub note: String,
}

pub fn scan_local_sessions() -> Result<(usize, Vec<UsageRecord>), AppError> {
    let home = dirs::home_dir().ok_or_else(|| AppError::Message("无法定位用户主目录".into()))?;
    let mut records = Vec::new();
    let mut files_scanned = 0usize;

    let claude_dir = home.join(".claude").join("projects");
    if claude_dir.exists() {
        for path in walk_files(&claude_dir)? {
            if !is_jsonl_or_json(&path) {
                continue;
            }
            files_scanned += 1;
            records.extend(scan_claude_file(&path));
        }
    }

    let codex_dir = codex_sessions_dir(&home);
    if codex_dir.exists() {
        for path in walk_files(&codex_dir)? {
            if !is_jsonl_or_json(&path) {
                continue;
            }
            files_scanned += 1;
            records.extend(scan_codex_file(&path));
        }
    }

    records.sort_by(|a, b| b.ts.cmp(&a.ts));
    Ok((files_scanned, records))
}

pub fn build_usage_summary() -> Result<SimpleConnectUsageSummary, AppError> {
    let (files_scanned, records) = scan_local_sessions()?;
    let mut by_tool: std::collections::HashMap<String, ToolUsageBreakdown> =
        std::collections::HashMap::new();

    let mut total_input = 0u64;
    let mut total_output = 0u64;
    let mut total_cache = 0u64;

    for r in &records {
        total_input += r.input_tokens;
        total_output += r.output_tokens;
        total_cache += r.cache_read_tokens;
        let entry = by_tool.entry(r.tool.clone()).or_insert(ToolUsageBreakdown {
            tool: r.tool.clone(),
            records: 0,
            input_tokens: 0,
            output_tokens: 0,
        });
        entry.records += 1;
        entry.input_tokens += r.input_tokens;
        entry.output_tokens += r.output_tokens;
    }

    let mut breakdown: Vec<_> = by_tool.into_values().collect();
    breakdown.sort_by(|a, b| b.input_tokens.cmp(&a.input_tokens));

    let proxy = token_usage::snapshot();

    Ok(SimpleConnectUsageSummary {
        files_scanned,
        record_count: records.len(),
        total_input_tokens: total_input,
        total_output_tokens: total_output,
        total_cache_read_tokens: total_cache,
        by_tool: breakdown,
        recent_records: records.into_iter().take(15).collect(),
        proxy_session_input: proxy.session_input,
        proxy_session_output: proxy.session_output,
        proxy_total_input: proxy.total_input,
        proxy_total_output: proxy.total_output,
        proxy_port: crate::services::simple_connect::proxy_poc::SPIKE_PROXY_PORT,
        note: "本地会话扫描 + Simple Connect 代理计数；只读，不写入 CLI 配置".into(),
    })
}

fn codex_sessions_dir(home: &Path) -> PathBuf {
    if let Ok(dir) = std::env::var("CODEX_HOME") {
        let p = PathBuf::from(dir);
        if p.exists() {
            return p.join("sessions");
        }
    }
    home.join(".codex").join("sessions")
}

fn walk_files(dir: &Path) -> Result<Vec<PathBuf>, AppError> {
    let mut out = Vec::new();
    walk_dir(dir, &mut out);
    Ok(out)
}

fn walk_dir(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_dir(&path, out);
        } else if path.is_file() {
            out.push(path);
        }
    }
}

fn is_jsonl_or_json(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|s| s.to_str()),
        Some("jsonl") | Some("json")
    )
}

fn scan_claude_file(path: &Path) -> Vec<UsageRecord> {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    let session = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("session")
        .to_string();
    let mut out = Vec::new();
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        let v: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let usage = match extract_usage_value(&v) {
            Some(u) => u,
            None => continue,
        };
        let tokens = extract_tokens(usage);
        if tokens.0 == 0 && tokens.1 == 0 {
            continue;
        }
        let ts = v
            .get("timestamp")
            .or_else(|| v.get("message").and_then(|m| m.get("timestamp")))
            .and_then(|t| t.as_str())
            .and_then(parse_ts)
            .unwrap_or(0);
        let model = v
            .get("message")
            .and_then(|m| m.get("model"))
            .and_then(|m| m.as_str())
            .or_else(|| usage.get("model").and_then(|m| m.as_str()))
            .unwrap_or("unknown")
            .to_string();
        out.push(UsageRecord {
            ts,
            tool: "claude-code".into(),
            session: session.clone(),
            model,
            input_tokens: tokens.0,
            output_tokens: tokens.1,
            cache_read_tokens: tokens.2,
        });
    }
    out
}

fn scan_codex_file(path: &Path) -> Vec<UsageRecord> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let session = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("session")
        .to_string();
    let v: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let payload = v.get("payload").unwrap_or(&v);
    let usage = match extract_codex_usage(payload) {
        Some(u) => u,
        None => return Vec::new(),
    };
    let tokens = extract_tokens(usage);
    if tokens.0 == 0 && tokens.1 == 0 {
        return Vec::new();
    }
    vec![UsageRecord {
        ts: 0,
        tool: "codex".into(),
        session,
        model: payload
            .get("model")
            .and_then(|m| m.as_str())
            .unwrap_or("unknown")
            .to_string(),
        input_tokens: tokens.0,
        output_tokens: tokens.1,
        cache_read_tokens: tokens.2,
    }]
}

fn extract_codex_usage(payload: &serde_json::Value) -> Option<&serde_json::Value> {
    payload
        .get("token_count")
        .and_then(|v| v.get("last_token_usage"))
        .or_else(|| extract_usage_value(payload))
}

fn extract_usage_value(value: &serde_json::Value) -> Option<&serde_json::Value> {
    value
        .get("usage")
        .or_else(|| value.get("message").and_then(|m| m.get("usage")))
}

fn extract_tokens(usage: &serde_json::Value) -> (u64, u64, u64) {
    let input = usage
        .get("input_tokens")
        .or_else(|| usage.get("prompt_tokens"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let output = usage
        .get("output_tokens")
        .or_else(|| usage.get("completion_tokens"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let cache = usage
        .get("cache_read_input_tokens")
        .or_else(|| usage.get("prompt_cache_hit_tokens"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    (input, output, cache)
}

fn parse_ts(s: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_runs_offline() {
        let summary = build_usage_summary().expect("summary");
        assert!(summary.note.contains("只读"));
    }
}
