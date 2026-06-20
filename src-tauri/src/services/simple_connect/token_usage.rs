//! Simple Connect 代理 Token 计数（只读聚合，Phase 2）

use serde::Serialize;
use std::sync::Mutex;
use std::sync::OnceLock;

#[derive(Debug, Clone, Default, Serialize)]
pub struct ScTokenUsage {
    pub session_input: u64,
    pub session_output: u64,
    pub session_cache_read: u64,
    pub total_input: u64,
    pub total_output: u64,
    pub total_cache_read: u64,
}

fn store() -> &'static Mutex<ScTokenUsage> {
    static STORE: OnceLock<Mutex<ScTokenUsage>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(ScTokenUsage::default()))
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
