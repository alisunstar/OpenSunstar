//! AI 洞察提供方配置 — Keychain 存储密钥，settings 表存储非敏感元数据

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::ai::types::AIProviderConfig;
use crate::keychain;
use crate::store::AppState;

const PROVIDER_SETTING_KEY: &str = "ai_insight_provider";
const GLM_META_KEY: &str = "ai_insight_glm_meta";
const CUSTOM_META_KEY: &str = "ai_insight_custom_meta";

const KEY_DEEPSEEK: &str = "ai-insight/deepseek";
const KEY_GLM: &str = "ai-insight/glm";
const KEY_CUSTOM: &str = "ai-insight/custom";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderMeta {
    #[serde(default = "default_glm_url")]
    api_url: String,
    #[serde(default = "default_glm_model")]
    model: String,
}

fn default_glm_url() -> String {
    "https://open.bigmodel.cn/api/coding/paas/v4/chat/completions".to_string()
}

fn default_glm_model() -> String {
    "GLM-5.1".to_string()
}

fn default_custom_url() -> String {
    "https://api.openai.com/v1/chat/completions".to_string()
}

fn default_custom_model() -> String {
    "gpt-4o".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiInsightProviderSettingsView {
    pub provider: String,
    pub deepseek_configured: bool,
    pub glm_api_url: String,
    pub glm_model: String,
    pub glm_configured: bool,
    pub custom_api_url: String,
    pub custom_model: String,
    pub custom_configured: bool,
}

fn secret_configured(entry_key: &str) -> bool {
    keychain::get_secret(entry_key)
        .ok()
        .flatten()
        .is_some_and(|v| !v.trim().is_empty())
}

fn read_meta_from_db(
    db: &crate::database::Database,
    key: &str,
    default_url: &str,
    default_model: &str,
) -> (String, String) {
    match db.get_setting(key) {
        Ok(Some(raw)) => {
            if let Ok(meta) = serde_json::from_str::<ProviderMeta>(&raw) {
                return (meta.api_url, meta.model);
            }
        }
        _ => {}
    }
    (default_url.to_string(), default_model.to_string())
}

#[tauri::command]
pub fn get_ai_insight_provider_settings(
    state: State<'_, AppState>,
) -> Result<AiInsightProviderSettingsView, String> {
    let provider = state
        .db
        .get_setting(PROVIDER_SETTING_KEY)
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| "deepseek".to_string());

    let (glm_api_url, glm_model) =
        read_meta_from_db(&state.db, GLM_META_KEY, &default_glm_url(), &default_glm_model());
    let (custom_api_url, custom_model) = read_meta_from_db(
        &state.db,
        CUSTOM_META_KEY,
        &default_custom_url(),
        &default_custom_model(),
    );

    Ok(AiInsightProviderSettingsView {
        provider,
        deepseek_configured: secret_configured(KEY_DEEPSEEK),
        glm_api_url,
        glm_model,
        glm_configured: secret_configured(KEY_GLM),
        custom_api_url,
        custom_model,
        custom_configured: secret_configured(KEY_CUSTOM),
    })
}

#[tauri::command]
pub fn save_ai_insight_provider_choice(
    state: State<'_, AppState>,
    provider: String,
) -> Result<(), String> {
    if !matches!(provider.as_str(), "deepseek" | "glm" | "custom") {
        return Err(format!("无效的 AI 提供方: {provider}"));
    }
    state
        .db
        .set_setting(PROVIDER_SETTING_KEY, &provider)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_ai_insight_deepseek_key(
    api_key: String,
) -> Result<bool, String> {
    let trimmed = api_key.trim();
    if trimmed.is_empty() {
        keychain::delete_secret(KEY_DEEPSEEK).map_err(|e| e.to_string())?;
        return Ok(false);
    }
    keychain::store_secret(KEY_DEEPSEEK, trimmed).map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub fn save_ai_insight_glm_settings(
    state: State<'_, AppState>,
    api_key: String,
    api_url: String,
    model: String,
) -> Result<bool, String> {
    let meta = ProviderMeta {
        api_url: if api_url.trim().is_empty() {
            default_glm_url()
        } else {
            api_url.trim().to_string()
        },
        model: if model.trim().is_empty() {
            default_glm_model()
        } else {
            model.trim().to_string()
        },
    };
    state
        .db
        .set_setting(GLM_META_KEY, &serde_json::to_string(&meta).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;

    let trimmed = api_key.trim();
    if trimmed.is_empty() {
        keychain::delete_secret(KEY_GLM).map_err(|e| e.to_string())?;
        return Ok(false);
    }
    keychain::store_secret(KEY_GLM, trimmed).map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub fn save_ai_insight_custom_settings(
    state: State<'_, AppState>,
    api_key: String,
    api_url: String,
    model: String,
) -> Result<bool, String> {
    let meta = ProviderMeta {
        api_url: if api_url.trim().is_empty() {
            default_custom_url()
        } else {
            api_url.trim().to_string()
        },
        model: if model.trim().is_empty() {
            default_custom_model()
        } else {
            model.trim().to_string()
        },
    };
    state
        .db
        .set_setting(CUSTOM_META_KEY, &serde_json::to_string(&meta).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;

    let trimmed = api_key.trim();
    if trimmed.is_empty() {
        keychain::delete_secret(KEY_CUSTOM).map_err(|e| e.to_string())?;
        return Ok(false);
    }
    keychain::store_secret(KEY_CUSTOM, trimmed).map_err(|e| e.to_string())?;
    Ok(true)
}

/// 构建 AI 洞察调用所需的完整配置（含 Keychain 密钥）
#[tauri::command]
pub fn build_ai_insight_provider_config(
    state: State<'_, AppState>,
) -> Result<Option<AIProviderConfig>, String> {
    let provider = state
        .db
        .get_setting(PROVIDER_SETTING_KEY)
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| "deepseek".to_string());

    match provider.as_str() {
        "deepseek" => {
            let key = keychain::get_secret(KEY_DEEPSEEK)
                .map_err(|e| e.to_string())?
                .unwrap_or_default();
            if key.trim().is_empty() {
                return Ok(None);
            }
            Ok(Some(AIProviderConfig {
                provider: "deepseek".to_string(),
                api_key: key,
                api_url: "https://api.deepseek.com/v1/chat/completions".to_string(),
                model: "deepseek-chat".to_string(),
            }))
        }
        "glm" => {
            let key = keychain::get_secret(KEY_GLM)
                .map_err(|e| e.to_string())?
                .unwrap_or_default();
            if key.trim().is_empty() {
                return Ok(None);
            }
            let (api_url, model) =
                read_meta_from_db(&state.db, GLM_META_KEY, &default_glm_url(), &default_glm_model());
            Ok(Some(AIProviderConfig {
                provider: "glm".to_string(),
                api_key: key,
                api_url,
                model,
            }))
        }
        "custom" => {
            let key = keychain::get_secret(KEY_CUSTOM)
                .map_err(|e| e.to_string())?
                .unwrap_or_default();
            if key.trim().is_empty() {
                return Ok(None);
            }
            let (api_url, model) = read_meta_from_db(
                &state.db,
                CUSTOM_META_KEY,
                &default_custom_url(),
                &default_custom_model(),
            );
            Ok(Some(AIProviderConfig {
                provider: "custom".to_string(),
                api_key: key,
                api_url,
                model,
            }))
        }
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_meta_defaults() {
        assert!(default_glm_url().contains("bigmodel"));
        assert_eq!(default_glm_model(), "GLM-5.1");
    }
}
