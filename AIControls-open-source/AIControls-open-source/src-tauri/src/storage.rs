//! Local persistence for DeepSeek API key and AI‑assigned asset scenario labels.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::AppHandle;
use tauri::Manager;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeepseekSettingsPublic {
    pub api_key_configured: bool,
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct DeepseekSettingsFile {
    #[serde(default)]
    api_key: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct FloatBallPosition {
    pub x: i32,
    pub y: i32,
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(p) = path.parent() {
        fs::create_dir_all(p).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn app_local_dir(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_local_data_dir()
        .map_err(|e| format!("无法解析应用数据目录：{e}"))
}

fn deepseek_settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_local_dir(app)?.join("deepseek_settings.json"))
}

fn float_ball_position_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_local_dir(app)?.join("float_ball_position.json"))
}

fn scenario_map_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_local_dir(app)?.join("asset_scenarios.json"))
}

fn brief_map_path(app: &AppHandle, locale: &str) -> Result<PathBuf, String> {
    let key = match locale {
        "zh" => "asset_briefs_zh.json",
        "en" => "asset_briefs_en.json",
        _ => "asset_briefs_en.json",
    };
    Ok(app_local_dir(app)?.join(key))
}

pub fn get_deepseek_settings_public(app: &AppHandle) -> Result<DeepseekSettingsPublic, String> {
    let configured = load_deepseek_api_key(app)?
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    Ok(DeepseekSettingsPublic {
        api_key_configured: configured,
    })
}

pub fn load_deepseek_api_key(app: &AppHandle) -> Result<Option<String>, String> {
    let path = deepseek_settings_path(app)?;
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let file: DeepseekSettingsFile =
        serde_json::from_str(&text).map_err(|e| format!("读取 DeepSeek 配置失败：{e}"))?;
    let k = file.api_key.trim().to_string();
    if k.is_empty() {
        Ok(None)
    } else {
        Ok(Some(k))
    }
}

pub fn save_deepseek_api_key(app: &AppHandle, api_key: String) -> Result<(), String> {
    let path = deepseek_settings_path(app)?;
    ensure_parent(&path)?;
    let file = DeepseekSettingsFile {
        api_key: api_key.trim().to_string(),
    };
    let json = serde_json::to_string_pretty(&file).map_err(|e| format!("序列化配置失败：{e}"))?;
    fs::write(path, json).map_err(|e| format!("写入 DeepSeek 配置失败：{e}"))?;
    Ok(())
}

pub fn load_float_ball_position(app: &AppHandle) -> Result<Option<FloatBallPosition>, String> {
    let path = float_ball_position_path(app)?;
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let position: FloatBallPosition =
        serde_json::from_str(&text).map_err(|e| format!("读取悬浮球位置失败：{e}"))?;
    Ok(Some(position))
}

pub fn save_float_ball_position(
    app: &AppHandle,
    position: FloatBallPosition,
) -> Result<(), String> {
    let path = float_ball_position_path(app)?;
    ensure_parent(&path)?;
    let json = serde_json::to_string_pretty(&position)
        .map_err(|e| format!("序列化悬浮球位置失败：{e}"))?;
    fs::write(path, json).map_err(|e| format!("写入悬浮球位置失败：{e}"))?;
    Ok(())
}

pub fn load_scenario_map(app: &AppHandle) -> Result<HashMap<String, String>, String> {
    let path = scenario_map_path(app)?;
    if !path.is_file() {
        return Ok(HashMap::new());
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let v: HashMap<String, String> =
        serde_json::from_str(&text).map_err(|e| format!("读取分类缓存失败：{e}"))?;
    Ok(v)
}

pub fn merge_scenario_map(app: &AppHandle, delta: &HashMap<String, String>) -> Result<(), String> {
    if delta.is_empty() {
        return Ok(());
    }
    let mut m = load_scenario_map(app).unwrap_or_default();
    for (k, v) in delta {
        m.insert(k.clone(), v.clone());
    }
    save_scenario_map(app, &m)
}

pub fn load_brief_map(app: &AppHandle, locale: &str) -> Result<HashMap<String, String>, String> {
    let path = brief_map_path(app, locale)?;
    if !path.is_file() {
        // backward compatibility: legacy zh brief cache file
        if locale == "zh" {
            let legacy = app_local_dir(app)?.join("asset_briefs_zh.json");
            if legacy.is_file() {
                let text = fs::read_to_string(&legacy).map_err(|e| e.to_string())?;
                let v: HashMap<String, String> = serde_json::from_str(&text)
                    .map_err(|e| format!("读取缩略介绍缓存失败：{e}"))?;
                return Ok(v);
            }
        }
        return Ok(HashMap::new());
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let v: HashMap<String, String> =
        serde_json::from_str(&text).map_err(|e| format!("读取缩略介绍缓存失败：{e}"))?;
    Ok(v)
}

pub fn merge_brief_map(
    app: &AppHandle,
    locale: &str,
    delta: &HashMap<String, String>,
) -> Result<(), String> {
    if delta.is_empty() {
        return Ok(());
    }
    let mut m = load_brief_map(app, locale).unwrap_or_default();
    for (k, v) in delta {
        m.insert(k.clone(), v.clone());
    }
    save_brief_map(app, locale, &m)
}

fn save_scenario_map(app: &AppHandle, map: &HashMap<String, String>) -> Result<(), String> {
    let path = scenario_map_path(app)?;
    ensure_parent(&path)?;
    let json = serde_json::to_string_pretty(map).map_err(|e| format!("序列化分类缓存失败：{e}"))?;
    fs::write(path, json).map_err(|e| format!("写入分类缓存失败：{e}"))?;
    Ok(())
}

pub fn clear_scenario_map(app: &AppHandle) -> Result<(), String> {
    let path = scenario_map_path(app)?;
    if path.is_file() {
        fs::remove_file(&path).map_err(|e| format!("删除分类缓存失败：{e}"))?;
    }
    Ok(())
}

fn save_brief_map(
    app: &AppHandle,
    locale: &str,
    map: &HashMap<String, String>,
) -> Result<(), String> {
    let path = brief_map_path(app, locale)?;
    ensure_parent(&path)?;
    let json =
        serde_json::to_string_pretty(map).map_err(|e| format!("序列化缩略介绍缓存失败：{e}"))?;
    fs::write(path, json).map_err(|e| format!("写入缩略介绍缓存失败：{e}"))?;
    Ok(())
}

// --- AI provider selection + GLM settings ---

pub const DEEPSEEK_PROVIDER: &str = "deepseek";
pub const GLM_PROVIDER: &str = "glm";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiConfig {
    pub api_key: String,
    pub api_url: String,
    pub model: String,
    pub provider: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlmSettingsPublic {
    pub api_key_configured: bool,
    pub api_url: String,
    pub model: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct GlmSettingsFile {
    #[serde(default)]
    api_key: String,
    #[serde(default)]
    api_url: String,
    #[serde(default)]
    model: String,
}

impl Default for GlmSettingsFile {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            api_url: "https://open.bigmodel.cn/api/coding/paas/v4/chat/completions".into(),
            model: "GLM-5.1".into(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct AiProviderFile {
    #[serde(default)]
    provider: String,
}

fn ai_provider_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_local_dir(app)?.join("ai_provider.json"))
}

fn glm_settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_local_dir(app)?.join("glm_settings.json"))
}

pub fn load_ai_provider(app: &AppHandle) -> Result<String, String> {
    let path = ai_provider_path(app)?;
    if !path.is_file() {
        return Ok(DEEPSEEK_PROVIDER.to_string());
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let file: AiProviderFile =
        serde_json::from_str(&text).map_err(|e| format!("读取 AI provider 配置失败：{e}"))?;
    let p = file.provider.trim().to_lowercase();
    if p == GLM_PROVIDER {
        Ok(GLM_PROVIDER.to_string())
    } else {
        Ok(DEEPSEEK_PROVIDER.to_string())
    }
}

pub fn save_ai_provider(app: &AppHandle, provider: String) -> Result<(), String> {
    let path = ai_provider_path(app)?;
    ensure_parent(&path)?;
    let file = AiProviderFile {
        provider: provider.trim().to_lowercase(),
    };
    let json =
        serde_json::to_string_pretty(&file).map_err(|e| format!("序列化 AI provider 配置失败：{e}"))?;
    fs::write(path, json).map_err(|e| format!("写入 AI provider 配置失败：{e}"))?;
    Ok(())
}

pub fn load_active_ai_config(app: &AppHandle) -> Result<AiConfig, String> {
    let provider = load_ai_provider(app)?;
    match provider.as_str() {
        GLM_PROVIDER => {
            let path = glm_settings_path(app)?;
            let file = if path.is_file() {
                let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
                serde_json::from_str::<GlmSettingsFile>(&text)
                    .map_err(|e| format!("读取 GLM 配置失败：{e}"))?
            } else {
                GlmSettingsFile::default()
            };
            let key = file.api_key.trim().to_string();
            if key.is_empty() {
                return Err("请先在设置中保存 GLM API Key。".to_string());
            }
            let url = if file.api_url.trim().is_empty() {
                "https://open.bigmodel.cn/api/coding/paas/v4/chat/completions".to_string()
            } else {
                file.api_url.trim().to_string()
            };
            let model = if file.model.trim().is_empty() {
                "GLM-5.1".to_string()
            } else {
                file.model.trim().to_string()
            };
            Ok(AiConfig {
                api_key: key,
                api_url: url,
                model,
                provider: GLM_PROVIDER.to_string(),
            })
        }
        _ => {
            // DeepSeek (default)
            let key = load_deepseek_api_key(app)?
                .filter(|k| !k.is_empty())
                .ok_or_else(|| "请先在设置中保存 API Key。".to_string())?;
            Ok(AiConfig {
                api_key: key,
                api_url: "https://api.deepseek.com/chat/completions".to_string(),
                model: "deepseek-chat".to_string(),
                provider: DEEPSEEK_PROVIDER.to_string(),
            })
        }
    }
}

pub fn get_glm_settings_public(app: &AppHandle) -> Result<GlmSettingsPublic, String> {
    let path = glm_settings_path(app)?;
    if !path.is_file() {
        let def = GlmSettingsFile::default();
        return Ok(GlmSettingsPublic {
            api_key_configured: false,
            api_url: def.api_url,
            model: def.model,
        });
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let file: GlmSettingsFile =
        serde_json::from_str(&text).map_err(|e| format!("读取 GLM 配置失败：{e}"))?;
    let configured = !file.api_key.trim().is_empty();
    let url = if file.api_url.trim().is_empty() {
        "https://open.bigmodel.cn/api/coding/paas/v4/chat/completions"
    } else {
        file.api_url.trim()
    }
    .to_string();
    let model = if file.model.trim().is_empty() {
        "GLM-5.1"
    } else {
        file.model.trim()
    }
    .to_string();
    Ok(GlmSettingsPublic {
        api_key_configured: configured,
        api_url: url,
        model,
    })
}

pub fn save_glm_settings(
    app: &AppHandle,
    api_key: String,
    api_url: String,
    model: String,
) -> Result<(), String> {
    let path = glm_settings_path(app)?;
    ensure_parent(&path)?;
    let file = GlmSettingsFile {
        api_key: api_key.trim().to_string(),
        api_url: api_url.trim().to_string(),
        model: model.trim().to_string(),
    };
    let json =
        serde_json::to_string_pretty(&file).map_err(|e| format!("序列化 GLM 配置失败：{e}"))?;
    fs::write(path, json).map_err(|e| format!("写入 GLM 配置失败：{e}"))?;
    Ok(())
}

pub fn load_glm_api_key(app: &AppHandle) -> Result<Option<String>, String> {
    let path = glm_settings_path(app)?;
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let file: GlmSettingsFile =
        serde_json::from_str(&text).map_err(|e| format!("读取 GLM 配置失败：{e}"))?;
    let k = file.api_key.trim().to_string();
    if k.is_empty() {
        Ok(None)
    } else {
        Ok(Some(k))
    }
}

// --- Custom categories (persisted after reclassify confirm) ---

fn custom_categories_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_local_dir(app)?.join("custom_categories.json"))
}

pub fn load_custom_categories(
    app: &AppHandle,
) -> Result<Vec<crate::deepseek::CustomCategory>, String> {
    let path = custom_categories_path(app)?;
    if !path.is_file() {
        return Ok(vec![]);
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let v: Vec<crate::deepseek::CustomCategory> =
        serde_json::from_str(&text).map_err(|e| format!("读取自定义分类失败：{e}"))?;
    Ok(v)
}

pub fn save_custom_categories(
    app: &AppHandle,
    categories: &[crate::deepseek::CustomCategory],
) -> Result<(), String> {
    let path = custom_categories_path(app)?;
    ensure_parent(&path)?;
    let json = serde_json::to_string_pretty(categories)
        .map_err(|e| format!("序列化自定义分类失败：{e}"))?;
    fs::write(path, json).map_err(|e| format!("写入自定义分类失败：{e}"))?;
    Ok(())
}

pub fn clear_custom_categories(app: &AppHandle) -> Result<(), String> {
    let path = custom_categories_path(app)?;
    if path.is_file() {
        fs::remove_file(&path).map_err(|e| format!("删除自定义分类文件失败：{e}"))?;
    }
    Ok(())
}

// --- Gitee OAuth / backup ---

fn gitee_app_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_local_dir(app)?.join("gitee_app.json"))
}

fn gitee_token_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_local_dir(app)?.join("gitee_token.json"))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GiteeSettingsPublic {
    pub app_configured: bool,
    pub connected: bool,
    pub owner_login: Option<String>,
    pub repo_name: Option<String>,
    /// 已保存的 OAuth Client ID（非密钥，用于表单预填）。
    pub client_id_saved: Option<String>,
    /// 本地已保存的备份仓库名（来自应用配置，用于表单预填）。
    pub saved_repo_name: Option<String>,
    /// 用户须在 Gitee 第三方应用里填写完全一致的回调地址。
    pub oauth_callback_url: String,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct GiteeAppFile {
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub client_secret: String,
    #[serde(default)]
    pub repo_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GiteeTokenFile {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: String,
    /// 毫秒时间戳；到期前会尝试 refresh。
    #[serde(default)]
    pub expires_at_ms: Option<i64>,
    pub owner_login: String,
    pub repo_name: String,
}

pub fn gitee_oauth_callback_url() -> &'static str {
    "http://127.0.0.1:19876/oauth/gitee/callback"
}

pub fn get_gitee_settings_public(app: &AppHandle) -> Result<GiteeSettingsPublic, String> {
    let app_path = gitee_app_path(app)?;
    let (app_ok, client_id_saved, saved_repo_name) = if app_path.is_file() {
        let text = fs::read_to_string(&app_path).map_err(|e| e.to_string())?;
        let f: GiteeAppFile =
            serde_json::from_str(&text).map_err(|e| format!("读取 Gitee 应用配置失败：{e}"))?;
        let ok = !f.client_id.trim().is_empty() && !f.client_secret.trim().is_empty();
        let cid = if f.client_id.trim().is_empty() {
            None
        } else {
            Some(f.client_id.trim().to_string())
        };
        let sr = if f.repo_name.trim().is_empty() {
            None
        } else {
            Some(f.repo_name.trim().to_string())
        };
        (ok, cid, sr)
    } else {
        (false, None, None)
    };

    let token_path = gitee_token_path(app)?;
    let (connected, owner, repo) = if token_path.is_file() {
        let text = fs::read_to_string(&token_path).map_err(|e| e.to_string())?;
        if let Ok(t) = serde_json::from_str::<GiteeTokenFile>(&text) {
            if !t.access_token.trim().is_empty() && !t.owner_login.trim().is_empty() {
                (
                    true,
                    Some(t.owner_login),
                    Some(t.repo_name).filter(|s| !s.trim().is_empty()),
                )
            } else {
                (false, None, None)
            }
        } else {
            (false, None, None)
        }
    } else {
        (false, None, None)
    };

    Ok(GiteeSettingsPublic {
        app_configured: app_ok,
        connected,
        owner_login: owner,
        repo_name: repo,
        client_id_saved,
        saved_repo_name,
        oauth_callback_url: gitee_oauth_callback_url().to_string(),
    })
}

pub fn load_gitee_app(app: &AppHandle) -> Result<GiteeAppFile, String> {
    let path = gitee_app_path(app)?;
    if !path.is_file() {
        return Ok(GiteeAppFile::default());
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| format!("读取 Gitee 应用配置失败：{e}"))
}

fn sanitize_gitee_repo_name(raw: &str) -> String {
    let s = raw.trim().to_lowercase();
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
            out.push(c);
        } else if c.is_whitespace() {
            out.push('-');
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        "aicontrols-backup".to_string()
    } else {
        out
    }
}

pub fn save_gitee_app(
    app: &AppHandle,
    client_id: String,
    client_secret: String,
    repo_name: String,
) -> Result<(), String> {
    let path = gitee_app_path(app)?;
    let mut prev = load_gitee_app(app).unwrap_or_default();
    let mut id = client_id.trim().to_string();
    if id.is_empty() {
        id = prev.client_id.trim().to_string();
    }
    let mut secret = client_secret.trim().to_string();
    if secret.is_empty() {
        secret = prev.client_secret.clone();
    }
    let repo = repo_name.trim().to_string();
    let repo = if repo.is_empty() {
        "aicontrols-backup".to_string()
    } else {
        sanitize_gitee_repo_name(&repo)
    };
    if id.is_empty() {
        return Err("Client ID 不能为空。".into());
    }
    if secret.is_empty() {
        return Err("Client Secret 不能为空（首次保存请填写完整）。".into());
    }
    prev.client_id = id;
    prev.client_secret = secret;
    prev.repo_name = repo;
    ensure_parent(&path)?;
    let json =
        serde_json::to_string_pretty(&prev).map_err(|e| format!("序列化 Gitee 配置失败：{e}"))?;
    fs::write(path, json).map_err(|e| format!("写入 Gitee 配置失败：{e}"))?;
    Ok(())
}

pub fn load_gitee_token(app: &AppHandle) -> Result<Option<GiteeTokenFile>, String> {
    let path = gitee_token_path(app)?;
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let t: GiteeTokenFile =
        serde_json::from_str(&text).map_err(|e| format!("读取 Gitee 授权失败：{e}"))?;
    if t.access_token.trim().is_empty() {
        return Ok(None);
    }
    Ok(Some(t))
}

pub fn save_gitee_token(app: &AppHandle, token: &GiteeTokenFile) -> Result<(), String> {
    let path = gitee_token_path(app)?;
    ensure_parent(&path)?;
    let json =
        serde_json::to_string_pretty(token).map_err(|e| format!("序列化 Gitee 授权失败：{e}"))?;
    fs::write(path, json).map_err(|e| format!("写入 Gitee 授权失败：{e}"))?;
    Ok(())
}

pub fn clear_gitee_token(app: &AppHandle) -> Result<(), String> {
    let path = gitee_token_path(app)?;
    if path.is_file() {
        fs::remove_file(&path).map_err(|e| format!("删除 Gitee 授权文件失败：{e}"))?;
    }
    clear_gitee_backup_fingerprint(app)?;
    Ok(())
}

// --- Gitee 备份内容指纹（用于跳过无变更的定时同步）---

fn gitee_backup_fingerprint_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_local_dir(app)?.join("gitee_backup_fingerprint.json"))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct GiteeBackupFingerprint {
    #[serde(default)]
    pub prompt_sha256: Option<String>,
    #[serde(default)]
    pub resource_sha256: Option<String>,
    #[serde(default)]
    pub all_scenarios_sha256: Option<String>,
    #[serde(default)]
    pub all_briefs_sha256: Option<String>,
}

pub fn load_gitee_backup_fingerprint(
    app: &AppHandle,
) -> Result<Option<GiteeBackupFingerprint>, String> {
    let path = gitee_backup_fingerprint_path(app)?;
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let v: GiteeBackupFingerprint =
        serde_json::from_str(&text).map_err(|e| format!("读取备份指纹失败：{e}"))?;
    Ok(Some(v))
}

pub fn save_gitee_backup_fingerprint(
    app: &AppHandle,
    fp: &GiteeBackupFingerprint,
) -> Result<(), String> {
    let path = gitee_backup_fingerprint_path(app)?;
    ensure_parent(&path)?;
    let json = serde_json::to_string_pretty(fp).map_err(|e| format!("序列化备份指纹失败：{e}"))?;
    fs::write(path, json).map_err(|e| format!("写入备份指纹失败：{e}"))?;
    Ok(())
}

pub fn clear_gitee_backup_fingerprint(app: &AppHandle) -> Result<(), String> {
    let path = gitee_backup_fingerprint_path(app)?;
    if path.is_file() {
        fs::remove_file(&path).map_err(|e| format!("删除备份指纹失败：{e}"))?;
    }
    Ok(())
}

// --- User-added agent roots (dot-folders like `~/.mytool`) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserAgentEntry {
    pub id: String,
    pub path: String,
    pub label: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct UserAgentsFile {
    #[serde(default)]
    agents: Vec<UserAgentEntry>,
}

fn user_agents_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_local_dir(app)?.join("user_agents.json"))
}

pub fn load_user_agents(app: &AppHandle) -> Result<Vec<UserAgentEntry>, String> {
    let path = user_agents_path(app)?;
    if !path.is_file() {
        return Ok(vec![]);
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let file: UserAgentsFile =
        serde_json::from_str(&text).map_err(|e| format!("读取自定义 Agent 列表失败：{e}"))?;
    Ok(file.agents)
}

fn save_user_agents(app: &AppHandle, agents: &[UserAgentEntry]) -> Result<(), String> {
    let path = user_agents_path(app)?;
    ensure_parent(&path)?;
    let file = UserAgentsFile {
        agents: agents.to_vec(),
    };
    let json = serde_json::to_string_pretty(&file).map_err(|e| format!("序列化自定义 Agent 失败：{e}"))?;
    fs::write(path, json).map_err(|e| format!("写入自定义 Agent 失败：{e}"))?;
    Ok(())
}

/// Resolve persisted root for ids returned by [`crate::scan::user_agent_stable_id`].
pub fn user_agent_root_for_id(app: &AppHandle, agent_id: &str) -> Result<Option<PathBuf>, String> {
    if !agent_id.starts_with("useragent-") {
        return Ok(None);
    }
    let agents = load_user_agents(app)?;
    Ok(agents
        .iter()
        .find(|a| a.id == agent_id)
        .map(|a| PathBuf::from(a.path.trim())))
}

/// Add a dot-folder as a custom agent. `path` should be a directory whose name starts with `.`.
pub fn add_user_agent_from_path(app: &AppHandle, path: &str) -> Result<UserAgentEntry, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("路径不能为空".into());
    }
    let p = Path::new(trimmed);
    let can = p
        .canonicalize()
        .map_err(|e| format!("无法解析所选路径：{e}"))?;
    if !can.is_dir() {
        return Err("所选路径不是文件夹".into());
    }
    let name = can
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "无法读取文件夹名称".to_string())?;
    if !name.starts_with('.') {
        return Err("请选择以「.」开头的配置目录（例如 .cursor、.myagent）。".into());
    }

    let mut agents = load_user_agents(app)?;
    let can_s = can.to_string_lossy().to_string();
    for a in &agents {
        if a.path == can_s {
            return Err("该目录已在自定义 Agent 列表中。".into());
        }
    }

    let builtins = crate::scan::detect_agents();
    for b in &builtins {
        if Path::new(&b.root_path).canonicalize().map(|x| x == can).unwrap_or(false) {
            return Err("该目录已由内置 Agent 识别，无需重复添加。".into());
        }
    }

    let id = crate::scan::user_agent_stable_id(&can);
    let label = name
        .strip_prefix('.')
        .filter(|s| !s.is_empty())
        .unwrap_or(name)
        .to_string();
    let entry = UserAgentEntry {
        id,
        path: can_s,
        label,
    };
    agents.push(entry.clone());
    save_user_agents(app, &agents)?;
    Ok(entry)
}

pub fn remove_user_agent(app: &AppHandle, agent_id: &str) -> Result<(), String> {
    if !agent_id.starts_with("useragent-") {
        return Err("只能移除自定义 Agent".into());
    }
    let mut agents = load_user_agents(app)?;
    let before = agents.len();
    agents.retain(|a| a.id != agent_id);
    if agents.len() == before {
        return Err("未找到该自定义 Agent".into());
    }
    save_user_agents(app, &agents)
}

// --- Hidden built-in agents (removed from sidebar only; scan / disk unchanged) ---

#[derive(Debug, Default, Deserialize, Serialize)]
struct HiddenSidebarAgentsFile {
    #[serde(default)]
    hidden_ids: Vec<String>,
}

fn hidden_sidebar_agents_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_local_dir(app)?.join("hidden_sidebar_agent_ids.json"))
}

pub fn load_hidden_sidebar_agent_ids(app: &AppHandle) -> Result<HashSet<String>, String> {
    let path = hidden_sidebar_agents_path(app)?;
    if !path.is_file() {
        return Ok(HashSet::new());
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let file: HiddenSidebarAgentsFile =
        serde_json::from_str(&text).map_err(|e| format!("读取侧栏隐藏 Agent 列表失败：{e}"))?;
    Ok(file.hidden_ids.into_iter().map(|s| s.trim().to_string()).collect())
}

fn save_hidden_sidebar_agent_ids(app: &AppHandle, ids: &HashSet<String>) -> Result<(), String> {
    let path = hidden_sidebar_agents_path(app)?;
    ensure_parent(&path)?;
    let mut v: Vec<String> = ids.iter().cloned().collect();
    v.sort();
    let file = HiddenSidebarAgentsFile { hidden_ids: v };
    let json =
        serde_json::to_string_pretty(&file).map_err(|e| format!("序列化侧栏隐藏列表失败：{e}"))?;
    fs::write(path, json).map_err(|e| format!("写入侧栏隐藏列表失败：{e}"))?;
    Ok(())
}

/// Hide a built-in agent (`cursor`, `claude`, …) from the sidebar list only.
pub fn hide_sidebar_builtin_agent(app: &AppHandle, agent_id: &str) -> Result<(), String> {
    let id = agent_id.trim();
    if id.is_empty() {
        return Err("agent_id 不能为空".into());
    }
    if id.starts_with("useragent-") {
        return Err("自定义 Agent 请使用 remove_user_agent".into());
    }
    let mut set = load_hidden_sidebar_agent_ids(app)?;
    set.insert(id.to_string());
    save_hidden_sidebar_agent_ids(app, &set)
}

/// Clear all hidden built-in agents so the sidebar shows the full auto-detected list again.
pub fn clear_hidden_sidebar_agents(app: &AppHandle) -> Result<(), String> {
    let path = hidden_sidebar_agents_path(app)?;
    if path.is_file() {
        fs::remove_file(&path).map_err(|e| format!("清除侧栏隐藏列表失败：{e}"))?;
    }
    Ok(())
}

// ─── Custom skill paths per agent ──────────────────────────────────────────

#[derive(Debug, Default, Deserialize, Serialize)]
struct AgentCustomSkillPathsFile {
    /// agent_id → list of absolute (or ~-prefixed) paths
    #[serde(default)]
    paths: HashMap<String, Vec<String>>,
}

fn agent_custom_skill_paths_file(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_local_dir(app)?.join("agent_custom_skill_paths.json"))
}

pub fn load_agent_custom_skill_paths(
    app: &AppHandle,
) -> Result<HashMap<String, Vec<String>>, String> {
    let path = agent_custom_skill_paths_file(app)?;
    if !path.is_file() {
        return Ok(HashMap::new());
    }
    let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let file: AgentCustomSkillPathsFile = serde_json::from_str(&text)
        .map_err(|e| format!("读取 Agent 自定义 Skill 路径失败：{e}"))?;
    Ok(file.paths)
}

pub fn save_agent_custom_skill_paths(
    app: &AppHandle,
    paths: &HashMap<String, Vec<String>>,
) -> Result<(), String> {
    let path = agent_custom_skill_paths_file(app)?;
    ensure_parent(&path)?;
    let file = AgentCustomSkillPathsFile {
        paths: paths.clone(),
    };
    let json = serde_json::to_string_pretty(&file)
        .map_err(|e| format!("序列化 Agent 自定义 Skill 路径失败：{e}"))?;
    fs::write(path, json).map_err(|e| format!("写入 Agent 自定义 Skill 路径失败：{e}"))?;
    Ok(())
}
