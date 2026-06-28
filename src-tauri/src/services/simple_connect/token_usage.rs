//! Simple Connect 代理 Token 计数（只读聚合，Phase 2）

use serde::Serialize;
use std::sync::Mutex;
use std::sync::OnceLock;

#[derive(Debug, Clone, Default, Serialize, serde::Deserialize)]
pub struct ScTokenUsage {
    pub session_input: u64,
    pub session_output: u64,
    pub session_cache_read: u64,
    pub total_input: u64,
    pub total_output: u64,
    pub total_cache_read: u64,
}

fn usage_path() -> std::path::PathBuf {
    crate::config::get_app_config_dir().join("simple_connect_usage.json")
}

/// 仅持久化累计字段（total_*），session_* 每次启动归零
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct PersistedTotals {
    total_input: u64,
    total_output: u64,
    total_cache_read: u64,
}

fn load_persisted() -> ScTokenUsage {
    let path = usage_path();
    if !path.exists() {
        return ScTokenUsage::default();
    }
    match std::fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str::<PersistedTotals>(&text) {
            Ok(totals) => ScTokenUsage {
                total_input: totals.total_input,
                total_output: totals.total_output,
                total_cache_read: totals.total_cache_read,
                ..Default::default()
            },
            Err(_) => ScTokenUsage::default(),
        },
        Err(_) => ScTokenUsage::default(),
    }
}

fn persist(u: &ScTokenUsage) {
    let totals = PersistedTotals {
        total_input: u.total_input,
        total_output: u.total_output,
        total_cache_read: u.total_cache_read,
    };
    if let Ok(json) = serde_json::to_string_pretty(&totals) {
        let path = usage_path();
        let _ = std::fs::write(&path, json);
    }
}

fn store() -> &'static Mutex<ScTokenUsage> {
    static STORE: OnceLock<Mutex<ScTokenUsage>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(load_persisted()))
}

pub fn snapshot() -> ScTokenUsage {
    store().lock().unwrap_or_else(|e| e.into_inner()).clone()
}

pub fn reset_session() {
    if let Ok(mut u) = store().lock() {
        u.session_input = 0;
        u.session_output = 0;
        u.session_cache_read = 0;
    }
}

pub fn add_usage(input: u64, output: u64, cache_read: u64) {
    if input == 0 && output == 0 && cache_read == 0 {
        return;
    }
    if let Ok(mut u) = store().lock() {
        u.session_input += input;
        u.session_output += output;
        u.session_cache_read += cache_read;
        u.total_input += input;
        u.total_output += output;
        u.total_cache_read += cache_read;
        persist(&u);
    }
}

pub fn extract_usage_from_body(bytes: &[u8]) -> Option<(u64, u64, u64)> {
    let v: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    let usage = v.get("usage").or_else(|| {
        v.get("response")
            .and_then(|r| r.get("usage"))
    })?;
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
    let cache_read = usage
        .get("cache_read_input_tokens")
        .or_else(|| usage.get("prompt_cache_hit_tokens"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    if input == 0 && output == 0 && cache_read == 0 {
        return None;
    }
    Some((input, output, cache_read))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_openai_usage_shape() {
        let body = br#"{"usage":{"prompt_tokens":10,"completion_tokens":5}}"#;
        let (i, o, c) = extract_usage_from_body(body).unwrap();
        assert_eq!(i, 10);
        assert_eq!(o, 5);
        assert_eq!(c, 0);
    }
}
