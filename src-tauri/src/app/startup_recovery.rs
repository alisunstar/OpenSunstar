//! Startup proxy recovery and common-config initialization.

use crate::store;

// ============================================================
// 启动时恢复代理状态
// ============================================================

/// 启动时代理恢复（崩溃清理 + 按 DB 状态重新接管）。
/// 必须在主窗口显示前完成，否则用户可能在代理尚未监听时打开 Claude Code。
pub(super) async fn run_startup_proxy_recovery(state: &store::AppState) {
    let has_backups = match state.db.has_any_live_backup().await {
        Ok(v) => v,
        Err(e) => {
            log::error!("检查 Live 备份失败: {e}");
            false
        }
    };
    let live_taken_over = state.proxy_service.detect_takeover_in_live_configs();

    if has_backups || live_taken_over {
        log::warn!("检测到上次异常退出（存在接管残留），正在恢复 Live 配置...");
        if let Err(e) = state.proxy_service.recover_from_crash().await {
            log::error!("恢复 Live 配置失败: {e}");
        } else {
            log::info!("Live 配置已恢复");
        }
    }

    initialize_common_config_snippets(state);
    restore_proxy_state_on_startup(state).await;
}

///
/// 检查 `proxy_config.enabled` 字段，如果有任一应用的状态为 `true`，
/// 则自动启动代理服务并接管对应应用的 Live 配置。
async fn restore_proxy_state_on_startup(state: &store::AppState) {
    // 收集需要恢复接管的应用列表（从 proxy_config.enabled 读取）
    let mut apps_to_restore = Vec::new();
    for app_type in ["claude", "codex", "gemini"] {
        if let Ok(config) = state.db.get_proxy_config_for_app(app_type).await {
            if config.enabled {
                apps_to_restore.push(app_type);
            }
        }
    }

    if apps_to_restore.is_empty() {
        log::debug!("启动时无需恢复代理状态");
        return;
    }

    log::info!("检测到上次代理状态需要恢复，应用列表: {apps_to_restore:?}");

    // 逐个恢复接管状态
    for app_type in apps_to_restore {
        match state
            .proxy_service
            .set_takeover_for_app(app_type, true)
            .await
        {
            Ok(()) => {
                log::info!("✓ 已恢复 {app_type} 的代理接管状态");
            }
            Err(e) => {
                log::error!("✗ 恢复 {app_type} 的代理接管状态失败: {e}");
                // 失败时清除该应用的状态，避免下次启动再次尝试
                if let Err(clear_err) = state
                    .proxy_service
                    .set_takeover_for_app(app_type, false)
                    .await
                {
                    log::error!("清除 {app_type} 代理状态失败: {clear_err}");
                }
            }
        }
    }
}

fn initialize_common_config_snippets(state: &store::AppState) {
    // Auto-extract common config snippets from clean live files when snippet is missing.
    // This must run before proxy takeover is restored on startup, otherwise we'd read
    // proxy-placeholder configs instead of the user's actual live settings.
    for app_type in crate::app_config::AppType::all() {
        if !state
            .db
            .should_auto_extract_config_snippet(app_type.as_str())
            .unwrap_or(false)
        {
            continue;
        }

        let settings = match crate::services::provider::ProviderService::read_live_settings(
            app_type.clone(),
        ) {
            Ok(s) => s,
            Err(_) => continue,
        };

        match crate::services::provider::ProviderService::extract_common_config_snippet_from_settings(
            app_type.clone(),
            &settings,
        ) {
            Ok(snippet) if !snippet.is_empty() && snippet != "{}" => {
                match state.db.set_config_snippet(app_type.as_str(), Some(snippet)) {
                    Ok(()) => {
                        let _ = state.db.set_config_snippet_cleared(app_type.as_str(), false);
                        log::info!(
                            "✓ Auto-extracted common config snippet for {}",
                            app_type.as_str()
                        );
                    }
                    Err(e) => log::warn!(
                        "✗ Failed to save config snippet for {}: {e}",
                        app_type.as_str()
                    ),
                }
            }
            Ok(_) => log::debug!(
                "○ Live config for {} has no extractable common fields",
                app_type.as_str()
            ),
            Err(e) => log::warn!(
                "✗ Failed to extract config snippet for {}: {e}",
                app_type.as_str()
            ),
        }
    }

    let should_run_legacy_migration = state
        .db
        .is_legacy_common_config_migrated()
        .map(|done| !done)
        .unwrap_or(true);

    if should_run_legacy_migration {
        for app_type in [
            crate::app_config::AppType::Claude,
            crate::app_config::AppType::Codex,
            crate::app_config::AppType::Gemini,
        ] {
            if let Err(e) = crate::services::provider::ProviderService::migrate_legacy_common_config_usage_if_needed(
                state,
                app_type.clone(),
            ) {
                log::warn!(
                    "✗ Failed to migrate legacy common-config usage for {}: {e}",
                    app_type.as_str()
                );
            }
        }

        if let Err(e) = state.db.set_legacy_common_config_migrated(true) {
            log::warn!("✗ Failed to persist legacy common-config migration flag: {e}");
        }
    }
}
