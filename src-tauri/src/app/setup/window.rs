//! Final platform webview adjustments and initial window visibility.

#[cfg(target_os = "linux")]
use crate::linux_fix;
#[cfg(target_os = "macos")]
use crate::tray;
use tauri::Manager;

pub(super) fn finalize(app: &mut tauri::App) {
    // Linux: 禁用 WebKitGTK 硬件加速，防止 EGL 初始化失败导致白屏
    #[cfg(target_os = "linux")]
    {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.with_webview(|webview| {
                use webkit2gtk::{HardwareAccelerationPolicy, SettingsExt, WebViewExt};
                let wk_webview = webview.inner();
                if let Some(settings) = WebViewExt::settings(&wk_webview) {
                    SettingsExt::set_hardware_acceleration_policy(
                        &settings,
                        HardwareAccelerationPolicy::Never,
                    );
                    log::info!("已禁用 WebKitGTK 硬件加速");
                }
            });
        }
    }

    // 静默启动：根据设置决定是否显示主窗口
    let settings = crate::settings::get_settings();
    if let Some(window) = app.get_webview_window("main") {
        // 在窗口首次显示前同步装饰状态，避免前端加载后再切换导致标题栏闪烁
        // 仅 Linux 生效：解决 Wayland 下系统窗口按钮不可用的问题
        #[cfg(target_os = "linux")]
        let _ = window.set_decorations(!settings.use_app_window_controls);
        if settings.silent_startup {
            // 静默启动模式：保持窗口隐藏
            let _ = window.hide();
            #[cfg(target_os = "windows")]
            let _ = window.set_skip_taskbar(true);
            #[cfg(target_os = "macos")]
            tray::apply_tray_policy(app.handle(), false);
            log::info!("静默启动模式：主窗口已隐藏");
        } else {
            // 正常启动模式：显示窗口
            let _ = window.show();
            log::info!("正常启动模式：主窗口已显示");

            // Linux: 解决首次启动 UI 无响应问题（Tauri #10746 + wry #637）。
            // 启动时 webview 未获取焦点 + surface 尺寸协商失败，导致点击无效。
            // 这里做 set_focus + 伪 resize，等价于无视觉版本的"最大化-还原"。
            #[cfg(target_os = "linux")]
            {
                linux_fix::nudge_main_window(window.clone());
            }
        }
    }
}
