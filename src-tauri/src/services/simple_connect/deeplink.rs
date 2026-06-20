//! beeapi 兼容 Deep Link 导入 → Keychain（Phase 2）

use crate::error::AppError;
use crate::services::simple_connect::key_store::{
    get_api_key, get_primary_key, key_hint, store_api_key, store_primary_key,
};
use crate::services::simple_connect::state::{load_state, save_state, PoolKeyMeta, SimpleConnectState};
use crate::services::simple_connect::verify::verify_api_key;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimpleConnectImportPayload {
    pub keys: Vec<String>,
    pub label: Option<String>,
    pub model: Option<String>,
    pub pool_enabled: Option<bool>,
    pub supplier_id: Option<String>,
    pub source_url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimpleConnectImportResult {
    pub keys_added: usize,
    pub duplicates: usize,
    pub primary_key_hint: Option<String>,
    pub model: Option<String>,
    pub pool_enabled: Option<bool>,
    pub supplier_id: String,
}

pub fn is_simple_connect_import_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.starts_with("beeapi://")
        || lower.starts_with("beeapi-switch://")
        || lower.contains("simple-connect/import")
        || lower.contains("simpleconnect/import")
}

pub fn try_parse_url(raw: &str) -> Result<SimpleConnectImportPayload, AppError> {
    if !is_simple_connect_import_url(raw) {
        return Err(AppError::Message("非 Simple Connect 导入链接".into()));
    }
    let url = Url::parse(raw.trim())
        .map_err(|e| AppError::Message(format!("URL 解析失败: {e}")))?;

    let mut keys = Vec::new();
    let mut label: Option<String> = None;
    let mut model: Option<String> = None;
    let mut pool_enabled: Option<bool> = None;
    let mut supplier_id: Option<String> = None;

    for (k, v) in url.query_pairs() {
        match k.as_ref() {
            "key" | "api_key" | "apikey" | "token" | "secret" => {
                keys.push(v.into_owned());
            }
            "keys" | "api_keys" => {
                keys.extend(
                    v.split([',', ';', '\n', ' ', '\r'])
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string),
                );
            }
            "label" | "name" | "key_name" | "keyName" | "title" => {
                label = Some(v.into_owned());
            }
            "model" => model = Some(v.into_owned()),
            "supplier" | "supplier_id" => supplier_id = Some(v.into_owned()),
            "pool" | "pool_enabled" => {
                let v = v.to_ascii_lowercase();
                pool_enabled = match v.as_str() {
                    "1" | "true" | "yes" | "on" => Some(true),
                    "0" | "false" | "no" | "off" => Some(false),
                    _ => None,
                };
            }
            _ => {}
        }
    }

    let path = url.path().trim_matches('/');
    if !path.is_empty() && path != "import" && path != "add" {
        if let Some(last) = path.rsplit('/').next() {
            if last != "import" && last != "add" && is_probable_api_key(last) {
                keys.push(last.to_string());
            }
        }
    }
    if let Some(host) = url.host_str() {
        if host != "import"
            && host != "add"
            && host != "simple-connect"
            && host != "simpleconnect"
            && is_probable_api_key(host)
        {
            keys.push(host.to_string());
        }
    }

    keys.retain(|k| is_probable_api_key(k));
    keys.sort();
    keys.dedup();

    if keys.is_empty() {
        return Err(AppError::Message("链接中未找到有效 API Key".into()));
    }

    Ok(SimpleConnectImportPayload {
        keys,
        label,
        model,
        pool_enabled,
        supplier_id,
        source_url: raw.trim().to_string(),
    })
}

fn is_probable_api_key(raw: &str) -> bool {
    let s = raw.trim();
    s.len() >= 8
        && s.chars()
            .all(|c| !c.is_control() && !c.is_whitespace())
}

fn normalize_secret(raw: &str) -> String {
    raw.trim().to_string()
}

fn secret_exists(state: &SimpleConnectState, supplier_id: &str, secret: &str) -> Result<bool, AppError> {
    if let Some(primary) = get_primary_key(supplier_id)? {
        if primary == secret {
            return Ok(true);
        }
    }
    for meta in &state.pool_keys {
        if let Some(existing) = get_api_key(supplier_id, &meta.id)? {
            if existing == secret {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

pub async fn import_keys(
    payload: &SimpleConnectImportPayload,
    skip_verify: bool,
) -> Result<SimpleConnectImportResult, AppError> {
    let mut state = load_state()?;

    if !state.deeplink_import_enabled {
        return Err(AppError::Message(
            "Deep Link 导入已在设置中禁用".into(),
        ));
    }

    let supplier_id = payload
        .supplier_id
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(state.supplier_id.as_str())
        .to_string();

    if let Some(enabled) = payload.pool_enabled {
        state.pool_enabled = enabled;
    }
    if let Some(model) = payload.model.as_deref().filter(|m| !m.is_empty()) {
        state.last_model = Some(model.to_string());
    }

    let custom_base = state.custom_openai_base.as_deref();
    let mut added = 0usize;
    let mut duplicates = 0usize;
    let mut primary_hint: Option<String> = None;

    for (idx, raw) in payload.keys.iter().enumerate() {
        let _ = idx;
        let secret = normalize_secret(raw);
        if secret_exists(&state, &supplier_id, &secret)? {
            duplicates += 1;
            continue;
        }

        if !skip_verify && state.require_key_verify {
            let verify = verify_api_key(&supplier_id, &secret, custom_base).await?;
            if !verify.ok {
                return Err(AppError::Message(
                    verify
                        .error
                        .unwrap_or_else(|| "Key 校验失败".into()),
                ));
            }
        }

        let needs_primary = get_primary_key(&supplier_id)?.is_none();
        if needs_primary {
            store_primary_key(&supplier_id, &secret)?;
            let hint = key_hint(&secret);
            primary_hint = Some(hint);
            if !state.pool_keys.iter().any(|k| k.id == "primary") {
                state.pool_keys.push(PoolKeyMeta {
                    id: "primary".into(),
                    label: payload
                        .label
                        .clone()
                        .unwrap_or_else(|| "主 Key".into()),
                    weight: 1,
                    enabled: true,
                });
            }
        } else {
            let id = format!("k{}", uuid::Uuid::new_v4().simple());
            store_api_key(&supplier_id, &id, &secret)?;
            let label = payload
                .label
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .map(|s| {
                    if added == 0 {
                        s.trim().to_string()
                    } else {
                        format!("{}-{}", s.trim(), added + 1)
                    }
                })
                .unwrap_or_else(|| format!("导入 Key {}", state.pool_keys.len() + 1));
            state.pool_keys.push(PoolKeyMeta {
                id,
                label,
                weight: 1,
                enabled: true,
            });
        }
        added += 1;
    }

    if added == 0 && duplicates > 0 {
        return Err(AppError::Message("所有 Key 均已存在，未导入新 Key".into()));
    }

    state.supplier_id = supplier_id.clone();
    save_state(&state)?;

    Ok(SimpleConnectImportResult {
        keys_added: added,
        duplicates,
        primary_key_hint: primary_hint.or_else(|| {
            get_primary_key(&supplier_id)
                .ok()
                .flatten()
                .map(|k| key_hint(&k))
        }),
        model: payload.model.clone(),
        pool_enabled: payload.pool_enabled,
        supplier_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_beeapi_import_query() {
        let p = try_parse_url("beeapi://import?key=sk-test12345678&name=主密钥").unwrap();
        assert_eq!(p.keys, vec!["sk-test12345678"]);
        assert_eq!(p.label.as_deref(), Some("主密钥"));
    }

    #[test]
    fn parses_compact_host_key() {
        let p = try_parse_url("beeapi-switch://sk-compact-key-abcdefgh").unwrap();
        assert!(p.keys.iter().any(|k| k.contains("sk-compact")));
    }

    #[test]
    fn parses_opensunstar_simple_connect_path() {
        let p = try_parse_url(
            "OpenSunstar://simple-connect/import?key=sk-opensunstar99&supplier=deepseek",
        )
        .unwrap();
        assert_eq!(p.supplier_id.as_deref(), Some("deepseek"));
    }
}
