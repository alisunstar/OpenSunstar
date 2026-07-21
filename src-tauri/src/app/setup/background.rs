//! Proxy recovery and long-running maintenance jobs.

use super::super::startup_recovery;
use crate::store::AppState;
use tauri::Manager;

pub(super) fn recover_and_start(app: &mut tauri::App) {
    // 代理恢复必须在主窗口显示前完成，避免 Claude Code 在端口未监听时连接 127.0.0.1
    {
        let state = app.state::<AppState>();
        tauri::async_runtime::block_on(startup_recovery::run_startup_proxy_recovery(state.inner()));
    }

    // 后台周期任务（备份、会话同步等）
    let app_handle = app.handle().clone();
    tauri::async_runtime::spawn(async move {
        let state = app_handle.state::<AppState>();

        // Periodic backup check (on startup)
        if let Err(e) = state.db.periodic_backup_if_needed() {
            log::warn!("Periodic backup failed on startup: {e}");
        }

        // Periodic maintenance timer: run once per day while the app is running
        let db_for_timer = state.db.clone();
        tauri::async_runtime::spawn(async move {
            const PERIODIC_MAINTENANCE_INTERVAL_SECS: u64 = 24 * 60 * 60;
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(
                PERIODIC_MAINTENANCE_INTERVAL_SECS,
            ));
            interval.tick().await; // skip immediate first tick (already checked above)
            loop {
                interval.tick().await;
                if let Err(e) = db_for_timer.periodic_backup_if_needed() {
                    log::warn!("Periodic maintenance timer failed: {e}");
                }
            }
        });

        // Session log usage sync: 启动时同步一次，之后每 60 秒检查
        let db_for_session_sync = state.db.clone();
        tauri::async_runtime::spawn(async move {
            const SESSION_SYNC_INTERVAL_SECS: u64 = 60;

            fn run_step<T>(name: &str, result: Result<T, crate::error::AppError>) {
                if let Err(e) = result {
                    log::warn!("{name} failed: {e}");
                }
            }

            let db = &db_for_session_sync;

            // 首次同步
            run_step(
                "Usage cost startup backfill",
                db.backfill_missing_usage_costs(),
            );
            run_step(
                "Session usage initial sync",
                crate::services::session_usage::sync_claude_session_logs(db),
            );
            run_step(
                "Codex usage initial sync",
                crate::services::session_usage_codex::sync_codex_usage(db),
            );
            run_step(
                "Gemini usage initial sync",
                crate::services::session_usage_gemini::sync_gemini_usage(db),
            );
            run_step(
                "OpenCode usage initial sync",
                crate::services::session_usage_opencode::sync_opencode_usage(db),
            );

            // 定期同步
            let mut interval =
                tokio::time::interval(std::time::Duration::from_secs(SESSION_SYNC_INTERVAL_SECS));
            interval.tick().await; // skip immediate first tick
            loop {
                interval.tick().await;
                run_step(
                    "Session usage periodic sync",
                    crate::services::session_usage::sync_claude_session_logs(db),
                );
                run_step(
                    "Codex usage periodic sync",
                    crate::services::session_usage_codex::sync_codex_usage(db),
                );
                run_step(
                    "Gemini usage periodic sync",
                    crate::services::session_usage_gemini::sync_gemini_usage(db),
                );
                run_step(
                    "OpenCode usage periodic sync",
                    crate::services::session_usage_opencode::sync_opencode_usage(db),
                );
            }
        });
    });
}
