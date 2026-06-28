//! Provider ↔ Keychain 胶水层
//!
//! 在 `ProviderService` 写入 DB 之前，把 `settings_config` 里的明文 API key
//! 迁移到 OS keychain（复用 [`crate::keychain`]），DB 只存 `keychain://ref/`
//! 占位符。在消费 `settings_config` 的出口（write_live / proxy / usage），
//! 再把占位符解析回明文。
//!
//! 设计原则：
//! - 复用现有 `keychain.rs` 底层（store/get/delete secret），不新建存储。
//! - 不改 provider 数据表 schema —— 占位符是普通字符串，落库方式不变。
//! - 配置驱动：遍历 `settings_config` JSON 树，对"已知凭证字段名"的字符串值
//!   做迁移/解析，避免硬编码各 app 的路径。

use crate::app_config::AppType;
use crate::error::AppError;
use crate::keychain;
use crate::provider::Provider;
use serde_json::Value;

/// 已知的凭证字段名集合。
///
/// 出现这些名字的字段，其字符串值会被当作 secret 处理。涵盖所有 app 的
/// key 字段位置（env.* / auth.* / options.* / 顶层）。
const KNOWN_KEY_FIELDS: &[&str] = &[
    // Claude / ClaudeDesktop (env.*)
    "ANTHROPIC_AUTH_TOKEN",
    "ANTHROPIC_API_KEY",
    "OPENROUTER_API_KEY",
    "OPENAI_API_KEY",
    "GEMINI_API_KEY",
    // Codex (auth.*)
    // OPENAI_API_KEY 已包含
    // Gemini (env.*)
    "GOOGLE_API_KEY",
    // OpenClaw (顶层 apiKey) / OpenCode (options.apiKey)
    "apiKey",
    // Hermes (顶层 api_key)
    "api_key",
];

fn is_known_key_field(field: &str) -> bool {
    KNOWN_KEY_FIELDS.iter().any(|f| *f == field)
}

// ─── 写入路径：明文 → keychain 占位符 ─────────────────────────

/// 把 `provider.settings_config` 里的明文 key 迁移到 keychain，原位替换为
/// `keychain://ref/` 占位符。
///
/// 在 [`crate::services::ProviderService::add`] / `update` 调用 `save_provider`
/// 之前调用。幂等：已经是占位符或空字符串的值不变。
pub fn migrate_provider_settings_to_keychain(
    provider: &mut Provider,
    app_type: &AppType,
) -> Result<(), AppError> {
    migrate_value_in_place(
        &mut provider.settings_config,
        &provider.id,
        app_type.as_str(),
    )
}

fn migrate_value_in_place(
    value: &mut Value,
    provider_id: &str,
    app_type: &str,
) -> Result<(), AppError> {
    if let Value::Object(map) = value {
        for (key, val) in map.iter_mut() {
            // 先递归处理子对象（env / auth / options 等），再判断当前字段。
            // 这样无论 key 字段嵌在哪一层都能覆盖。
            migrate_value_in_place(val, provider_id, app_type)?;

            if is_known_key_field(key) {
                if let Some(s) = val.as_str() {
                    if !s.is_empty() && !keychain::is_keychain_ref(s) {
                        let entry_key =
                            keychain::provider_field_entry_key(provider_id, app_type, key);
                        keychain::store_secret(&entry_key, s)?;
                        *val = Value::String(keychain::make_keychain_ref(&entry_key));
                    }
                }
            }
        }
    }
    Ok(())
}

// ─── 读取路径：keychain 占位符 → 明文 ─────────────────────────

/// 返回 `provider` 的一个副本，其中 `settings_config` 里的 `keychain://ref/`
/// 占位符已被解析回明文。
///
/// 供消费 settings_config 的出口调用（write_live / proxy extract_key / usage
/// 查询）。解析失败（如跨设备同步来的占位符在本机 keychain 无对应条目）时
/// 保留占位符并记 warning，不中断流程。
pub fn resolve_provider_settings_for_use(
    provider: &Provider,
    _app_type: &AppType,
) -> Result<Provider, AppError> {
    let mut resolved = provider.clone();
    resolve_value_in_place(&mut resolved.settings_config)?;
    Ok(resolved)
}

/// 就地解析 `settings_config` 里的所有 keychain 占位符。
///
/// 与 [`resolve_provider_settings_for_use`] 的区别：直接修改传入的 provider，
/// 避免克隆。用于已经在调用方克隆过的场景。
pub fn resolve_settings_in_place(settings: &mut Value) -> Result<(), AppError> {
    resolve_value_in_place(settings)
}

fn resolve_value_in_place(value: &mut Value) -> Result<(), AppError> {
    if let Value::Object(map) = value {
        for (_key, val) in map.iter_mut() {
            resolve_value_in_place(val)?;

            if let Some(s) = val.as_str() {
                if keychain::is_keychain_ref(s) {
                    match keychain::resolve_value(s) {
                        Ok(plaintext) => {
                            *val = Value::String(plaintext);
                        }
                        Err(e) => {
                            // 跨设备同步等场景下 keychain 可能无对应条目；
                            // 保留占位符，仅告警，避免中断整个操作。
                            log::warn!(
                                "Failed to resolve keychain ref, leaving placeholder: {e}"
                            );
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

// ─── 删除路径：清理 keychain 条目 ─────────────────────────────

/// 删除 `provider` 在 keychain 中的所有凭证条目。
///
/// 在 [`crate::services::ProviderService::delete`] 调用 `delete_provider`
/// 之后调用。遍历 `settings_config`，凡是值为 `keychain://ref/` 的都删除
/// 对应 entry。
pub fn delete_provider_keys_from_keychain(
    provider: &Provider,
    _app_type: &AppType,
) -> Result<(), AppError> {
    delete_keys_in_value(&provider.settings_config)
}

fn delete_keys_in_value(value: &Value) -> Result<(), AppError> {
    if let Value::Object(map) = value {
        for (_key, val) in map {
            if let Some(s) = val.as_str() {
                if keychain::is_keychain_ref(s) {
                    if let Some(entry_key) = keychain::extract_ref_key(s) {
                        // delete_secret 对 NoEntry 容错，安全忽略。
                        let _ = keychain::delete_secret(entry_key);
                    }
                }
            }
            delete_keys_in_value(val)?;
        }
    }
    Ok(())
}

// ─── 启动时存量迁移 ───────────────────────────────────────────

/// 判断 `settings_config` 是否含有尚未迁移到 keychain 的明文 key。
fn has_plaintext_key(settings: &Value) -> bool {
    if let Value::Object(map) = settings {
        for (key, val) in map {
            if has_plaintext_key(val) {
                return true;
            }
            if is_known_key_field(key) {
                if let Some(s) = val.as_str() {
                    if !s.is_empty() && !keychain::is_keychain_ref(s) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// 启动时一次性迁移：把 DB 里所有 provider 的明文 key 迁移到 keychain。
///
/// 遍历所有 [`AppType`]，对每个 provider 检查是否含明文 key，有则迁移并回写
/// DB。幂等：重复调用不会重复迁移。返回迁移的 provider 数量。
pub fn migrate_all_providers_if_needed(
    db: &crate::database::Database,
) -> Result<usize, AppError> {
    let mut migrated = 0;
    for app_type in AppType::all() {
        let providers = match db.get_all_providers(app_type.as_str()) {
            Ok(p) => p,
            Err(e) => {
                log::warn!(
                    "Skipping keychain migration for {}: failed to read providers: {e}",
                    app_type.as_str()
                );
                continue;
            }
        };

        for (id, mut provider) in providers {
            if has_plaintext_key(&provider.settings_config) {
                if let Err(e) =
                    migrate_provider_settings_to_keychain(&mut provider, &app_type)
                {
                    log::warn!(
                        "Failed to migrate keychain for provider '{}' ({}): {e}",
                        id,
                        app_type.as_str()
                    );
                    continue;
                }
                if let Err(e) = db.save_provider(app_type.as_str(), &provider) {
                    log::warn!(
                        "Failed to save provider '{}' after keychain migration ({}): {e}",
                        id,
                        app_type.as_str()
                    );
                    continue;
                }
                migrated += 1;
                log::info!(
                    "✓ Migrated plaintext key to keychain for provider '{}' ({})",
                    id,
                    app_type.as_str()
                );
            }
        }
    }
    Ok(migrated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_is_known_key_field() {
        assert!(is_known_key_field("ANTHROPIC_AUTH_TOKEN"));
        assert!(is_known_key_field("ANTHROPIC_API_KEY"));
        assert!(is_known_key_field("apiKey"));
        assert!(is_known_key_field("api_key"));
        assert!(!is_known_key_field("ANTHROPIC_BASE_URL"));
        assert!(!is_known_key_field("model"));
    }

    #[test]
    fn test_has_plaintext_key_detects_env_token() {
        let settings = json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://api.deepseek.com/anthropic",
                "ANTHROPIC_AUTH_TOKEN": "sk-plaintext"
            }
        });
        assert!(has_plaintext_key(&settings));
    }

    #[test]
    fn test_has_plaintext_key_ignores_placeholder() {
        let settings = json!({
            "env": {
                "ANTHROPIC_AUTH_TOKEN": "keychain://ref/some-id/claude/ANTHROPIC_AUTH_TOKEN"
            }
        });
        assert!(!has_plaintext_key(&settings));
    }

    #[test]
    fn test_has_plaintext_key_ignores_empty() {
        let settings = json!({
            "env": {
                "ANTHROPIC_AUTH_TOKEN": ""
            }
        });
        assert!(!has_plaintext_key(&settings));
    }

    #[test]
    fn test_has_plaintext_key_detects_top_level_apikey() {
        let settings = json!({
            "baseUrl": "https://example.com",
            "apiKey": "sk-top-level"
        });
        assert!(has_plaintext_key(&settings));
    }

    #[test]
    fn test_has_plaintext_key_detects_nested_options_apikey() {
        let settings = json!({
            "options": {
                "baseURL": "https://example.com",
                "apiKey": "sk-nested"
            }
        });
        assert!(has_plaintext_key(&settings));
    }

    #[test]
    fn test_has_plaintext_key_ignores_non_key_fields() {
        let settings = json!({
            "env": {
                "ANTHROPIC_BASE_URL": "https://api.deepseek.com",
                "ANTHROPIC_MODEL": "deepseek-chat"
            }
        });
        assert!(!has_plaintext_key(&settings));
    }
}
