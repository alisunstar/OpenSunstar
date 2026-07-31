//! Tauri desktop application composition root.
//!
//! Keeps startup, runtime events, and shutdown orchestration out of the crate API surface.

mod dialogs;
mod invoke;
mod runtime_events;
mod setup;
mod shutdown;
mod startup_recovery;

pub(crate) use shutdown::remove_tray_icon_before_exit;
pub use shutdown::{
    cleanup_before_exit, destroy_single_instance_lock, restart_process,
    save_window_state_before_exit,
};

#[cfg(target_os = "linux")]
use crate::linux_fix;
use crate::store::AppState;
use crate::{panic_hook, tray};
#[cfg(target_os = "macos")]
use tauri::image::Image;
use tauri::{Emitter, Manager};

#[cfg(target_os = "windows")]
fn set_windows_app_user_model_id(app: &tauri::AppHandle) {
    let app_id = app.config().identifier.clone();
    let wide_app_id: Vec<u16> = app_id.encode_utf16().chain(std::iter::once(0)).collect();

    let result = unsafe {
        windows_sys::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID(wide_app_id.as_ptr())
    };

    if result < 0 {
        log::warn!("设置 Windows AppUserModelID 失败: 0x{result:08X}");
    } else {
        log::debug!("Windows AppUserModelID 已设置为 {app_id}");
    }
}

fn redact_url_for_log(url_str: &str) -> String {
    match url::Url::parse(url_str) {
        Ok(url) => {
            let mut output = format!("{}://", url.scheme());
            if let Some(host) = url.host_str() {
                output.push_str(host);
            }
            output.push_str(url.path());

            let mut keys: Vec<String> = url.query_pairs().map(|(k, _)| k.to_string()).collect();
            keys.sort();
            keys.dedup();

            if !keys.is_empty() {
                output.push_str("?[keys:");
                output.push_str(&keys.join(","));
                output.push(']');
            }

            output
        }
        Err(_) => {
            let base = url_str.split('#').next().unwrap_or(url_str);
            match base.split_once('?') {
                Some((prefix, _)) => format!("{prefix}?[redacted]"),
                None => base.to_string(),
            }
        }
    }
}

/// 统一处理 OpenSunstar:// 深链接 URL
///
/// - 解析 URL
/// - 向前端发射 `deeplink-import` / `deeplink-error` 事件
/// - 可选：在成功时聚焦主窗口
fn handle_deeplink_url(
    app: &tauri::AppHandle,
    url_str: &str,
    focus_main_window: bool,
    source: &str,
) -> bool {
    if !url_str.starts_with("OpenSunstar://") {
        return false;
    }

    let redacted_url = redact_url_for_log(url_str);
    log::info!("✓ Deep link URL detected from {source}: {redacted_url}");
    log::debug!("Deep link URL (raw) from {source}: {url_str}");

    match crate::deeplink::parse_deeplink_url(url_str) {
        Ok(request) => {
            log::info!(
                "✓ Successfully parsed deep link: resource={}, app={:?}, name={:?}",
                request.resource,
                request.app,
                request.name
            );

            if let Err(e) = app.emit("deeplink-import", &request) {
                log::error!("✗ Failed to emit deeplink-import event: {e}");
            } else {
                log::info!("✓ Emitted deeplink-import event to frontend");
            }

            if focus_main_window {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.unminimize();
                    let _ = window.show();
                    let _ = window.set_focus();
                    #[cfg(target_os = "linux")]
                    {
                        linux_fix::nudge_main_window(window.clone());
                    }
                    log::info!("✓ Window shown and focused");
                }
            }
        }
        Err(e) => {
            log::error!("✗ Failed to parse deep link URL: {e}");

            if let Err(emit_err) = app.emit(
                "deeplink-error",
                serde_json::json!({
                    "url": url_str,
                    "error": e.to_string()
                }),
            ) {
                log::error!("✗ Failed to emit deeplink-error event: {emit_err}");
            }
        }
    }

    true
}

/// 更新托盘菜单的Tauri命令
#[tauri::command]
async fn update_tray_menu(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<bool, String> {
    match tray::create_tray_menu(&app, state.inner()) {
        Ok(new_menu) => {
            if let Some(tray) = app.tray_by_id(tray::TRAY_ID) {
                tray.set_menu(Some(new_menu))
                    .map_err(|e| format!("更新托盘菜单失败: {e}"))?;
                return Ok(true);
            }
            Ok(false)
        }
        Err(err) => {
            log::error!("创建托盘菜单失败: {err}");
            Ok(false)
        }
    }
}

#[cfg(target_os = "macos")]
fn macos_tray_icon() -> Option<Image<'static>> {
    const ICON_BYTES: &[u8] = include_bytes!("../icons/tray/macos/statusbar_template_3x.png");

    match Image::from_bytes(ICON_BYTES) {
        Ok(icon) => Some(icon),
        Err(err) => {
            log::warn!("Failed to load macOS tray icon: {err}");
            None
        }
    }
}

pub fn run() {
    // 设置 panic hook，在应用崩溃时记录日志到 <app_config_dir>/crash.log（默认 ~/.OpenSunstar/crash.log）
    panic_hook::setup_panic_hook();

    let mut builder = tauri::Builder::default();

    #[cfg(any(target_os = "macos", target_os = "windows", target_os = "linux"))]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            log::info!("=== Single Instance Callback Triggered ===");
            log::debug!("Args count: {}", args.len());
            for (i, arg) in args.iter().enumerate() {
                log::debug!("  arg[{i}]: {}", redact_url_for_log(arg));
            }

            if crate::lightweight::is_lightweight_mode() {
                if let Err(e) = crate::lightweight::exit_lightweight_mode(app) {
                    log::error!("退出轻量模式重建窗口失败: {e}");
                }
            }

            // Check for deep link URL in args (mainly for Windows/Linux command line)
            let mut found_deeplink = false;
            for arg in &args {
                if handle_deeplink_url(app, arg, false, "single_instance args") {
                    found_deeplink = true;
                    break;
                }
            }

            if !found_deeplink {
                log::info!("ℹ No deep link URL found in args (this is expected on macOS when launched via system)");
            }

            // Show and focus window regardless
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
                #[cfg(target_os = "linux")]
                {
                    linux_fix::nudge_main_window(window.clone());
                }
            }
        }));
    }

    let builder = builder
        // 注册 deep-link 插件（处理 macOS AppleEvent 和其他平台的深链接）
        .plugin(tauri_plugin_deep_link::init())
        // 拦截窗口关闭：根据设置决定是否最小化到托盘
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let settings = crate::settings::get_settings();

                if settings.minimize_to_tray_on_close {
                    api.prevent_close();
                    let _ = window.hide();
                    #[cfg(target_os = "windows")]
                    {
                        let _ = window.set_skip_taskbar(true);
                    }
                    #[cfg(target_os = "macos")]
                    {
                        tray::apply_tray_policy(window.app_handle(), false);
                    }
                } else {
                    api.prevent_close();
                    window.app_handle().exit(0);
                }
            }
        })
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        // OS 系统通知（工作区重构 2026-07-30）：预算 critical/emergency 与
        // 故障转移走系统级通知 —— 窗口关了也能响，托盘才是这个产品的家。
        // 使用 tauri-winrt-notification 直接发送 Windows toast 通知。
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(shutdown::window_state_flags())
                .build(),
        )
        .setup(setup::run);

    let builder = invoke::register(builder);

    let app = builder
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(runtime_events::handle);
}
