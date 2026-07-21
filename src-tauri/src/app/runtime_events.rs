//! Tauri runtime event dispatch.

#[cfg(target_os = "macos")]
use super::handle_deeplink_url;
use super::shutdown::{
    classify_exit_request, cleanup_before_exit, remove_tray_icon_before_exit,
    save_window_state_before_exit, ExitRequestAction,
};
#[cfg(target_os = "linux")]
use crate::linux_fix;
#[cfg(target_os = "macos")]
use crate::{lightweight, tray};
#[cfg(target_os = "macos")]
use tauri::Manager;
use tauri::RunEvent;
#[cfg(target_os = "macos")]
use tauri_plugin_deep_link::DeepLinkExt;

pub(super) fn handle(app_handle: &tauri::AppHandle, event: RunEvent) {
    // 处理退出请求（所有平台）
    if let RunEvent::ExitRequested { api, code, .. } = &event {
        match classify_exit_request(*code) {
            // code 为 None 表示运行时自动触发（如隐藏窗口的 WebView 被回收导致无存活窗口），
            // 此时应仅阻止退出、保持托盘后台运行。
            ExitRequestAction::StayInTray => {
                log::info!("运行时触发退出请求（无存活窗口），阻止退出以保持托盘后台运行");
                api.prevent_exit();
                return;
            }
            // code 为 RESTART_EXIT_CODE：app.restart() / 自更新 relaunch 发起的重启。
            // 这条路径上 prevent_exit() 会被 Tauri 忽略，事件循环必定退出，随后由
            // Tauri 在 RunEvent::Exit 后用新二进制 re-exec（macOS 会按更新后的
            // Info.plist 解析可执行名）。
            //
            // 绝不能复用下面的异步清理任务：该任务在 tokio 线程调 save_window_state，
            // 持有 window-state 插件锁的同时向主线程查询窗口几何；而主线程此刻正在
            // 退出事件循环，并在插件自带的 RunEvent::Exit 钩子里等待同一把锁——双方
            // 互等造成进程永久卡死（更新已安装但应用冻结、不再重启，见 #3998）。
            //
            // 重启路径交还 Tauri 默认流程即可：
            //   - 窗口状态：插件 Exit 钩子在主线程保存（同线程读取窗口几何，无死锁）
            //   - 托盘图标：Tauri 内部 cleanup_before_exit 清理，正常走 Drop
            //   - 代理/Live 配置：无需恢复，重启后新实例立即接管并恢复代理状态
            //   - 100ms 落盘等待：重启前的 DB 写入均为命令驱动、此刻已完成，
            //     与所有 Tauri 应用默认重启路径的行为一致，无需额外等待
            ExitRequestAction::DeferToTauriRestart => {
                log::info!("收到重启请求 (code={code:?})，交由 Tauri 默认重启流程 re-exec");
                return;
            }
            // 其它 Some(_)：用户主动调用 app.exit() 退出（如托盘菜单"退出"），
            // 此时执行清理后退出。
            ExitRequestAction::CleanupAndExit => {}
        }

        log::info!("收到用户主动退出请求 (code={code:?})，开始清理...");
        api.prevent_exit();

        let app_handle = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            save_window_state_before_exit(&app_handle);
            cleanup_before_exit(&app_handle).await;
            // 先于 std::process::exit 显式移除托盘图标。
            // 进程直接退出时 Tauri 运行时不走正常 Drop 流程，
            // 不会向 Windows Shell 发送 NIM_DELETE，导致已退出的进程
            // 注册的图标仍残留在系统托盘（鼠标悬停 Shell 才会重绘发现进程已死）。
            remove_tray_icon_before_exit(&app_handle);
            log::info!("清理完成，退出应用");

            // 短暂等待确保所有 I/O 操作（如数据库写入）刷新到磁盘
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;

            // 使用 std::process::exit 避免再次触发 ExitRequested
            std::process::exit(0);
        });
        return;
    }

    #[cfg(target_os = "macos")]
    {
        match event {
            // macOS 在 Dock 图标被点击并重新激活应用时会触发 Reopen 事件，这里手动恢复主窗口
            RunEvent::Reopen { .. } => {
                if let Some(window) = app_handle.get_webview_window("main") {
                    #[cfg(target_os = "windows")]
                    {
                        let _ = window.set_skip_taskbar(false);
                    }
                    let _ = window.unminimize();
                    let _ = window.show();
                    let _ = window.set_focus();
                    tray::apply_tray_policy(app_handle, true);
                } else if crate::lightweight::is_lightweight_mode() {
                    if let Err(e) = crate::lightweight::exit_lightweight_mode(app_handle) {
                        log::error!("退出轻量模式重建窗口失败: {e}");
                    }
                }
            }
            // 处理通过自定义 URL 协议触发的打开事件（例如 OpenSunstar://...）
            RunEvent::Opened { urls } => {
                if let Some(url) = urls.first() {
                    let url_str = url.to_string();
                    log::info!("RunEvent::Opened with URL: {url_str}");

                    if url_str.starts_with("OpenSunstar://") {
                        if crate::lightweight::is_lightweight_mode() {
                            if let Err(e) = crate::lightweight::exit_lightweight_mode(app_handle) {
                                log::error!("退出轻量模式重建窗口失败: {e}");
                            }
                        }

                        // 解析并广播深链接事件，复用与 single_instance 相同的逻辑
                        match crate::deeplink::parse_deeplink_url(&url_str) {
                            Ok(request) => {
                                log::info!(
                                        "Successfully parsed deep link from RunEvent::Opened: resource={}, app={:?}",
                                        request.resource,
                                        request.app
                                    );

                                if let Err(e) = app_handle.emit("deeplink-import", &request) {
                                    log::error!(
                                        "Failed to emit deep link event from RunEvent::Opened: {e}"
                                    );
                                }
                            }
                            Err(e) => {
                                log::error!(
                                    "Failed to parse deep link URL from RunEvent::Opened: {e}"
                                );

                                if let Err(emit_err) = app_handle.emit(
                                    "deeplink-error",
                                    serde_json::json!({
                                        "url": url_str,
                                        "error": e.to_string()
                                    }),
                                ) {
                                    log::error!(
                                            "Failed to emit deep link error event from RunEvent::Opened: {emit_err}"
                                        );
                                }
                            }
                        }

                        // 确保主窗口可见
                        if let Some(window) = app_handle.get_webview_window("main") {
                            let _ = window.unminimize();
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                }
            }
            _ => {}
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app_handle, event);
    }
}
