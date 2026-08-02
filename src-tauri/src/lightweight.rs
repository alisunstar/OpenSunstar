use std::sync::atomic::{AtomicBool, Ordering};

use tauri::Manager;

static LIGHTWEIGHT_MODE: AtomicBool = AtomicBool::new(false);
static MAIN_WINDOW_REBUILDING: AtomicBool = AtomicBool::new(false);

struct MainWindowRebuildGuard;

impl Drop for MainWindowRebuildGuard {
    fn drop(&mut self) {
        MAIN_WINDOW_REBUILDING.store(false, Ordering::Release);
    }
}

fn reveal_main_window(_app: &tauri::AppHandle, window: &tauri::WebviewWindow) {
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();

    #[cfg(target_os = "linux")]
    {
        crate::linux_fix::nudge_main_window(window.clone());
    }
    #[cfg(target_os = "windows")]
    {
        let _ = window.set_skip_taskbar(false);
    }
    #[cfg(target_os = "macos")]
    {
        crate::tray::apply_tray_policy(_app, true);
    }
}

pub fn show_main_window(app: &tauri::AppHandle) -> bool {
    if let Some(window) = app.get_webview_window("main") {
        reveal_main_window(app, &window);
        true
    } else {
        false
    }
}

pub fn enter_lightweight_mode(app: &tauri::AppHandle) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.set_skip_taskbar(true);
        }
    }
    #[cfg(target_os = "macos")]
    {
        crate::tray::apply_tray_policy(app, false);
    }

    if let Some(window) = app.get_webview_window("main") {
        crate::save_window_state_before_exit(app);
        window
            .destroy()
            .map_err(|e| format!("销毁主窗口失败: {e}"))?;
    }
    // else: already in lightweight mode or window not found, just set the flag

    LIGHTWEIGHT_MODE.store(true, Ordering::Release);
    crate::tray::refresh_tray_menu(app);
    log::info!("进入轻量模式");
    Ok(())
}

pub fn exit_lightweight_mode(app: &tauri::AppHandle) -> Result<(), String> {
    use tauri::WebviewWindowBuilder;

    if show_main_window(app) {
        LIGHTWEIGHT_MODE.store(false, Ordering::Release);
        crate::tray::refresh_tray_menu(app);
        log::info!("退出轻量模式");
        return Ok(());
    }

    if MAIN_WINDOW_REBUILDING
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        log::info!("主窗口正在重建，跳过重复恢复请求");
        return Ok(());
    }
    let _rebuild_guard = MainWindowRebuildGuard;

    if show_main_window(app) {
        LIGHTWEIGHT_MODE.store(false, Ordering::Release);
        crate::tray::refresh_tray_menu(app);
        log::info!("退出轻量模式");
        return Ok(());
    }

    let window_config = app
        .config()
        .app
        .windows
        .iter()
        .find(|w| w.label == "main")
        .ok_or("主窗口配置未找到")?;

    WebviewWindowBuilder::from_config(app, window_config)
        .map_err(|e| format!("加载主窗口配置失败: {e}"))?
        .build()
        .map_err(|e| format!("创建主窗口失败: {e}"))?;

    let _ = show_main_window(app);

    LIGHTWEIGHT_MODE.store(false, Ordering::Release);
    crate::tray::refresh_tray_menu(app);
    log::info!("退出轻量模式");
    Ok(())
}

pub fn is_lightweight_mode() -> bool {
    LIGHTWEIGHT_MODE.load(Ordering::Acquire)
}
