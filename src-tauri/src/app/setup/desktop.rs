//! Deep-link, tray, and desktop sync worker setup.

#[cfg(target_os = "macos")]
use super::super::macos_tray_icon;
use super::super::{handle_deeplink_url, redact_url_for_log};
use crate::store::AppState;
use crate::{app_store, tray};
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
#[cfg(target_os = "linux")]
use tauri::Manager;
use tauri_plugin_deep_link::DeepLinkExt;

pub(super) fn configure(
    app: &mut tauri::App,
    app_state: &AppState,
) -> Result<(), Box<dyn std::error::Error>> {
    // 迁移旧的 app_config_dir 配置到 Store
    if let Err(e) = app_store::migrate_app_config_dir_from_settings(app.handle()) {
        log::warn!("迁移 app_config_dir 失败: {e}");
    }

    // 启动阶段不再无条件保存,避免意外覆盖用户配置。

    // 注册 deep-link URL 处理器（使用正确的 DeepLinkExt API）
    log::info!("=== Registering deep-link URL handler ===");

    // Linux 和 Windows 调试模式需要显式注册
    #[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
    {
        #[cfg(target_os = "linux")]
        {
            // Use Tauri's path API to get correct path (includes app identifier)
            // tauri-plugin-deep-link writes to: ~/.local/share/com.OpenSunstar.desktop/applications/OpenSunstar-handler.desktop
            // Only register if .desktop file doesn't exist to avoid overwriting user customizations
            let should_register = app
                .path()
                .data_dir()
                .map(|d| !d.join("applications/OpenSunstar-handler.desktop").exists())
                .unwrap_or(true);

            if should_register {
                if let Err(e) = app.deep_link().register_all() {
                    log::error!("✗ Failed to register deep link schemes: {}", e);
                } else {
                    log::info!("✓ Deep link schemes registered (Linux)");
                }
            } else {
                log::info!("⊘ Deep link handler already exists, skipping registration");
            }
        }

        #[cfg(all(debug_assertions, windows))]
        {
            if let Err(e) = app.deep_link().register_all() {
                log::error!("✗ Failed to register deep link schemes: {}", e);
            } else {
                log::info!("✓ Deep link schemes registered (Windows debug)");
            }
        }
    }

    // 注册 URL 处理回调（所有平台通用）
    app.deep_link().on_open_url({
        let app_handle = app.handle().clone();
        move |event| {
            log::info!("=== Deep Link Event Received (on_open_url) ===");
            let urls = event.urls();
            log::info!("Received {} URL(s)", urls.len());

            if crate::lightweight::is_lightweight_mode() {
                if let Err(e) = crate::lightweight::exit_lightweight_mode(&app_handle) {
                    log::error!("退出轻量模式重建窗口失败: {e}");
                }
            }

            for (i, url) in urls.iter().enumerate() {
                let url_str = url.as_str();
                log::debug!("  URL[{i}]: {}", redact_url_for_log(url_str));

                if handle_deeplink_url(&app_handle, url_str, true, "on_open_url") {
                    break; // Process only first OpenSunstar:// URL
                }
            }
        }
    });
    log::info!("✓ Deep-link URL handler registered");

    // 创建动态托盘菜单
    let menu = tray::create_tray_menu(app.handle(), app_state)?;

    // 构建托盘
    let mut tray_builder = TrayIconBuilder::with_id(tray::TRAY_ID)
        .tooltip("OpenSunstar") // 鼠标悬停提示
        .on_tray_icon_event(|tray, event| match event {
            // 鼠标悬停/点击到托盘图标时，后台异步刷新用量缓存，
            // 让用户下一次（或快速打开菜单的那一刻）看到较新的数字。
            // refresh_all_usage_in_tray 内部有 10 秒防抖。
            TrayIconEvent::Enter { .. } | TrayIconEvent::Click { .. } => {
                let app = tray.app_handle().clone();
                tauri::async_runtime::spawn(async move {
                    crate::tray::refresh_all_usage_in_tray(&app).await;
                });
            }
            _ => log::debug!("unhandled event {event:?}"),
        })
        .menu(&menu)
        .on_menu_event(|app, event| {
            tray::handle_tray_menu_event(app, &event.id.0);
        })
        .show_menu_on_left_click(true);

    // 使用平台对应的托盘图标（macOS 使用模板图标适配深浅色）
    #[cfg(target_os = "macos")]
    {
        if let Some(icon) = macos_tray_icon() {
            tray_builder = tray_builder.icon(icon).icon_as_template(true);
        } else if let Some(icon) = app.default_window_icon() {
            log::warn!("Falling back to default window icon for tray");
            tray_builder = tray_builder.icon(icon.clone());
        } else {
            log::warn!("Failed to load macOS tray icon for tray");
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        if let Some(icon) = app.default_window_icon() {
            tray_builder = tray_builder.icon(icon.clone());
        } else {
            log::warn!("Failed to get default window icon for tray");
        }
    }

    let _tray = tray_builder.build(app)?;
    crate::services::webdav_auto_sync::start_worker(app_state.db.clone(), app.handle().clone());
    crate::services::s3_auto_sync::start_worker(app_state.db.clone(), app.handle().clone());
    Ok(())
}
