//! Tauri-managed services and outbound HTTP runtime state.

use crate::store::AppState;
use crate::{commands, SkillService};
use std::sync::Arc;
use tauri::Manager;

pub(super) fn manage(app: &mut tauri::App, app_state: AppState) {
    // 将同一个实例注入到全局状态，避免重复创建导致的不一致
    app.manage(app_state);

    // 从数据库加载日志配置并应用
    {
        let db = &app.state::<AppState>().db;
        if let Ok(log_config) = db.get_log_config() {
            log::set_max_level(log_config.to_level_filter());
            log::info!(
                "已加载日志配置: enabled={}, level={}",
                log_config.enabled,
                log_config.level
            );
        }
    }

    // 初始化 SkillService
    let skill_service = SkillService::new();
    app.manage(commands::skill::SkillServiceState(Arc::new(skill_service)));

    // 初始化 CopilotAuthManager
    {
        use crate::proxy::providers::copilot_auth::CopilotAuthManager;
        use commands::CopilotAuthState;
        use tokio::sync::RwLock;

        let app_config_dir = crate::config::get_app_config_dir();
        let copilot_auth_manager = CopilotAuthManager::new(app_config_dir);
        app.manage(CopilotAuthState(Arc::new(RwLock::new(
            copilot_auth_manager,
        ))));
        log::info!("✓ CopilotAuthManager initialized");
    }

    // 初始化 CodexOAuthManager (ChatGPT Plus/Pro 反代)
    {
        use crate::proxy::providers::codex_oauth_auth::CodexOAuthManager;
        use commands::CodexOAuthState;
        use tokio::sync::RwLock;

        let app_config_dir = crate::config::get_app_config_dir();
        let codex_oauth_manager = CodexOAuthManager::new(app_config_dir);
        app.manage(CodexOAuthState(Arc::new(RwLock::new(codex_oauth_manager))));
        log::info!("✓ CodexOAuthManager initialized");
    }

    // 初始化 XaiOAuthManager (xAI / Grok OAuth)
    {
        use crate::proxy::providers::xai_oauth_auth::XaiOAuthManager;
        use commands::XaiOAuthState;
        use tokio::sync::RwLock;

        let app_config_dir = crate::config::get_app_config_dir();
        let xai_oauth_manager = XaiOAuthManager::new(app_config_dir);
        app.manage(XaiOAuthState(Arc::new(RwLock::new(xai_oauth_manager))));
        log::info!("✓ XaiOAuthManager initialized");
    }

    // 初始化全局出站代理 HTTP 客户端
    {
        let db = &app.state::<AppState>().db;
        let proxy_url = db.get_global_proxy_url().ok().flatten();

        if let Err(e) = crate::proxy::http_client::init(proxy_url.as_deref()) {
            log::error!("[GlobalProxy] [GP-005] Failed to initialize with saved config: {e}");

            // 清除无效的代理配置
            if proxy_url.is_some() {
                log::warn!("[GlobalProxy] [GP-006] Clearing invalid proxy config from database");
                if let Err(clear_err) = db.set_global_proxy_url(None) {
                    log::error!(
                        "[GlobalProxy] [GP-007] Failed to clear invalid config: {clear_err}"
                    );
                }
            }

            // 使用直连模式重新初始化
            if let Err(fallback_err) = crate::proxy::http_client::init(None) {
                log::error!(
                    "[GlobalProxy] [GP-008] Failed to initialize direct connection: {fallback_err}"
                );
            }
        }
    }
}
